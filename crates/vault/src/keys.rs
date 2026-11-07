//! SSH key import (OpenSSH incl. encrypted, PEM PKCS#1/PKCS#8) and generation.
//! Only public metadata leaves this module; private bytes stay in `Zeroizing` buffers.

use russh::keys::ssh_key::{
    getrandom::SysRng, rand_core::UnwrapErr, Algorithm, HashAlg, LineEnding, PrivateKey,
};
use secrecy::{ExposeSecret, SecretString};
use zeroize::Zeroizing;

/// Key import / generation failures. Never contain key material.
#[derive(Debug, thiserror::Error)]
pub enum KeyError {
    /// Key is encrypted and no passphrase was given.
    #[error("passphrase required")]
    PassphraseRequired,
    /// Passphrase did not decrypt the key.
    #[error("wrong passphrase")]
    WrongPassphrase,
    /// Not a parseable private key.
    #[error("invalid key")]
    Invalid,
    /// Generation failed.
    #[error("key generation failed")]
    Generate,
}

/// Public facts about a key. Safe to send to the UI.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeyInfo {
    /// e.g. `ssh-ed25519`, `ssh-rsa`.
    pub algo: String,
    /// `SHA256:<base64>`.
    pub fingerprint: String,
    /// `authorized_keys`-style public key line.
    pub public_key: String,
}

/// Algorithms offered for generation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyGenAlgo {
    /// Default.
    Ed25519,
    /// RSA 4096.
    Rsa4096,
}

/// A freshly generated key: info plus the unencrypted OpenSSH private key text.
pub struct GeneratedKey {
    /// Public facts.
    pub info: KeyInfo,
    /// OpenSSH PEM; store via `Vault::put_secret`.
    pub private_openssh: Zeroizing<Vec<u8>>,
}

impl std::fmt::Debug for GeneratedKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GeneratedKey")
            .field("info", &self.info)
            .field("private_openssh", &"[REDACTED]")
            .finish()
    }
}

fn info_of(key: &PrivateKey) -> Result<KeyInfo, KeyError> {
    let public = key.public_key();
    Ok(KeyInfo {
        algo: public.algorithm().to_string(),
        fingerprint: public.fingerprint(HashAlg::Sha256).to_string(),
        public_key: public.to_openssh().map_err(|_| KeyError::Invalid)?,
    })
}

/// Parses a private key (any supported format, optionally passphrase-protected) and returns
/// its public metadata. The caller stores the original bytes with `Vault::put_secret`.
pub fn import_private_key(
    data: &[u8],
    passphrase: Option<&SecretString>,
) -> Result<KeyInfo, KeyError> {
    let text = Zeroizing::new(
        std::str::from_utf8(data)
            .map_err(|_| KeyError::Invalid)?
            .to_owned(),
    );
    let pass = passphrase.map(ExposeSecret::expose_secret);
    let key = russh::keys::decode_secret_key(&text, pass).map_err(|e| match e {
        russh::keys::Error::KeyIsEncrypted => KeyError::PassphraseRequired,
        _ if pass.is_some() => KeyError::WrongPassphrase,
        _ => KeyError::Invalid,
    })?;
    info_of(&key)
}

/// Generates a key with the OS RNG.
pub fn generate_key(algo: KeyGenAlgo) -> Result<GeneratedKey, KeyError> {
    let algorithm = match algo {
        KeyGenAlgo::Ed25519 => Algorithm::Ed25519,
        KeyGenAlgo::Rsa4096 => Algorithm::Rsa { hash: None },
    };
    let mut rng = UnwrapErr(SysRng);
    let key = match algo {
        KeyGenAlgo::Ed25519 => PrivateKey::random(&mut rng, algorithm),
        KeyGenAlgo::Rsa4096 => {
            let kp = russh::keys::ssh_key::private::RsaKeypair::random(&mut rng, 4096);
            kp.map(PrivateKey::from)
        }
    }
    .map_err(|_| KeyError::Generate)?;
    let pem = key
        .to_openssh(LineEnding::LF)
        .map_err(|_| KeyError::Generate)?;
    Ok(GeneratedKey {
        info: info_of(&key)?,
        private_openssh: Zeroizing::new(pem.as_bytes().to_vec()),
    })
}
