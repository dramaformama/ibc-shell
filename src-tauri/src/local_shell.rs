//! The user's own shell on this computer (ADR-017): a login shell in a local PTY, pumped through
//! the same channels, coalescing and credit window as SSH sessions (ADR-004).
//!
//! `portable-pty` is blocking, so a reader thread feeds a bounded channel and a writer thread
//! drains keystrokes; the async pump only decides when to read (credit) and when to flush.

use std::io::{Read, Write};
use std::sync::{Arc, PoisonError};
use std::time::Duration;

use portable_pty::{native_pty_system, Child, CommandBuilder, MasterPty, PtySize};
use tokio::sync::{mpsc, Notify};
use tokio::time::Instant;

use crate::api::dto::SessionEvent;
use crate::api::stream::RawBytes;
use crate::api::ApiError;
use crate::session_task::{FLUSH_AFTER, FLUSH_BYTES};
use crate::state::{SessionHandle, ShellCmd, WriteBytes};

/// Stop reading the PTY while this many bytes are unacknowledged by the UI.
const CREDIT_WINDOW: u64 = 1 << 20;
/// One read from the PTY.
const READ_CHUNK: usize = 16 * 1024;

fn size(cols: u16, rows: u16) -> PtySize {
    PtySize {
        rows: rows.max(1),
        cols: cols.max(1),
        pixel_width: 0,
        pixel_height: 0,
    }
}

/// Login shell in the home folder, with a UTF-8 locale when the app was started without one
/// (apps launched from Finder / the Dock get no `LANG`).
fn command() -> CommandBuilder {
    let mut cmd = CommandBuilder::new_default_prog();
    if let Some(home) = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")) {
        cmd.cwd(home);
    }
    cmd.env("TERM", "xterm-256color");
    cmd.env("COLORTERM", "truecolor");
    cmd.env("TERM_PROGRAM", "iBC-Shell");
    if std::env::var_os("LANG").is_none() && std::env::var_os("LC_ALL").is_none() {
        cmd.env("LANG", "en_US.UTF-8");
    }
    cmd
}

struct Pty {
    master: Box<dyn MasterPty + Send>,
    child: Box<dyn Child + Send + Sync>,
    out: mpsc::Receiver<Vec<u8>>,
    /// Keystrokes stay counted against the input budget until the writer thread wrote them.
    input: std::sync::mpsc::Sender<WriteBytes>,
}

fn spawn_pty(cols: u16, rows: u16) -> Result<Pty, String> {
    let pair = native_pty_system()
        .openpty(size(cols, rows))
        .map_err(|e| e.to_string())?;
    let child = pair
        .slave
        .spawn_command(command())
        .map_err(|e| e.to_string())?;
    drop(pair.slave);
    let mut reader = pair.master.try_clone_reader().map_err(|e| e.to_string())?;
    let mut writer = pair.master.take_writer().map_err(|e| e.to_string())?;

    let (tx, out) = mpsc::channel::<Vec<u8>>(8);
    std::thread::Builder::new()
        .name("local-pty-read".into())
        .spawn(move || {
            let mut buf = vec![0u8; READ_CHUNK];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => {
                        if tx.blocking_send(buf[..n].to_vec()).is_err() {
                            break;
                        }
                    }
                }
            }
        })
        .map_err(|e| e.to_string())?;

    let (input, keys) = std::sync::mpsc::channel::<WriteBytes>();
    std::thread::Builder::new()
        .name("local-pty-write".into())
        .spawn(move || {
            while let Ok(bytes) = keys.recv() {
                if writer
                    .write_all(bytes.as_slice())
                    .and_then(|()| writer.flush())
                    .is_err()
                {
                    break;
                }
            }
        })
        .map_err(|e| e.to_string())?;

    Ok(Pty {
        master: pair.master,
        child,
        out,
        input,
    })
}

