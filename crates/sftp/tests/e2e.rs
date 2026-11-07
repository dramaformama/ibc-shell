//! End-to-end tests against a real OpenSSH `sftp-server` process spoken to over its stdio
//! (same SFTP protocol the SSH subsystem carries). Skipped when no `sftp-server` binary exists.
#![allow(
    clippy::many_single_char_names,
    clippy::similar_names,
    clippy::cast_possible_truncation,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    missing_docs
)]

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use ibc_shell_sftp::{
    ConflictChoice, ConflictPolicy, Endpoint, EntryKind, Sftp, TransferConfig, TransferEngine,
    TransferEvent, TransferJob,
};
use sha2::{Digest, Sha256};
use tokio::sync::mpsc::UnboundedReceiver;

fn server_bin() -> Option<PathBuf> {
    [
        "/usr/libexec/sftp-server",
        "/usr/lib/openssh/sftp-server",
        "/usr/libexec/openssh/sftp-server",
        "/usr/lib/ssh/sftp-server",
    ]
    .iter()
    .map(PathBuf::from)
    .find(|p| p.exists())
}

macro_rules! sftp_or_skip {
    () => {
        match connect().await {
            Some(s) => s,
            None => {
                eprintln!("sftp-server not found, skipping");
                return;
            }
        }
    };
}

async fn connect() -> Option<Arc<Sftp>> {
    let bin = server_bin()?;
    let mut child = tokio::process::Command::new(bin)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .ok()?;
    let io = tokio::io::join(child.stdout.take()?, child.stdin.take()?);
    tokio::spawn(async move {
        let _ = child.wait().await;
    });
    Some(Arc::new(Sftp::from_stream(io).await.expect("sftp init")))
}

fn rnd(n: usize, seed: u64) -> Vec<u8> {
    let mut x = seed | 1;
    (0..n)
        .map(|_| {
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            (x >> 24) as u8
        })
        .collect()
}

fn sha(p: &Path) -> Vec<u8> {
    Sha256::digest(std::fs::read(p).unwrap()).to_vec()
}

fn job(
    src: Endpoint,
    dst: Endpoint,
    items: &[&Path],
    dst_dir: &Path,
    c: ConflictPolicy,
) -> TransferJob {
    TransferJob {
        src,
        dst,
        items: items
            .iter()
            .map(|p| p.to_string_lossy().into_owned())
            .collect(),
        dst_dir: dst_dir.to_string_lossy().into_owned(),
        conflict: c,
    }
}

/// Collects events until `Done`/`Failed`/`Cancelled` for any id; returns all events.
async fn run_to_end(rx: &mut UnboundedReceiver<TransferEvent>, n: usize) -> Vec<TransferEvent> {
    let mut out = Vec::new();
    let mut ends = 0;
    while ends < n {
        let ev = tokio::time::timeout(Duration::from_secs(120), rx.recv())
            .await
            .expect("timeout")
            .expect("closed");
        if matches!(
            ev,
            TransferEvent::Done { .. }
                | TransferEvent::Failed { .. }
                | TransferEvent::Cancelled { .. }
        ) {
            ends += 1;
        }
        out.push(ev);
    }
    out
}

