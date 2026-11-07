//! Mapping of russh errors to [`SshError`]. Never forwards free-form text that could carry
//! credentials (IO errors are reduced to their kind).

use crate::SshError;

/// Converts a russh error into a sanitized [`SshError`].
pub(crate) fn map_russh(e: &russh::Error) -> SshError {
    use russh::Error as E;
    match e {
        E::NoCommonAlgo { kind, .. } => SshError::Negotiation(format!("{kind:?}")),
        E::ConnectionTimeout | E::KeepaliveTimeout | E::InactivityTimeout | E::Elapsed(_) => {
            SshError::Timeout
        }
        E::IO(io) => SshError::Network(format!("{:?}", io.kind())),
        E::Disconnect | E::HUP | E::SendError => SshError::Network("disconnected".into()),
        // Never forward free-form text from the library: fixed strings only.
        E::NotAuthenticated | E::WrongServerSig | E::Kex | E::Inconsistent => {
            SshError::Protocol("protocol violation".into())
        }
        _ => SshError::Protocol("protocol error".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn io_is_reduced_to_kind() {
        let e = russh::Error::IO(std::io::Error::new(
            std::io::ErrorKind::ConnectionRefused,
            "secret-host-detail",
        ));
        match map_russh(&e) {
            SshError::Network(s) => {
                assert_eq!(s, "ConnectionRefused");
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn other_errors_use_fixed_strings() {
        for e in [
            russh::Error::Inconsistent,
            russh::Error::Version,
            russh::Error::Keys(russh::keys::Error::CouldNotReadKey),
        ] {
            match map_russh(&e) {
                SshError::Protocol(s) => assert!(s == "protocol violation" || s == "protocol error", "{s}"),
                other => panic!("{other:?}"),
            }
        }
    }

    #[test]
    fn timeouts() {
        assert!(matches!(
            map_russh(&russh::Error::KeepaliveTimeout),
            SshError::Timeout
        ));
    }
}
