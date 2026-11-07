//! Backend idle auto-lock (ADR-007). The deadline lives next to the DEK so the vault locks
//! itself even when the UI timer never fires (throttled / hung / compromised webview).

use std::sync::{Mutex, PoisonError};
use std::time::{Duration, Instant};

struct State {
    timeout: Option<Duration>,
    last: Instant,
}

/// Idle timer: `timeout = None` never expires.
pub(crate) struct Idle(Mutex<State>);

impl Default for Idle {
    fn default() -> Self {
        Self(Mutex::new(State {
            timeout: Some(DEFAULT_TIMEOUT),
            last: Instant::now(),
        }))
    }
}

/// `security.autolock_minutes` default (15 min).
pub const DEFAULT_TIMEOUT: Duration = Duration::from_mins(15);

impl Idle {
    fn with<R>(&self, f: impl FnOnce(&mut State) -> R) -> R {
        f(&mut self.0.lock().unwrap_or_else(PoisonError::into_inner))
    }

    /// Replaces the timeout and restarts the clock.
    pub(crate) fn set_timeout(&self, timeout: Option<Duration>) {
        self.with(|s| {
            s.timeout = timeout;
            s.last = Instant::now();
        });
    }

    /// Records user activity at `now`.
    pub(crate) fn touch_at(&self, now: Instant) {
        self.with(|s| s.last = s.last.max(now));
    }

    /// True once `now` is past `last activity + timeout`.
    pub(crate) fn expired_at(&self, now: Instant) -> bool {
        self.with(|s| {
            s.timeout
                .is_some_and(|t| now.saturating_duration_since(s.last) >= t)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expires_after_timeout_and_touch_extends() {
        let i = Idle::default();
        i.set_timeout(Some(Duration::from_secs(60)));
        let t0 = Instant::now();
        assert!(!i.expired_at(t0 + Duration::from_secs(59)));
        i.touch_at(t0 + Duration::from_secs(50));
        assert!(!i.expired_at(t0 + Duration::from_secs(100)));
        assert!(i.expired_at(t0 + Duration::from_secs(111)));
    }

    #[test]
    fn none_never_expires_and_old_touch_is_ignored() {
        let i = Idle::default();
        i.set_timeout(None);
        assert!(!i.expired_at(Instant::now() + Duration::from_secs(1 << 30)));
        i.set_timeout(Some(Duration::from_secs(10)));
        let late = Instant::now() + Duration::from_secs(5);
        i.touch_at(late);
        i.touch_at(Instant::now()); // earlier: must not move the clock back
        assert!(!i.expired_at(late + Duration::from_secs(9)));
    }

    fn vault() -> crate::Vault {
        let store = std::sync::Arc::new(ibc_shell_store::Store::open_in_memory().unwrap());
        let v = crate::Vault::new(store, std::sync::Arc::new(crate::MemoryKeyStore::default()));
        v.init(crate::VaultMode::OsKeychain, None).unwrap();
        v
    }

    #[test]
    fn expired_vault_refuses_secrets_and_zeroizes_the_dek() {
        let v = vault();
        let id = v.put_secret(b"pw").unwrap();
        v.set_idle_timeout(Some(Duration::from_millis(20)));
        std::thread::sleep(Duration::from_millis(40));
        assert!(matches!(v.get_secret(id), Err(crate::VaultError::Locked)));
        assert!(!v.dek_loaded());
        assert_eq!(v.state().unwrap(), crate::VaultState::Locked);
        // Unlocking again starts a fresh idle period.
        v.unlock_with_keystore().unwrap();
        assert_eq!(&*v.get_secret(id).unwrap(), b"pw");
    }

    #[test]
    fn sweeper_hook_locks_once_and_touch_keeps_it_open() {
        let v = vault();
        v.set_idle_timeout(Some(Duration::from_millis(60)));
        for _ in 0..4 {
            std::thread::sleep(Duration::from_millis(25));
            v.touch();
            assert!(!v.lock_if_idle());
        }
        std::thread::sleep(Duration::from_millis(80));
        assert!(v.lock_if_idle());
        assert!(!v.lock_if_idle(), "already locked");
        v.set_idle_timeout(None);
        v.unlock_with_keystore().unwrap();
        std::thread::sleep(Duration::from_millis(80));
        assert!(!v.lock_if_idle());
    }
}
