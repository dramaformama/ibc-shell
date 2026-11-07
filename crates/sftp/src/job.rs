//! One transfer (a top-level item): scan, conflicts, per-file copy with `.tlpart` + resume.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use futures_util::stream::{self, StreamExt};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::copy::{copy, read_at, Plan};
use crate::engine::{lk, Shared};
use crate::scan::{scan, FileTask};
use crate::side::Side;
use crate::{
    ConflictChoice, ConflictId, ConflictInfo, ConflictPolicy, EntryKind, SftpError, TransferEvent,
    TransferId,
};

const TAIL_VERIFY: u64 = 1 << 20;
const MAX_REQUEST: u32 = 256 * 1024;
const TLPART: &str = ".tlpart";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Status {
    Running,
    Paused,
    Cancelled,
    Failed,
    Done,
}

pub(crate) struct Ctl {
    pub status: Status,
    pub token: CancellationToken,
    /// A driver task is alive (it flips this to false under the lock when it exits).
    pub active: bool,
}

pub(crate) struct Job {
    pub id: TransferId,
    pub src: Side,
    pub dst: Side,
    pub item: String,
    pub dst_dir: String,
    pub policy: ConflictPolicy,
    pub ctl: Mutex<Ctl>,
    pub apply_all: Mutex<Option<ConflictChoice>>,
    pub decisions: Mutex<HashMap<String, ConflictChoice>>,
    pub done: Mutex<HashSet<String>>,
    /// Serialises conflict prompts so "apply to all" reaches later files.
    pub gate: tokio::sync::Mutex<()>,
    pub started: AtomicBool,
}

/// Runs the job until done/failed/paused/cancelled (re-runs after a quick resume).
pub(crate) async fn drive(sh: Arc<Shared>, job: Arc<Job>) {
    loop {
        let token = lk(&job.ctl).token.clone();
        let res = run_once(&sh, &job, &token).await;
        let mut c = lk(&job.ctl);
        let id = job.id;
        let ev = match res {
            Ok(()) => {
                c.status = Status::Done;
                TransferEvent::Done { id }
            }
            Err(e) => {
                let interrupted = matches!(e, SftpError::Cancelled) || token.is_cancelled();
                if interrupted {
                    match c.status {
                        Status::Running => continue, // resumed while winding down
                        Status::Paused => TransferEvent::Paused { id },
                        _ => {
                            c.status = Status::Cancelled;
                            TransferEvent::Cancelled { id }
                        }
                    }
                } else {
                    c.status = Status::Failed;
                    TransferEvent::Failed {
                        id,
                        message: e.to_string(),
                    }
                }
            }
        };
        c.active = false;
        drop(c);
        sh.emit(ev);
        return;
    }
}

fn check<T>(token: &CancellationToken, r: Result<T, SftpError>) -> Result<T, SftpError> {
    if token.is_cancelled() {
        return Err(SftpError::Cancelled);
    }
    r
}

async fn run_once(
    sh: &Arc<Shared>,
    job: &Arc<Job>,
    token: &CancellationToken,
) -> Result<(), SftpError> {
    let id = job.id;
    let progress = Arc::new(AtomicU64::new(0));
    let plan = tokio::select! {
        () = token.cancelled() => return Err(SftpError::Cancelled),
        r = scan(&job.src, &job.dst, &job.item, &job.dst_dir) => r?,
    };
    if !job.started.swap(true, Ordering::Relaxed) {
        sh.emit(TransferEvent::Started {
            id,
            total_bytes: plan.total,
            files: u32::try_from(plan.files.len()).unwrap_or(u32::MAX),
        });
    }
    job.dst.mkdir_p(&job.dst_dir).await?;
    let root = if job.dst.is_local() {
        Some(tokio::fs::canonicalize(&job.dst_dir).await?)
    } else {
        None
    };
    for d in &plan.dirs {
        job.dst.mkdir_p(d).await?;
        if let Some(root) = &root {
            let canon = tokio::fs::canonicalize(d).await?;
            if !canon.starts_with(root) {
                return Err(SftpError::UnsafePath(
                    "destination escapes the target folder".into(),
                ));
            }
        }
    }
    for s in &plan.skipped {
        sh.emit(TransferEvent::FileDone {
            id,
            path: s.clone(),
            skipped: true,
        });
    }

    let ticker = tokio::spawn(ticker(sh.clone(), id, progress.clone()));
    let chunk = effective_chunk(sh, job);
    let ctx = Ctx {
        sh,
        job,
        token,
        progress: &progress,
        root: root.as_deref().map(std::path::Path::to_path_buf),
        chunk,
    };
    let ctx = &ctx;
    let mut results = stream::iter(plan.files)
        .map(|t| async move { one_file(ctx, &t).await })
        .buffer_unordered(sh.config.parallel_files);
    let mut out = Ok(());
    while let Some(r) = results.next().await {
        if let Err(e) = r {
            out = Err(e);
            break;
        }
    }
    drop(results);
    ticker.abort();
    if out.is_ok() {
        sh.emit(TransferEvent::Progress {
            id,
            bytes_done: progress.load(Ordering::Relaxed),
            bytes_per_s: 0,
        });
    }
    out
}

