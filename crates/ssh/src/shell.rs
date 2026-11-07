//! Interactive PTY shell with credit-based backpressure.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use russh::client::Msg;
use russh::{ChannelMsg, ChannelReadHalf, ChannelWriteHalf};
use tokio::sync::Notify;

use crate::session::CREDIT_WINDOW;
use crate::SshError;

/// Counter of bytes delivered to the consumer but not yet acknowledged.
#[derive(Debug, Default)]
pub(crate) struct Credit {
    unacked: AtomicU64,
    notify: Notify,
}

impl Credit {
    pub(crate) fn add(&self, n: usize) {
        self.unacked.fetch_add(n as u64, Ordering::SeqCst);
    }

    pub(crate) fn ack(&self, n: u32) {
        let _ = self
            .unacked
            .try_update(Ordering::SeqCst, Ordering::SeqCst, |v| {
                Some(v.saturating_sub(u64::from(n)))
            });
        self.notify.notify_one();
    }

    pub(crate) fn unacked(&self) -> u64 {
        self.unacked.load(Ordering::SeqCst)
    }

    fn exhausted(&self) -> bool {
        self.unacked() > u64::from(CREDIT_WINDOW)
    }

    /// Resolves once unacked bytes are within the window.
    pub(crate) async fn wait_available(&self) {
        loop {
            let notified = self.notify.notified();
            tokio::pin!(notified);
            notified.as_mut().enable();
            if !self.exhausted() {
                return;
            }
            notified.await;
        }
    }
}

/// Item read from a shell.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ShellOutput {
    /// stdout/stderr bytes (merged, as a terminal sees them).
    Data(Vec<u8>),
    /// Remote process exit status.
    ExitStatus(u32),
}

/// Interactive shell with credit-based backpressure ([`CREDIT_WINDOW`]).
pub struct Shell {
    rx: ChannelReadHalf,
    tx: ChannelWriteHalf<Msg>,
    credit: Arc<Credit>,
}

impl std::fmt::Debug for Shell {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Shell")
            .field("unacked", &self.credit.unacked())
            .finish_non_exhaustive()
    }
}

impl Shell {
    pub(crate) fn new(rx: ChannelReadHalf, tx: ChannelWriteHalf<Msg>) -> Self {
        Self {
            rx,
            tx,
            credit: Arc::new(Credit::default()),
        }
    }

    /// Next output item; `None` = channel closed (EOF / disconnect). Waits while the credit
    /// window is exhausted (the SSH channel is not drained meanwhile, so the server stalls).
    pub async fn read(&mut self) -> Option<ShellOutput> {
        self.credit.wait_available().await;
        loop {
            match self.rx.wait().await? {
                ChannelMsg::Data { data } | ChannelMsg::ExtendedData { data, .. } => {
                    self.credit.add(data.len());
                    return Some(ShellOutput::Data(data.to_vec()));
                }
                ChannelMsg::ExitStatus { exit_status } => {
                    return Some(ShellOutput::ExitStatus(exit_status));
                }
                ChannelMsg::Close => return None,
                _ => {}
            }
        }
    }

    /// Sends input bytes.
    pub async fn write(&self, data: &[u8]) -> Result<(), SshError> {
        self.tx
            .data_bytes(data.to_vec())
            .await
            .map_err(|e| crate::error::map_russh(&e))
    }

    /// Window change.
    pub async fn resize(&self, cols: u16, rows: u16) -> Result<(), SshError> {
        self.tx
            .window_change(u32::from(cols), u32::from(rows), 0, 0)
            .await
            .map_err(|e| crate::error::map_russh(&e))
    }

    /// Returns credit for `bytes` consumed by the UI (`shell_ack`).
    pub fn ack(&self, bytes: u32) {
        self.credit.ack(bytes);
    }

    /// Sends EOF + close.
    pub async fn close(self) {
        let _ = self.tx.eof().await;
        let _ = self.tx.close().await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn available_under_window() {
        let c = Credit::default();
        c.add(CREDIT_WINDOW as usize);
        tokio::time::timeout(Duration::from_millis(50), c.wait_available())
            .await
            .expect("at the limit is still available");
    }

    #[tokio::test]
    async fn blocks_over_window_until_ack() {
        let c = Arc::new(Credit::default());
        c.add(CREDIT_WINDOW as usize + 10);
        let c2 = Arc::clone(&c);
        let waiter = tokio::spawn(async move { c2.wait_available().await });
        tokio::time::sleep(Duration::from_millis(30)).await;
        assert!(!waiter.is_finished());
        c.ack(11);
        tokio::time::timeout(Duration::from_millis(500), waiter)
            .await
            .expect("unblocked")
            .expect("join");
    }

    #[test]
    fn ack_saturates() {
        let c = Credit::default();
        c.add(5);
        c.ack(100);
        assert_eq!(c.unacked(), 0);
    }
}