/// Session task for [`crate::state::SessionTarget::Local`]. Ends with `Disconnected`:
/// `closed` (user) or `shell_exited` (reconnect restarts the shell).
pub async fn run(
    handle: Arc<SessionHandle>,
    mut rx: mpsc::UnboundedReceiver<ShellCmd>,
    _closed: Arc<Notify>,
) {
    let events = handle.spec.on_event.clone();
    let (cols, rows) = handle
        .live
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .size;
    let mut pty = match spawn_pty(cols, rows) {
        Ok(p) => p,
        Err(detail) => {
            let _ = events.send(SessionEvent::Failed {
                error: ApiError::Internal(detail),
            });
            return;
        }
    };
    let _ = events.send(SessionEvent::Connected {
        server_banner: None,
    });

    let user_closed = pump(&mut pty, &mut rx, &handle).await;
    if user_closed {
        let _ = pty.child.kill();
    }
    // The reader has seen EOF (or we killed the shell): collect the exit code off the runtime.
    let mut child = pty.child;
    let code = tauri::async_runtime::spawn_blocking(move || child.wait().ok())
        .await
        .ok()
        .flatten()
        .map(|s| s.exit_code());
    if let (Some(code), false) = (code, user_closed) {
        let _ = events.send(SessionEvent::ExitStatus { code });
    }
    let _ = events.send(SessionEvent::Disconnected {
        reason: if user_closed {
            "closed"
        } else {
            "shell_exited"
        }
        .into(),
        can_reconnect: !user_closed,
    });
}

/// Returns `true` when the user closed the session, `false` when the shell ended.
async fn pump(
    pty: &mut Pty,
    rx: &mut mpsc::UnboundedReceiver<ShellCmd>,
    handle: &SessionHandle,
) -> bool {
    let mut buf: Vec<u8> = Vec::new();
    let mut deadline: Option<Instant> = None;
    let mut unacked: u64 = 0;
    let flush = |buf: &mut Vec<u8>, unacked: &mut u64| {
        if !buf.is_empty() {
            *unacked += buf.len() as u64;
            let _ = handle.spec.on_data.send(RawBytes(std::mem::take(buf)));
        }
    };
    let user_closed = loop {
        let timer = async {
            match deadline {
                Some(d) => tokio::time::sleep_until(d).await,
                None => std::future::pending::<()>().await,
            }
        };
        let can_read = unacked + (buf.len() as u64) < CREDIT_WINDOW;
        tokio::select! {
            out = pty.out.recv(), if can_read => match out {
                Some(d) => {
                    buf.extend_from_slice(&d);
                    if buf.len() >= FLUSH_BYTES {
                        flush(&mut buf, &mut unacked);
                        deadline = None;
                    } else if deadline.is_none() {
                        deadline = Some(Instant::now() + FLUSH_AFTER);
                    }
                }
                None => break false,
            },
            cmd = rx.recv() => match cmd {
                Some(ShellCmd::Write(b)) => {
                    if pty.input.send(b).is_err() {
                        break false;
                    }
                }
                Some(ShellCmd::Resize(c, r)) => {
                    handle.live.lock().unwrap_or_else(PoisonError::into_inner).size = (c, r);
                    let _ = pty.master.resize(size(c, r));
                }
                Some(ShellCmd::Ack(n)) => unacked = unacked.saturating_sub(u64::from(n)),
                Some(ShellCmd::Close) | None => break true,
            },
            () = timer => {
                flush(&mut buf, &mut unacked);
                deadline = None;
            }
        }
    };
    flush(&mut buf, &mut unacked);
    // Give a just-exited shell's last bytes a moment to land before the banner.
    if !user_closed {
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    user_closed
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::ShellTx;

    // POSIX shell syntax; Windows starts cmd.exe (and the app's tests do not run there in CI).
    #[cfg(unix)]
    #[tokio::test]
    async fn runs_a_command_in_the_login_shell() {
        let mut pty = spawn_pty(80, 24).unwrap();
        // Keystrokes take the same path as from the UI: ShellTx → WriteBytes → writer thread.
        let (tx, mut rx, _) = ShellTx::channel();
        assert!(tx.write(b"echo ibc-$((40+2))\nexit\n"));
        let Some(ShellCmd::Write(keys)) = rx.recv().await else {
            panic!("expected keystrokes");
        };
        pty.input.send(keys).unwrap();
        let mut out = Vec::new();
        let deadline = Instant::now() + Duration::from_secs(10);
        while let Ok(Some(d)) = tokio::time::timeout_at(deadline, pty.out.recv()).await {
            out.extend_from_slice(&d);
        }
        let text = String::from_utf8_lossy(&out);
        assert!(text.contains("ibc-42"), "output: {text}");
        assert!(pty.child.wait().unwrap().success());
    }
}
