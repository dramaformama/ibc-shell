//! Transfer engine wiring: one engine, one replaceable event subscriber per window.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, PoisonError};

use ibc_shell_sftp::{TransferConfig, TransferEngine, TransferEvent};
use tauri::ipc::Channel;

/// Owns the `TransferEngine` and forwards its events to the UI channel.
pub struct TransferManager {
    /// The engine (cheap to call from commands).
    pub engine: TransferEngine,
    sink: Arc<Mutex<Subscribers>>,
}

type Subscribers = HashMap<String, Channel<TransferEvent>>;

impl TransferManager {
    /// Creates the engine and spawns the event pump.
    #[must_use]
    pub fn new() -> Self {
        let engine = TransferEngine::new(TransferConfig {
            parallel_files: 3,
            pipeline_depth: 64,
            chunk_size: 32 * 1024,
        });
        let sink = Arc::new(Mutex::new(Subscribers::new()));
        let mut rx = engine.subscribe();
        let s = sink.clone();
        tauri::async_runtime::spawn(async move {
            while let Some(ev) = rx.recv().await {
                let channels: Vec<_> = s
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner)
                    .values()
                    .cloned()
                    .collect();
                for ch in channels {
                    let _ = ch.send(ev.clone());
                }
            }
        });
        Self { engine, sink }
    }

    /// Replaces the subscriber (`transfers_subscribe`).
    pub fn set_subscriber(&self, window: &str, ch: Channel<TransferEvent>) {
        self.sink
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(window.into(), ch);
    }

    /// Removes a closed window's channel.
    pub fn remove_subscriber(&self, window: &str) {
        self.sink
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .remove(window);
    }
}

impl Default for TransferManager {
    fn default() -> Self {
        Self::new()
    }
}
