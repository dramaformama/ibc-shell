//! Pending interactive prompts (host key, keyboard-interactive, passphrase) and the
//! `Prompter` adapter that surfaces them as `SessionEvent`s (120 s timeout).

use std::collections::HashMap;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use ibc_shell_ssh::{ConnectStep, HostKeyDecision, HostKeyInfo, KbdInteractiveRequest, Prompter};
use secrecy::SecretString;
use tauri::ipc::Channel;
use tokio::sync::oneshot;
use uuid::Uuid;

use crate::api::dto::{PendingId, SessionEvent};
use crate::api::ApiError;

/// How long a prompt waits for the user.
pub const PROMPT_TIMEOUT: Duration = Duration::from_secs(120);

enum Slot {
    HostKey(oneshot::Sender<HostKeyDecision>),
    Kbd(oneshot::Sender<Option<Vec<SecretString>>>),
    Pass(oneshot::Sender<Option<SecretString>>),
}

/// Registry of unanswered prompts, keyed by `PendingId`, each owned by a session / sftp id.
#[derive(Default)]
pub struct PendingPrompts {
    map: Mutex<HashMap<PendingId, (Uuid, Slot)>>,
}

impl std::fmt::Debug for PendingPrompts {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("PendingPrompts")
    }
}

impl PendingPrompts {
    fn put(&self, owner: Uuid, slot: Slot) -> PendingId {
        let id = PendingId(Uuid::now_v7());
        self.map
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(id, (owner, slot));
        id
    }

    fn take(&self, id: PendingId) -> Option<Slot> {
        self.map
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .remove(&id)
            .map(|(_, s)| s)
    }

    /// Answers a host-key prompt. `NotFound` when expired / unknown / of another kind.
    pub fn reply_host_key(&self, id: PendingId, remember: bool) -> Result<(), ApiError> {
        match self.take(id) {
            Some(Slot::HostKey(tx)) => {
                let d = if remember {
                    HostKeyDecision::AcceptAndRemember
                } else {
                    HostKeyDecision::AcceptOnce
                };
                let _ = tx.send(d);
                Ok(())
            }
            _ => Err(ApiError::NotFound),
        }
    }

    /// Answers a keyboard-interactive prompt (`None` = cancel).
    pub fn reply_kbd(
        &self,
        id: PendingId,
        answers: Option<Vec<SecretString>>,
    ) -> Result<(), ApiError> {
        match self.take(id) {
            Some(Slot::Kbd(tx)) => {
                let _ = tx.send(answers);
                Ok(())
            }
            _ => Err(ApiError::NotFound),
        }
    }

    /// Answers a passphrase prompt (`None` = cancel).
    pub fn reply_passphrase(&self, id: PendingId, p: Option<SecretString>) -> Result<(), ApiError> {
        match self.take(id) {
            Some(Slot::Pass(tx)) => {
                let _ = tx.send(p);
                Ok(())
            }
            _ => Err(ApiError::NotFound),
        }
    }

    /// Drops every prompt owned by `owner`; their waiters observe a cancel / reject.
    pub fn cancel_owner(&self, owner: Uuid) {
        self.map
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .retain(|_, (o, _)| *o != owner);
    }

    /// Number of unanswered prompts.
    #[must_use]
    pub fn len(&self) -> usize {
        self.map
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .len()
    }

    /// True when nothing is pending.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// `Prompter` that emits `SessionEvent`s on a channel and awaits the matching reply.
pub struct EventPrompter {
    /// Owner id (session / sftp uuid) for cancellation.
    pub owner: Uuid,
    /// Event channel.
    pub events: Channel<SessionEvent>,
    /// Shared registry.
    pub pending: Arc<PendingPrompts>,
}

impl EventPrompter {
    async fn wait<T>(&self, rx: oneshot::Receiver<T>, id: PendingId) -> Option<T> {
        let r = tokio::time::timeout(PROMPT_TIMEOUT, rx).await;
        if r.is_err() {
            let _ = self.pending.take(id);
        }
        r.ok().and_then(Result::ok)
    }
}

impl Prompter for EventPrompter {
    fn progress(&self, hop_label: &str, step: ConnectStep) {
        let _ = self.events.send(SessionEvent::Connecting {
            step,
            hop: hop_label.to_owned(),
        });
    }

    fn attempt(&self, hop_label: &str, addr: &str, failure: Option<&str>) {
        let _ = self.events.send(SessionEvent::Attempt {
            hop: hop_label.to_owned(),
            addr: addr.to_owned(),
            failure: failure.map(str::to_owned),
        });
    }

    async fn confirm_unknown_host(&self, info: HostKeyInfo) -> HostKeyDecision {
        let (tx, rx) = oneshot::channel();
        let pending_id = self.pending.put(self.owner, Slot::HostKey(tx));
        if self
            .events
            .send(SessionEvent::HostKeyPrompt { pending_id, info })
            .is_err()
        {
            let _ = self.pending.take(pending_id);
            return HostKeyDecision::Reject;
        }
        self.wait(rx, pending_id)
            .await
            .unwrap_or(HostKeyDecision::Reject)
    }

    async fn keyboard_interactive(&self, req: KbdInteractiveRequest) -> Option<Vec<SecretString>> {
        let (tx, rx) = oneshot::channel();
        let pending_id = self.pending.put(self.owner, Slot::Kbd(tx));
        let ev = SessionEvent::KbdInteractive {
            pending_id,
            name: req.name,
            instruction: req.instruction,
            prompts: req.prompts,
        };
        if self.events.send(ev).is_err() {
            let _ = self.pending.take(pending_id);
            return None;
        }
        self.wait(rx, pending_id).await.flatten()
    }

    async fn passphrase(&self, identity_label: &str) -> Option<SecretString> {
        let (tx, rx) = oneshot::channel();
        let pending_id = self.pending.put(self.owner, Slot::Pass(tx));
        let ev = SessionEvent::PassphrasePrompt {
            pending_id,
            identity_label: identity_label.to_owned(),
        };
        if self.events.send(ev).is_err() {
            let _ = self.pending.take(pending_id);
            return None;
        }
        self.wait(rx, pending_id).await.flatten()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reply_to_unknown_prompt_is_not_found() {
        let p = PendingPrompts::default();
        assert_eq!(
            p.reply_host_key(PendingId(Uuid::now_v7()), true),
            Err(ApiError::NotFound)
        );
    }

    #[test]
    fn wrong_kind_consumes_and_fails() {
        let p = PendingPrompts::default();
        let (tx, _rx) = oneshot::channel();
        let id = p.put(Uuid::nil(), Slot::HostKey(tx));
        assert_eq!(p.reply_kbd(id, None), Err(ApiError::NotFound));
        assert!(p.is_empty());
    }

    #[test]
    fn cancel_owner_drops_waiters() {
        let p = PendingPrompts::default();
        let owner = Uuid::now_v7();
        let (tx, mut rx) = oneshot::channel();
        p.put(owner, Slot::HostKey(tx));
        p.cancel_owner(owner);
        assert!(p.is_empty());
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn host_key_reply_delivers_decision() {
        let p = PendingPrompts::default();
        let (tx, mut rx) = oneshot::channel();
        let id = p.put(Uuid::nil(), Slot::HostKey(tx));
        p.reply_host_key(id, true).unwrap();
        assert_eq!(rx.try_recv().unwrap(), HostKeyDecision::AcceptAndRemember);
    }
}
