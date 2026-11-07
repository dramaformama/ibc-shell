//! Transfer engine wiring: one engine, one replaceable event subscriber.

use std::sync::{Arc, Mutex, PoisonError};

use ibc_shell_sftp::{TransferConfig, TransferEngine, TransferEvent};
use tauri::ipc::Channel;

/// Owns the `TransferEngine` and forwards its events to the UI channel.
pub struct TransferManager {
    /// The engine (cheap to call from commands).
    pub engine: TransferEngine,
    sink: Arc<Mutex<Option<Channel<TransferEvent>>>>,
}

impl TransferManager {
    /// Creates the engine and spawns the event pump.
    #[must_use]
    pub fn new() -> Self {
        let engine = TransferEngine::new(TransferConfig {
            parallel_files: 3,
            pipeline_depth: 64,
            chunk_size: 32 * 1024,
        });
        let sink: Arc<Mutex<Option<Channel<TransferEvent>>>> = Arc::new(Mutex::new(None));
        let mut rx = engine.subscribe();
        let s = sink.clone();
        tauri::async_runtime::spawn(async move {
            while let Some(ev) = rx.recv().await {
                let ch = s.lock().unwrap_or_else(PoisonError::into_inner).clone();
                if let Some(ch) = ch {
                    let _ = ch.send(ev);
                }
            }
        });
        Self { engine, sink }
    }

    /// Replaces the subscriber (`transfers_subscribe`).
    pub fn set_subscriber(&self, ch: Channel<TransferEvent>) {
        *self.sink.lock().unwrap_or_else(PoisonError::into_inner) = Some(ch);
    }
}

impl Default for TransferManager {
    fn default() -> Self {
        Self::new()
    }
}