fn assert_done(evs: &[TransferEvent]) {
    assert!(
        evs.iter().any(|e| matches!(e, TransferEvent::Done { .. })),
        "not done: {:?}",
        evs.last()
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn file_ops() {
    let s = sftp_or_skip!();
    let t = tempfile::tempdir().unwrap();
    let d = t.path().canonicalize().unwrap();
    let p = |x: &str| d.join(x).to_string_lossy().into_owned();
    s.mkdir_p(&p("a/b/c")).await.unwrap();
    s.mkdir_p(&p("a/b/c")).await.unwrap();
    std::fs::write(d.join("a/b/f.txt"), b"hi").unwrap();
    let l = s.list(&p("a/b")).await.unwrap();
    assert_eq!(l.entries.len(), 2);
    let st = s.stat(&p("a/b/f.txt")).await.unwrap();
    assert_eq!((st.kind, st.size), (EntryKind::File, 2));
    s.rename(&p("a/b/f.txt"), &p("a/b/g.txt")).await.unwrap();
    std::fs::write(d.join("a/b/h.txt"), b"x").unwrap();
    s.rename(&p("a/b/h.txt"), &p("a/b/g.txt"))
        .await
        .unwrap_or(());
    s.chmod(&p("a/b/g.txt"), 0o600).await.unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let m = std::fs::metadata(d.join("a/b/g.txt"))
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(m & 0o777, 0o600);
    }
    s.symlink(&p("a/ln"), "b/g.txt").await.unwrap();
    assert_eq!(s.readlink(&p("a/ln")).await.unwrap(), "b/g.txt");
    assert!(s.realpath(".").await.unwrap().starts_with('/'));
    assert!(s.stat(&p("nope")).await.is_err());
    s.remove(&p("a"), true).await.unwrap();
    assert!(!d.join("a").exists());
}

#[tokio::test(flavor = "multi_thread")]
async fn big_file_both_ways_sha_matches() {
    let s = sftp_or_skip!();
    let mib: usize = std::env::var("IBC_BIG_MIB")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(96);
    let t = tempfile::tempdir().unwrap();
    let d = t.path().canonicalize().unwrap();
    std::fs::create_dir_all(d.join("up")).unwrap();
    std::fs::create_dir_all(d.join("down")).unwrap();
    let src = d.join("big.bin");
    std::fs::write(&src, rnd(mib << 20, 7)).unwrap();
    let mtime = 1_600_000_000_000_i64;
    let f = std::fs::OpenOptions::new().write(true).open(&src).unwrap();
    f.set_modified(std::time::UNIX_EPOCH + Duration::from_millis(mtime.unsigned_abs()))
        .unwrap();
    drop(f);
    let eng = TransferEngine::new(TransferConfig::default());
    let mut rx = eng.subscribe();
    // upload (local -> "remote"), then download the result back
    let started = std::time::Instant::now();
    eng.enqueue(job(
        Endpoint::Local,
        Endpoint::Remote(s.clone()),
        &[&src],
        &d.join("up"),
        ConflictPolicy::Overwrite,
    ))
    .unwrap();
    let evs = run_to_end(&mut rx, 1).await;
    assert_done(&evs);
    let up = started.elapsed();
    assert_eq!(sha(&src), sha(&d.join("up/big.bin")));
    let meta = std::fs::metadata(d.join("up/big.bin")).unwrap();
    assert_eq!(
        meta.modified().unwrap(),
        std::fs::metadata(&src).unwrap().modified().unwrap()
    );
    assert!(!d.join("up/big.bin.tlpart").exists());
    let started = std::time::Instant::now();
    eng.enqueue(job(
        Endpoint::Remote(s.clone()),
        Endpoint::Local,
        &[&d.join("up/big.bin")],
        &d.join("down"),
        ConflictPolicy::Overwrite,
    ))
    .unwrap();
    assert_done(&run_to_end(&mut rx, 1).await);
    let down = started.elapsed();
    assert_eq!(sha(&src), sha(&d.join("down/big.bin")));
    #[allow(clippy::cast_precision_loss)]
    let mbs = |e: Duration| mib as f64 / e.as_secs_f64();
    eprintln!(
        "BENCH {mib} MiB: upload {:.0} MiB/s, download {:.0} MiB/s",
        mbs(up),
        mbs(down)
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn pause_resume_and_cancel_keep_partial() {
    let s = sftp_or_skip!();
    let t = tempfile::tempdir().unwrap();
    let d = t.path().canonicalize().unwrap();
    std::fs::create_dir_all(d.join("out")).unwrap();
    let src = d.join("p.bin");
    std::fs::write(&src, rnd(400 << 20, 9)).unwrap();
    let eng = TransferEngine::new(TransferConfig {
        pipeline_depth: 2,
        ..TransferConfig::default()
    });
    let mut rx = eng.subscribe();
    let ids = eng
        .enqueue(job(
            Endpoint::Remote(s.clone()),
            Endpoint::Local,
            &[&src],
            &d.join("out"),
            ConflictPolicy::Overwrite,
        ))
        .unwrap();
    let id = ids[0];
    loop {
        if matches!(rx.recv().await.unwrap(), TransferEvent::Started { .. }) {
            break;
        }
    }
    eng.pause(id).unwrap();
    loop {
        if matches!(rx.recv().await.unwrap(), TransferEvent::Paused { .. }) {
            break;
        }
    }
    assert!(!d.join("out/p.bin").exists());
    eng.resume(id).unwrap();
    assert_done(&run_to_end(&mut rx, 1).await);
    assert_eq!(sha(&src), sha(&d.join("out/p.bin")));

    // cancel keeps partial; retry finishes
    std::fs::remove_file(d.join("out/p.bin")).unwrap();
    let id = eng
        .enqueue(job(
            Endpoint::Remote(s.clone()),
            Endpoint::Local,
            &[&src],
            &d.join("out"),
            ConflictPolicy::Overwrite,
        ))
        .unwrap()[0];
    loop {
        if matches!(rx.recv().await.unwrap(), TransferEvent::Started { .. }) {
            break;
        }
    }
    eng.cancel(id).unwrap();
    loop {
        if matches!(rx.recv().await.unwrap(), TransferEvent::Cancelled { .. }) {
            break;
        }
    }
    eng.retry(id).unwrap();
    assert_done(&run_to_end(&mut rx, 1).await);
    assert_eq!(sha(&src), sha(&d.join("out/p.bin")));
}

#[tokio::test(flavor = "multi_thread")]
async fn resume_from_stale_partial_and_reject_corrupt_tail() {
    let s = sftp_or_skip!();
    let t = tempfile::tempdir().unwrap();
    let d = t.path().canonicalize().unwrap();
    std::fs::create_dir_all(d.join("out")).unwrap();
    let data = rnd(5 << 20, 3);
    let src = d.join("r.bin");
    std::fs::write(&src, &data).unwrap();
    let eng = TransferEngine::new(TransferConfig::default());
    let mut rx = eng.subscribe();
    // good prefix -> resumed
    std::fs::write(d.join("out/r.bin.tlpart"), &data[..2 << 20]).unwrap();
    eng.enqueue(job(
        Endpoint::Remote(s.clone()),
        Endpoint::Local,
        &[&src],
        &d.join("out"),
        ConflictPolicy::Overwrite,
    ))
    .unwrap();
    assert_done(&run_to_end(&mut rx, 1).await);
    assert_eq!(sha(&src), sha(&d.join("out/r.bin")));
    // prefix with corrupt tail -> restarted from zero, still correct
    std::fs::remove_file(d.join("out/r.bin")).unwrap();
    let mut bad = data[..2 << 20].to_vec();
    let n = bad.len();
    bad[n - 10] ^= 0xff;
    std::fs::write(d.join("out/r.bin.tlpart"), &bad).unwrap();
    eng.enqueue(job(
        Endpoint::Remote(s.clone()),
        Endpoint::Local,
        &[&src],
        &d.join("out"),
        ConflictPolicy::Overwrite,
    ))
    .unwrap();
    assert_done(&run_to_end(&mut rx, 1).await);
    assert_eq!(sha(&src), sha(&d.join("out/r.bin")));
}

#[tokio::test(flavor = "multi_thread")]
async fn ten_thousand_small_files() {
    let s = sftp_or_skip!();
    let t = tempfile::tempdir().unwrap();
    let d = t.path().canonicalize().unwrap();
    let tree = d.join("tree");
    for i in 0..100 {
        let sub = tree.join(format!("d{i}"));
        std::fs::create_dir_all(&sub).unwrap();
        for j in 0..100 {
            std::fs::write(sub.join(format!("f{j}.txt")), format!("{i}-{j}")).unwrap();
        }
    }
    std::fs::create_dir_all(d.join("out")).unwrap();
    let eng = TransferEngine::new(TransferConfig::default());
    let mut rx = eng.subscribe();
    eng.enqueue(job(
        Endpoint::Remote(s.clone()),
        Endpoint::Local,
        &[&tree],
        &d.join("out"),
        ConflictPolicy::Overwrite,
    ))
    .unwrap();
    let evs = run_to_end(&mut rx, 1).await;
    assert_done(&evs);
    let files = evs
        .iter()
        .filter(|e| matches!(e, TransferEvent::FileDone { skipped: false, .. }))
        .count();
    assert_eq!(files, 10_000);
    assert_eq!(
        std::fs::read_to_string(d.join("out/tree/d57/f99.txt")).unwrap(),
        "57-99"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn hostile_and_unicode_names_are_made_safe() {
    let s = sftp_or_skip!();
    let t = tempfile::tempdir().unwrap();
    let d = t.path().canonicalize().unwrap();
    let src = d.join("src");
    std::fs::create_dir_all(&src).unwrap();
    for n in [
        "файл ✓.txt",
        "CON",
        "a:b?.txt",
        "trailing. ",
        "e\u{301}.txt",
        &"x".repeat(250),
        "..hidden",
    ] {
        std::fs::write(src.join(n), n).unwrap();
    }
    std::fs::create_dir_all(d.join("out")).unwrap();
    let eng = TransferEngine::new(TransferConfig::default());
    let mut rx = eng.subscribe();
    eng.enqueue(job(
        Endpoint::Remote(s.clone()),
        Endpoint::Local,
        &[&src],
        &d.join("out"),
        ConflictPolicy::Overwrite,
    ))
    .unwrap();
    assert_done(&run_to_end(&mut rx, 1).await);
    let root = d.join("out").canonicalize().unwrap();
    let mut n = 0;
    for e in std::fs::read_dir(root.join("src")).unwrap() {
        let e = e.unwrap();
        assert!(e.path().canonicalize().unwrap().starts_with(&root));
        assert!(e.file_name().len() <= 255);
        n += 1;
    }
    assert_eq!(n, 7, "every file arrives (renamed when unsafe)");
    assert!(root.join("src/файл ✓.txt").exists());
    assert!(
        std::fs::read_dir(d.join("out")).unwrap().count() == 1,
        "nothing escaped"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn symlinks_are_not_followed_in_recursive_download() {
    let s = sftp_or_skip!();
    let t = tempfile::tempdir().unwrap();
    let d = t.path().canonicalize().unwrap();
    let src = d.join("src");
    std::fs::create_dir_all(&src).unwrap();
    std::fs::write(d.join("secret"), "s").unwrap();
    std::fs::write(src.join("ok"), "ok").unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink(d.join("secret"), src.join("link")).unwrap();
    std::fs::create_dir_all(d.join("out")).unwrap();
    let eng = TransferEngine::new(TransferConfig::default());
    let mut rx = eng.subscribe();
    eng.enqueue(job(
        Endpoint::Remote(s.clone()),
        Endpoint::Local,
        &[&src],
        &d.join("out"),
        ConflictPolicy::Overwrite,
    ))
    .unwrap();
    let evs = run_to_end(&mut rx, 1).await;
    assert_done(&evs);
    assert!(d.join("out/src/ok").exists());
    #[cfg(unix)]
    {
        assert!(!d.join("out/src/link").exists());
        assert!(evs
            .iter()
            .any(|e| matches!(e, TransferEvent::FileDone { skipped: true, .. })));
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn conflict_policies() {
    let s = sftp_or_skip!();
    let t = tempfile::tempdir().unwrap();
    let d = t.path().canonicalize().unwrap();
    let src = d.join("src");
    std::fs::create_dir_all(&src).unwrap();
    let out = d.join("out");
    std::fs::create_dir_all(&out).unwrap();
    for n in ["a.txt", "b.txt"] {
        std::fs::write(src.join(n), "NEWDATA").unwrap();
        std::fs::write(out.join(n), "old").unwrap();
    }
    let eng = TransferEngine::new(TransferConfig::default());
    let mut rx = eng.subscribe();
    let go = |c| {
        eng.enqueue(job(
            Endpoint::Remote(s.clone()),
            Endpoint::Local,
            &[&src.join("a.txt"), &src.join("b.txt")],
            &out,
            c,
        ))
        .unwrap();
    };
    go(ConflictPolicy::Skip);
    run_to_end(&mut rx, 2).await;
    assert_eq!(std::fs::read_to_string(out.join("a.txt")).unwrap(), "old");
    go(ConflictPolicy::Rename);
    run_to_end(&mut rx, 2).await;
    assert_eq!(
        std::fs::read_to_string(out.join("a (1).txt")).unwrap(),
        "NEWDATA"
    );
    assert_eq!(std::fs::read_to_string(out.join("a.txt")).unwrap(), "old");
    // ResumeIfSmaller: "old" is smaller than source but not a prefix -> restarts, ends correct
    go(ConflictPolicy::ResumeIfSmaller);
    run_to_end(&mut rx, 2).await;
    assert_eq!(
        std::fs::read_to_string(out.join("a.txt")).unwrap(),
        "NEWDATA"
    );
    // Ask with apply-to-all: a single prompt replaces both
    std::fs::write(out.join("a.txt"), "old").unwrap();
    std::fs::write(out.join("b.txt"), "old").unwrap();
    let ids = eng
        .enqueue(job(
            Endpoint::Remote(s.clone()),
            Endpoint::Local,
            &[&src],
            &out,
            ConflictPolicy::Ask,
        ))
        .unwrap();
    assert_eq!(ids.len(), 1);
    std::fs::create_dir_all(out.join("src")).unwrap();
    std::fs::write(out.join("src/a.txt"), "old").unwrap();
    std::fs::write(out.join("src/b.txt"), "old").unwrap();
    let mut prompts = 0;
    loop {
        match rx.recv().await.unwrap() {
            TransferEvent::Conflict {
                pending_id, info, ..
            } => {
                prompts += 1;
                assert_eq!(info.dst_size, 3);
                eng.conflict_reply(pending_id, ConflictChoice::Overwrite, true)
                    .unwrap();
            }
            TransferEvent::Done { .. } => break,
            TransferEvent::Failed { message, .. } => panic!("{message}"),
            _ => {}
        }
    }
    assert_eq!(prompts, 1);
    assert_eq!(
        std::fs::read_to_string(out.join("src/b.txt")).unwrap(),
        "NEWDATA"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn remote_to_remote_streams_through_client() {
    let a = sftp_or_skip!();
    let b = connect().await.unwrap();
    let t = tempfile::tempdir().unwrap();
    let d = t.path().canonicalize().unwrap();
    let src = d.join("r2r.bin");
    std::fs::write(&src, rnd(20 << 20, 5)).unwrap();
    std::fs::create_dir_all(d.join("out")).unwrap();
    let eng = TransferEngine::new(TransferConfig::default());
    let mut rx = eng.subscribe();
    eng.enqueue(job(
        Endpoint::Remote(a),
        Endpoint::Remote(b),
        &[&src],
        &d.join("out"),
        ConflictPolicy::Overwrite,
    ))
    .unwrap();
    assert_done(&run_to_end(&mut rx, 1).await);
    assert_eq!(sha(&src), sha(&d.join("out/r2r.bin")));
}
