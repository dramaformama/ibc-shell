//! Algorithm policy (security.md §SSH policy) and client `Config` construction.

use std::borrow::Cow;
use std::sync::Arc;
use std::time::Duration;

use russh::client::Config;
use russh::keys::{Algorithm, EcdsaCurve, HashAlg};
use russh::{cipher, kex, mac, Preferred};

/// Key exchange: PQ hybrid first, then curve25519, ECDH, DH-16/14-SHA2; strict-kex always.
const KEX: &[kex::Name] = &[
    kex::MLKEM768X25519_SHA256,
    kex::CURVE25519,
    kex::CURVE25519_PRE_RFC_8731,
    kex::ECDH_SHA2_NISTP256,
    kex::ECDH_SHA2_NISTP384,
    kex::ECDH_SHA2_NISTP521,
    kex::DH_G16_SHA512,
    kex::DH_G14_SHA256,
    kex::EXTENSION_SUPPORT_AS_CLIENT,
    kex::EXTENSION_OPENSSH_STRICT_KEX_AS_CLIENT,
];

/// Extra KEX for the per-host "legacy algorithms" toggle (appended last).
const KEX_LEGACY: &[kex::Name] = &[kex::DH_GEX_SHA256, kex::DH_G14_SHA1, kex::DH_G1_SHA1];

const CIPHERS: &[cipher::Name] = &[
    cipher::CHACHA20_POLY1305,
    cipher::AES_256_GCM,
    cipher::AES_128_GCM,
    cipher::AES_256_CTR,
    cipher::AES_128_CTR,
];

const CIPHERS_LEGACY: &[cipher::Name] = &[
    cipher::AES_256_CBC,
    cipher::AES_192_CBC,
    cipher::AES_128_CBC,
];

/// ETM first; plain SHA-2 MACs only matter for CTR when the server offers nothing else.
const MACS: &[mac::Name] = &[
    mac::HMAC_SHA512_ETM,
    mac::HMAC_SHA256_ETM,
    mac::HMAC_SHA512,
    mac::HMAC_SHA256,
];

const MACS_LEGACY: &[mac::Name] = &[mac::HMAC_SHA1_ETM, mac::HMAC_SHA1];

const HOST_KEYS: &[Algorithm] = &[
    Algorithm::Ed25519,
    Algorithm::Ecdsa {
        curve: EcdsaCurve::NistP256,
    },
    Algorithm::Ecdsa {
        curve: EcdsaCurve::NistP384,
    },
    Algorithm::Ecdsa {
        curve: EcdsaCurve::NistP521,
    },
    Algorithm::Rsa {
        hash: Some(HashAlg::Sha512),
    },
    Algorithm::Rsa {
        hash: Some(HashAlg::Sha256),
    },
];

/// SHA-1 `ssh-rsa` (legacy toggle only).
const HOST_KEYS_LEGACY: &[Algorithm] = &[Algorithm::Rsa { hash: None }];

fn extend<T: Clone>(base: &[T], extra: &[T], with_extra: bool) -> Cow<'static, [T]> {
    let mut v = base.to_vec();
    if with_extra {
        v.extend_from_slice(extra);
    }
    Cow::Owned(v)
}

/// The algorithm lists offered to servers. `legacy` appends SHA-1 / CBC / group14-sha1 /
/// `ssh-rsa` (Phase 3 per-host toggle, default off). Compression is never offered.
#[must_use]
pub fn preferred(legacy: bool) -> Preferred {
    Preferred {
        kex: extend(KEX, KEX_LEGACY, legacy),
        key: extend(HOST_KEYS, HOST_KEYS_LEGACY, legacy),
        host_key_certificates: Cow::Borrowed(&[]),
        cipher: extend(CIPHERS, CIPHERS_LEGACY, legacy),
        mac: extend(MACS, MACS_LEGACY, legacy),
        compression: Cow::Borrowed(&[russh::compression::NONE]),
    }
}

/// Client config for one hop. `keepalive_s == 0` disables keepalives; otherwise 3 unanswered
/// keepalives close the connection.
#[must_use]
pub fn client_config(keepalive_s: u32, legacy: bool) -> Arc<Config> {
    Arc::new(Config {
        preferred: preferred(legacy),
        keepalive_interval: (keepalive_s > 0).then(|| Duration::from_secs(u64::from(keepalive_s))),
        keepalive_max: 3,
        nodelay: true,
        ..Config::default()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names<'a>(v: impl IntoIterator<Item = &'a str>) -> Vec<String> {
        v.into_iter().map(str::to_owned).collect()
    }

    #[test]
    fn strict_kex_always_offered() {
        for legacy in [false, true] {
            let p = preferred(legacy);
            assert!(p.kex.contains(&kex::EXTENSION_OPENSSH_STRICT_KEX_AS_CLIENT));
        }
    }

    #[test]
    fn kex_order_matches_policy() {
        let p = preferred(false);
        let k = names(p.kex.iter().map(AsRef::as_ref));
        assert_eq!(k[0], "mlkem768x25519-sha256");
        assert_eq!(k[1], "curve25519-sha256");
        assert!(k.contains(&"diffie-hellman-group16-sha512".to_owned()));
        assert!(k.contains(&"diffie-hellman-group14-sha256".to_owned()));
        assert!(k.contains(&"kex-strict-c-v00@openssh.com".to_owned()));
    }

    #[test]
    fn default_has_no_legacy() {
        let p = preferred(false);
        let k = names(p.kex.iter().map(AsRef::as_ref));
        assert!(!k.iter().any(|n| n.contains("sha1")));
        let c = names(p.cipher.iter().map(AsRef::as_ref));
        assert!(!c.iter().any(|n| n.contains("cbc") || n.contains("3des")));
        let m = names(p.mac.iter().map(AsRef::as_ref));
        assert!(!m.iter().any(|n| n.contains("sha1")));
        assert!(!p.key.contains(&Algorithm::Rsa { hash: None }));
        assert!(!p.key.contains(&Algorithm::Dsa));
        assert_eq!(p.compression.len(), 1);
    }

    #[test]
    fn ciphers_exact() {
        let p = preferred(false);
        let c = names(p.cipher.iter().map(AsRef::as_ref));
        assert_eq!(
            c,
            [
                "chacha20-poly1305@openssh.com",
                "aes256-gcm@openssh.com",
                "aes128-gcm@openssh.com",
                "aes256-ctr",
                "aes128-ctr"
            ]
        );
    }

    #[test]
    fn legacy_appends_after_modern() {
        let p = preferred(true);
        assert!(p.key.contains(&Algorithm::Rsa { hash: None }));
        assert_eq!(p.key[0], Algorithm::Ed25519);
        let m = names(p.mac.iter().map(AsRef::as_ref));
        assert!(m.iter().any(|n| n == "hmac-sha1"));
    }

    #[test]
    fn keepalive_config() {
        let c = client_config(30, false);
        assert_eq!(c.keepalive_interval, Some(Duration::from_secs(30)));
        assert_eq!(c.keepalive_max, 3);
        assert!(client_config(0, false).keepalive_interval.is_none());
    }
}