/// Emits progress at 4 Hz (only when bytes moved), with an exponentially smoothed speed.
async fn ticker(sh: Arc<Shared>, id: TransferId, progress: Arc<AtomicU64>) {
    let mut last = 0u64;
    let mut at = Instant::now();
    let mut speed = 0f64;
    loop {
        tokio::time::sleep(Duration::from_millis(250)).await;
        let now = progress.load(Ordering::Relaxed);
        let dt = at.elapsed().as_secs_f64().max(0.001);
        #[allow(clippy::cast_precision_loss)]
        let inst = now.saturating_sub(last) as f64 / dt;
        speed = if speed == 0.0 {
            inst
        } else {
            speed * 0.7 + inst * 0.3
        };
        at = Instant::now();
        if now != last {
            last = now;
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            sh.emit(TransferEvent::Progress {
                id,
                bytes_done: now,
                bytes_per_s: speed as u64,
            });
        }
    }
}

fn effective_chunk(sh: &Shared, job: &Job) -> u32 {
    let mut lim: Option<u32> = None;
    let mut merge = |v: Option<u32>| {
        if let Some(v) = v {
            lim = Some(lim.map_or(v, |l| l.min(v)));
        }
    };
    if let Side::Remote(s) = &job.src {
        merge(s.rw_limits().0);
    }
    if let Side::Remote(s) = &job.dst {
        merge(s.rw_limits().1);
    }
    lim.map_or(sh.config.chunk_size, |l| l.clamp(1024, MAX_REQUEST))
}

struct Ctx<'a> {
    sh: &'a Arc<Shared>,
    job: &'a Arc<Job>,
    token: &'a CancellationToken,
    progress: &'a Arc<AtomicU64>,
    #[allow(dead_code)]
    root: Option<PathBuf>,
    chunk: u32,
}

enum Action {
    Skip,
    /// Write to this final path; `resume_final` = continue from the existing final file.
    Go {
        target: String,
        resume_final: bool,
    },
}

fn renamed(name: &str, n: u32) -> String {
    let (stem, ext) = match name.rfind('.') {
        Some(i) if i > 0 => (&name[..i], &name[i..]),
        _ => (name, ""),
    };
    format!("{stem} ({n}){ext}")
}

async fn resolve(ctx: &Ctx<'_>, t: &FileTask, final_path: &str) -> Result<Action, SftpError> {
    let job = ctx.job;
    let _gate = tokio::select! {
        () = ctx.token.cancelled() => return Err(SftpError::Cancelled),
        g = job.gate.lock() => g,
    };
    let Some(ex) = job.dst.lstat(final_path).await? else {
        return Ok(Action::Go {
            target: final_path.to_owned(),
            resume_final: false,
        });
    };
    if ex.kind == EntryKind::Dir {
        return Err(SftpError::Protocol("destination is a folder".into()));
    }
    let known = lk(&job.decisions).get(final_path).copied();
    let choice = if let Some(c) = known {
        c
    } else {
        let c = match job.policy {
            ConflictPolicy::Overwrite => ConflictChoice::Overwrite,
            ConflictPolicy::Skip => ConflictChoice::Skip,
            ConflictPolicy::Rename => ConflictChoice::Rename,
            ConflictPolicy::ResumeIfSmaller if ex.kind == EntryKind::File && ex.size < t.size => {
                ConflictChoice::Resume
            }
            _ => {
                let all = *lk(&job.apply_all);
                if let Some(c) = all {
                    c
                } else {
                    ask(ctx, t, final_path, &ex).await?
                }
            }
        };
        lk(&job.decisions).insert(final_path.to_owned(), c);
        c
    };
    Ok(match choice {
        ConflictChoice::Skip => Action::Skip,
        ConflictChoice::Overwrite => Action::Go {
            target: final_path.to_owned(),
            resume_final: false,
        },
        ConflictChoice::Resume => {
            let ok = ex.kind == EntryKind::File && ex.size < t.size;
            Action::Go {
                target: final_path.to_owned(),
                resume_final: ok,
            }
        }
        ConflictChoice::Rename => {
            let mut n = 1;
            loop {
                let cand = job.dst.join(&t.dst_dir, &renamed(&t.dst_name, n));
                if job.dst.lstat(&cand).await?.is_none() {
                    break Action::Go {
                        target: cand,
                        resume_final: false,
                    };
                }
                n += 1;
            }
        }
    })
}

