//! Per-session task: connect, open the shell, pump output (coalesced 64 KiB / 8 ms, ADR-004)
//! to the UI and input / acks / resizes into the shell.

use std::sync::{Arc, PoisonError};
use std::time::Duration;

use ibc_shell_ssh::{ConnectStep, PtyRequest, Shell, ShellOutput};
use tokio::sync::{mpsc, Notify};
use tokio::time::Instant;

use crate::api::dto::SessionEvent;
use crate::api::stream::RawBytes;
use crate::api::ApiError;
use crate::connect::{build_chain, StoreKnownHosts};
use crate::prompts::EventPrompter;
use crate::state::{AppState, SessionHandle, SessionTarget, ShellCmd, ShellTx};

/// Flush when this many bytes are buffered.
pub const FLUSH_BYTES: usize = 64 * 1024;
/// Flush when the oldest buffered byte is this old.
pub const FLUSH_AFTER: Duration = Duration::from_millis(8);

/// Installs a fresh command channel into `handle` and spawns the session task.
pub fn spawn_session(state: &AppState, handle: &Arc<SessionHandle>, cols: u16, rows: u16) {
    let (tx, rx, closed) = ShellTx::channel();
    {
        let mut live = handle.live.lock().unwrap_or_else(PoisonError::into_inner);
        live.cmd = Some(tx);
        live.conn = None;
        live.size = (cols, rows);
    }
    let (state, handle) = (state.clone(), handle.clone());
    if matches!(handle.spec.target, SessionTarget::Local) {
        #[cfg(desktop)]
        tauri::async_runtime::spawn(crate::local_shell::run(handle, rx, closed));
        #[cfg(not(desktop))]
        let _ = (
            rx,
            closed,
            handle.spec.on_event.send(SessionEvent::Failed {
                error: ApiError::Validation("local shell is desktop-only".into()),
            }),
        );
        return;
    }
    tauri::async_runtime::spawn(run_session(state, handle, rx, closed));
}

async fn wait_close(rx: &mut mpsc::UnboundedReceiver<ShellCmd>) {
    loop {
        match rx.recv().await {
            Some(ShellCmd::Close) | None => return,
            Some(_) => {}
        }
    }
}

async fn run_session(
    state: AppState,
    handle: Arc<SessionHandle>,
    mut rx: mpsc::UnboundedReceiver<ShellCmd>,
    closed: Arc<Notify>,
) {
    let events = handle.spec.on_event.clone();
    let fail = |error: ApiError| {
        let _ = events.send(SessionEvent::Failed { error });
    };
    let chain = match build_chain(&state.store, &state.vault, &handle.spec.target) {
        Ok(c) => c,
        Err(e) => return fail(e),
    };
    let prompter = EventPrompter {
        owner: handle.id.0,
        events: events.clone(),
        pending: state.pending.clone(),
    };
    let known = StoreKnownHosts(state.store.clone());
    let conn = tokio::select! {
        r = ibc_shell_ssh::connect(chain, &known, &prompter) => match r {
            Ok(c) => Arc::new(c),
            Err(e) => return fail(e.into()),
        },
        () = wait_close(&mut rx) => {
            let _ = events.send(SessionEvent::Disconnected {
                reason: "closed".into(),
                can_reconnect: false,
            });
            return;
        }
    };
    let (cols, rows) = handle
        .live
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .size;
    let label = conn.server_banner().map(str::to_owned);
    let _ = events.send(SessionEvent::Connecting {
        step: ConnectStep::OpeningChannel,
        hop: String::new(),
    });
    let shell = match conn
        .open_shell(PtyRequest {
            term: "xterm-256color".into(),
            cols,
            rows,
        })
        .await
    {
        Ok(s) => s,
        Err(e) => return fail(e.into()),
    };
    handle
        .live
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .conn = Some(conn.clone());
    if let SessionTarget::Host(id) = &handle.spec.target {
        let _ = state.store.host_touch_connected(*id);
        // Startup command (host or inherited from a folder), typed into the fresh shell.
        if let Some(cmd) = startup_command(&state, *id) {
            let _ = shell.write(cmd.as_bytes()).await;
        }
    }
    let _ = events.send(SessionEvent::Connected {
        server_banner: label,
    });
    {
        let (state, handle) = (state.clone(), handle.clone());
        tauri::async_runtime::spawn(async move {
            crate::forwards::auto_start(&state, &handle).await;
        });
    }

    let user_closed = pump(shell, &mut rx, &closed, &handle).await;
    let reason = if user_closed {
        "closed".to_owned()
    } else {
        end_reason(&conn).await
    };
    handle
        .live
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .conn = None;
    crate::forwards::stop_all(&handle);
    conn.disconnect().await;
    let _ = events.send(SessionEvent::Disconnected {
        reason,
        can_reconnect: !user_closed,
    });
}

