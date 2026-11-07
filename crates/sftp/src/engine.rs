//! `TransferEngine`: queue, pause/resume/cancel/retry, conflict replies.

use std::collections::{HashMap, HashSet};
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use tokio::sync::{mpsc, oneshot, Semaphore};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::job::{drive, Ctl, Job, Status};
use crate::scan::{base_name, dst_name};
use crate::side::Side;
use crate::{
    ConflictChoice, ConflictId, SftpError, TransferConfig, TransferEvent, TransferId, TransferJob,
};

pub(crate) type Reply = (ConflictChoice, bool);

pub(crate) fn lk<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(PoisonError::into_inner)
}

/// State shared by the engine and its job tasks.
pub(crate) struct Shared {
    pub config: TransferConfig,
    pub sem: Arc<Semaphore>,
    tx: Mutex<Option<mpsc::UnboundedSender<TransferEvent>>>,
    jobs: Mutex<HashMap<TransferId, Arc<Job>>>,
    pub pending: Mutex<HashMap<ConflictId, oneshot::Sender<Reply>>>,
}

impl Shared {
    pub(crate) fn emit(&self, ev: TransferEvent) {
        if let Some(tx) = lk(&self.tx).as_ref() {
            let _ = tx.send(ev);
        }
    }
}

/// The queue. `Send + Sync`; `enqueue`/`resume`/`retry` must run inside a tokio runtime.
#[derive(Debug)]
pub struct TransferEngine {
    shared: Arc<Shared>,
}

impl std::fmt::Debug for Shared {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Shared").field("config", &self.config).finish_non_exhaustive()
    }
}

impl TransferEngine {
    /// Creates an idle engine.
    #[must_use]
    pub fn new(config: TransferConfig) -> Self {
        let config = TransferConfig {
            parallel_files: config.parallel_files.max(1),
            pipeline_depth: config.pipeline_depth.max(1),
            chunk_size: config.chunk_size.max(1024),
        };
        Self {
            shared: Arc::new(Shared {
                sem: Arc::new(Semaphore::new(config.parallel_files)),
                config,
                tx: Mutex::new(None),
                jobs: Mutex::new(HashMap::new()),
                pending: Mutex::new(HashMap::new()),
            }),
        }
    }

    /// Event stream; one subscriber (the bridge), a new call replaces the previous one.
    #[must_use]
    pub fn subscribe(&self) -> mpsc::UnboundedReceiver<TransferEvent> {
        let (tx, rx) = mpsc::unbounded_channel();
        *lk(&self.shared.tx) = Some(tx);
        rx
    }

    /// Queues a job; one [`TransferId`] per item.
    #[allow(clippy::needless_pass_by_value)] // frozen API
    pub fn enqueue(&self, job: TransferJob) -> Result<Vec<TransferId>, SftpError> {
        let src = Side::from(&job.src);
        let dst = Side::from(&job.dst);
        let mut ids = Vec::with_capacity(job.items.len());
        for item in &job.items {
            let id = TransferId(Uuid::now_v7());
            let name = dst_name(&src, &dst, &base_name(&src, item))
                .unwrap_or_else(|| base_name(&src, item));
            let j = Arc::new(Job {
                id,
                src: src.clone(),
                dst: dst.clone(),
                item: item.clone(),
                dst_dir: job.dst_dir.clone(),
                policy: job.conflict,
                ctl: Mutex::new(Ctl {
                    status: Status::Running,
                    token: CancellationToken::new(),
                    active: true,
                }),
                apply_all: Mutex::new(None),
                decisions: Mutex::new(HashMap::new()),
                done: Mutex::new(HashSet::new()),
                gate: tokio::sync::Mutex::new(()),
                started: AtomicBool::new(false),
            });
            self.shared.emit(TransferEvent::Queued {
                id,
                src_path: item.clone(),
                dst_path: dst.join(&job.dst_dir, &name),
            });
            lk(&self.shared.jobs).insert(id, j.clone());
            tokio::spawn(drive(self.shared.clone(), j));
            ids.push(id);
        }
        Ok(ids)
    }

    fn job(&self, id: TransferId) -> Result<Arc<Job>, SftpError> {
        lk(&self.shared.jobs).get(&id).cloned().ok_or(SftpError::NotFound)
    }

    /// Pauses (in-flight writes land, partial file kept).
    pub fn pause(&self, id: TransferId) -> Result<(), SftpError> {
        let j = self.job(id)?;
        let mut c = lk(&j.ctl);
        if c.status != Status::Running {
            return Err(SftpError::Protocol("transfer is not running".into()));
        }
        c.status = Status::Paused;
        c.token.cancel();
        Ok(())
    }

    fn restart(&self, id: TransferId, from: &[Status]) -> Result<(), SftpError> {
        let j = self.job(id)?;
        let mut c = lk(&j.ctl);
        if !from.contains(&c.status) {
            return Err(SftpError::Protocol("transfer cannot be resumed now".into()));
        }
        c.status = Status::Running;
        c.token = CancellationToken::new();
        if !c.active {
            c.active = true;
            tokio::spawn(drive(self.shared.clone(), j.clone()));
        }
        Ok(())
    }

    /// Resumes (byte-offset resume).
    pub fn resume(&self, id: TransferId) -> Result<(), SftpError> {
        self.restart(id, &[Status::Paused])
    }

    /// Cancels (partial files are kept for resume).
    pub fn cancel(&self, id: TransferId) -> Result<(), SftpError> {
        let j = self.job(id)?;
        let mut c = lk(&j.ctl);
        if matches!(c.status, Status::Done | Status::Cancelled) {
            return Ok(());
        }
        c.status = Status::Cancelled;
        c.token.cancel();
        if !c.active {
            drop(c);
            self.shared.emit(TransferEvent::Cancelled { id });
        }
        Ok(())
    }

    /// Retries a failed/cancelled transfer.
    pub fn retry(&self, id: TransferId) -> Result<(), SftpError> {
        self.restart(id, &[Status::Failed, Status::Cancelled])
    }

    /// Answers a conflict; `apply_to_all` covers later conflicts of the same transfer.
    pub fn conflict_reply(
        &self,
        pending_id: ConflictId,
        choice: ConflictChoice,
        apply_to_all: bool,
    ) -> Result<(), SftpError> {
        let tx = lk(&self.shared.pending).remove(&pending_id).ok_or(SftpError::NotFound)?;
        tx.send((choice, apply_to_all)).map_err(|_| SftpError::NotFound)
    }
}