async fn ask(
    ctx: &Ctx<'_>,
    t: &FileTask,
    final_path: &str,
    ex: &crate::Entry,
) -> Result<ConflictChoice, SftpError> {
    let pending_id = ConflictId(Uuid::now_v7());
    let (tx, rx) = tokio::sync::oneshot::channel();
    lk(&ctx.sh.pending).insert(pending_id, tx);
    ctx.sh.emit(TransferEvent::Conflict {
        id: ctx.job.id,
        pending_id,
        info: ConflictInfo {
            path: final_path.to_owned(),
            src_size: t.size,
            src_mtime: t.mtime,
            dst_size: ex.size,
            dst_mtime: ex.mtime,
        },
    });
    tokio::select! {
        () = ctx.token.cancelled() => {
            lk(&ctx.sh.pending).remove(&pending_id);
            Err(SftpError::Cancelled)
        }
        r = rx => {
            let (c, all) = r.map_err(|_| SftpError::Cancelled)?;
            if all {
                *lk(&ctx.job.apply_all) = Some(c);
            }
            Ok(c)
        }
    }
}

async fn tail_matches(job: &Job, t: &FileTask, tmp: &str, start: u64) -> Result<bool, SftpError> {
    let n = u32::try_from(start.min(TAIL_VERIFY)).unwrap_or(1 << 20);
    let off = start - u64::from(n);
    let a = read_at(job.src.open_read(&t.src).await?, off, n).await?;
    let b = read_at(job.dst.open_read(tmp).await?, off, n).await?;
    Ok(a.len() == n as usize && a == b)
}

async fn one_file(ctx: &Ctx<'_>, t: &FileTask) -> Result<(), SftpError> {
    let (sh, job, token) = (ctx.sh, ctx.job, ctx.token);
    let final_path = job.dst.join(&t.dst_dir, &t.dst_name);
    if lk(&job.done).contains(&final_path) {
        ctx.progress.fetch_add(t.size, Ordering::Relaxed);
        return Ok(());
    }
    let (target, resume_final) = match resolve(ctx, t, &final_path).await? {
        Action::Skip => {
            lk(&job.done).insert(final_path.clone());
            ctx.progress.fetch_add(t.size, Ordering::Relaxed);
            sh.emit(TransferEvent::FileDone {
                id: job.id,
                path: final_path,
                skipped: true,
            });
            return Ok(());
        }
        Action::Go {
            target,
            resume_final,
        } => (target, resume_final),
    };
    let _permit = tokio::select! {
        () = token.cancelled() => return Err(SftpError::Cancelled),
        p = sh.sem.clone().acquire_owned() => p.map_err(|_| SftpError::Cancelled)?,
    };
    let tmp = format!("{target}{TLPART}");
    let mut start = 0u64;
    match job.dst.lstat(&tmp).await? {
        Some(e) if e.kind == EntryKind::File => {
            if e.size <= t.size {
                start = e.size;
            }
        }
        Some(_) => job.dst.remove_file(&tmp).await?,
        None => {
            if resume_final {
                job.dst.rename_over(&target, &tmp).await?;
                start = job.dst.size_of(&tmp).await?;
            }
        }
    }
    if start > 0
        && !tail_matches(job, t, &tmp, start.min(t.size))
            .await
            .unwrap_or(false)
    {
        start = 0;
    }
    ctx.progress.fetch_add(start, Ordering::Relaxed);
    let src = job.src.open_read(&t.src).await?;
    let dst = match job.dst.open_write(&tmp, start).await {
        Ok(d) => d,
        Err(e) => {
            src.close().await;
            return Err(e);
        }
    };
    let plan = Plan {
        chunk: ctx.chunk,
        depth: sh.config.pipeline_depth,
        start,
        size: t.size,
    };
    check(token, copy(src, dst, plan, token, ctx.progress).await)?;
    if job.dst.size_of(&tmp).await? != t.size {
        job.dst.truncate(&tmp, t.size).await?;
        if job.dst.size_of(&tmp).await? != t.size {
            return Err(SftpError::Protocol("size mismatch after transfer".into()));
        }
    }
    job.dst
        .set_meta(&tmp, t.mtime, (t.mode != 0).then_some(t.mode))
        .await?;
    job.dst.rename_over(&tmp, &target).await?;
    lk(&job.done).insert(final_path);
    sh.emit(TransferEvent::FileDone {
        id: job.id,
        path: target,
        skipped: false,
    });
    Ok(())
}