/// The resolved startup command as one input line; `None` when unset or blank.
fn startup_command(state: &AppState, id: ibc_shell_store::HostId) -> Option<String> {
    let cmd = state.store.host_resolve(id).ok()?.startup_cmd.value?;
    let cmd = cmd.trim();
    // One line only: a pasted multi-line value must not run several commands.
    (!cmd.is_empty() && !cmd.contains(['\n', '\r'])).then(|| format!("{cmd}\n"))
}

/// Why the shell ended without the user closing it: the connection's own reason when the
/// transport reports one (it does so a moment after the channel closes, hence the short wait),
/// `shell_exited` when only the shell finished and the connection is still up.
async fn end_reason(conn: &ibc_shell_ssh::Connection) -> String {
    for _ in 0..6 {
        if let Some(r) = conn.close_reason() {
            return r;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    conn.close_reason().unwrap_or_else(|| {
        if conn.is_closed() {
            "connection closed".into()
        } else {
            "shell_exited".into()
        }
    })
}

/// Runs until the channel closes or `Close`; returns `true` when the user closed it.
async fn pump(
    mut shell: Shell,
    rx: &mut mpsc::UnboundedReceiver<ShellCmd>,
    closed: &Notify,
    handle: &SessionHandle,
) -> bool {
    let mut buf: Vec<u8> = Vec::new();
    let mut deadline: Option<Instant> = None;
    let flush = |buf: &mut Vec<u8>| {
        if !buf.is_empty() {
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
        tokio::select! {
            out = shell.read() => match out {
                Some(ShellOutput::Data(d)) => {
                    buf.extend_from_slice(&d);
                    if buf.len() >= FLUSH_BYTES {
                        flush(&mut buf);
                        deadline = None;
                    } else if deadline.is_none() {
                        deadline = Some(Instant::now() + FLUSH_AFTER);
                    }
                }
                Some(ShellOutput::ExitStatus(code)) => {
                    flush(&mut buf);
                    deadline = None;
                    let _ = handle.spec.on_event.send(SessionEvent::ExitStatus { code });
                }
                None => break false,
            },
            cmd = rx.recv() => match cmd {
                Some(ShellCmd::Write(b)) => {
                    // Raced against the close signal: a server that never opens its
                    // window must not make `session_close` wait behind this write.
                    tokio::select! {
                        r = shell.write(b.as_slice()) => {
                            if r.is_err() {
                                break false;
                            }
                        }
                        () = closed.notified() => break true,
                    }
                }
                Some(ShellCmd::Resize(c, r)) => {
                    handle.live.lock().unwrap_or_else(PoisonError::into_inner).size = (c, r);
                    let _ = shell.resize(c, r).await;
                }
                Some(ShellCmd::Ack(n)) => shell.ack(n),
                Some(ShellCmd::Close) | None => break true,
            },
            () = timer => {
                flush(&mut buf);
                deadline = None;
            }
        }
    };
    flush(&mut buf);
    shell.close().await;
    user_closed
}
