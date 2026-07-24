//! AES-256-GCM sealing of secret blobs under a keyring-held master key.

use std::sync::Arc;

use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Key, Nonce};
use nh_types::Secret;

use crate::error::{SecretsError, SecretsResult};
use crate::keyring::Keyring;

/// AES-256 key length.
const KEY_LEN: usize = 32;
/// AES-GCM nonce length (96 bits, the standard).
const NONCE_LEN: usize = 12;
/// The keyring entry name the master key is stored under, unless overridden.
pub(crate) const DEFAULT_ENTRY: &str = "networkhub-master-key";

/// Seals and opens secret blobs with AES-256-GCM under a master key that lives
/// in a [`Keyring`]. Only sealed bytes ever leave this type, so they are safe to
/// persist in `config.db` (§7.1) — losing the database leaks nothing.
pub struct SecretStore {
    key: Secret<[u8; KEY_LEN]>,
    keyring: Arc<dyn Keyring>,
    entry: String,
}

impl std::fmt::Debug for SecretStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Never render the key.
        f.debug_struct("SecretStore")
            .field("entry", &self.entry)
            .finish_non_exhaustive()
    }
}

impl SecretStore {
    /// Opens the store over `keyring`, loading the master key or, on first run,
    /// generating one and storing it in the keyring.
    ///
    /// # Errors
    /// Returns [`SecretsError`] if the keyring fails, the RNG fails, or a stored
    /// master key is the wrong length.
    pub fn open(keyring: Arc<dyn Keyring>) -> SecretsResult<Self> {
        Self::open_entry(keyring, DEFAULT_ENTRY)
    }

    /// Like [`open`](Self::open) but with an explicit keyring entry name (for
    /// tests or multiple isolated stores).
    ///
    /// # Errors
    /// See [`open`](Self::open).
    pub fn open_entry(keyring: Arc<dyn Keyring>, entry: &str) -> SecretsResult<Self> {
        let key = if let Some(bytes) = keyring.get(entry)? {
            let arr: [u8; KEY_LEN] = bytes
                .as_slice()
                .try_into()
                .map_err(|_| SecretsError::Corrupt("master key is not 32 bytes".to_owned()))?;
            Secret::new(arr)
        } else {
            let arr = random_key()?;
            keyring.set(entry, &arr)?;
            Secret::new(arr)
        };
        Ok(Self {
            key,
            keyring,
            entry: entry.to_owned(),
        })
    }

    /// Seals `plaintext`, returning `nonce || ciphertext||tag`. Each call uses a
    /// fresh random nonce, so sealing the same plaintext twice yields different
    /// bytes.
    ///
    /// # Errors
    /// Returns [`SecretsError::Rng`] or [`SecretsError::Crypto`] on failure.
    pub fn seal(&self, plaintext: &[u8]) -> SecretsResult<Vec<u8>> {
        seal_with(self.key.expose(), plaintext)
    }

    /// Opens a blob produced by [`seal`](Self::seal).
    ///
    /// # Errors
    /// Returns [`SecretsError::Crypto`] if the blob is truncated, was sealed
    /// under a different key, or has been tampered with (the GCM tag fails).
    pub fn unseal(&self, sealed: &[u8]) -> SecretsResult<Vec<u8>> {
        open_with(self.key.expose(), sealed)
    }

    /// Rotates the master key: generates a new one, stores it in the keyring, and
    /// returns a [`PriorKey`] that can still open blobs sealed under the old key
    /// — so the caller can re-seal each stored blob under the new key.
    ///
    /// # Errors
    /// Returns [`SecretsError`] if the RNG or the keyring fails. On keyring
    /// failure the in-memory key is left unchanged.
    pub fn rotate(&mut self) -> SecretsResult<PriorKey> {
        let new_key = random_key()?;
        self.keyring.set(&self.entry, &new_key)?;
        let prior = PriorKey {
            key: self.key.clone(),
        };
        self.key = Secret::new(new_key);
        Ok(prior)
    }
}

/// A handle to a superseded master key, kept only long enough to re-seal blobs
/// after a [`rotate`](SecretStore::rotate). Zeroized on drop.
pub struct PriorKey {
    key: Secret<[u8; KEY_LEN]>,
}

impl std::fmt::Debug for PriorKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("PriorKey(<redacted>)")
    }
}

impl PriorKey {
    /// Opens a blob that was sealed under the previous master key.
    ///
    /// # Errors
    /// Returns [`SecretsError::Crypto`] if the blob does not open under this key.
    pub fn unseal(&self, sealed: &[u8]) -> SecretsResult<Vec<u8>> {
        open_with(self.key.expose(), sealed)
    }
}

fn random_key() -> SecretsResult<[u8; KEY_LEN]> {
    let mut key = [0u8; KEY_LEN];
    getrandom::getrandom(&mut key).map_err(|e| SecretsError::Rng(e.to_string()))?;
    Ok(key)
}

fn seal_with(key: &[u8; KEY_LEN], plaintext: &[u8]) -> SecretsResult<Vec<u8>> {
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key));
    let mut nonce = [0u8; NONCE_LEN];
    getrandom::getrandom(&mut nonce).map_err(|e| SecretsError::Rng(e.to_string()))?;
    let ciphertext = cipher
        .encrypt(Nonce::from_slice(&nonce), plaintext)
        .map_err(|_| SecretsError::Crypto("sealing failed".to_owned()))?;

    let mut out = Vec::with_capacity(NONCE_LEN + ciphertext.len());
    out.extend_from_slice(&nonce);
    out.extend_from_slice(&ciphertext);
    Ok(out)
}

fn open_with(key: &[u8; KEY_LEN], sealed: &[u8]) -> SecretsResult<Vec<u8>> {
    if sealed.len() < NONCE_LEN {
        return Err(SecretsError::Crypto("sealed blob is truncated".to_owned()));
    }
    let (nonce, ciphertext) = sealed.split_at(NONCE_LEN);
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key));
    cipher
        .decrypt(Nonce::from_slice(nonce), ciphertext)
        .map_err(|_| SecretsError::Crypto("opening failed (wrong key or tampered)".to_owned()))
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::keyring::MemoryKeyring;

    fn store() -> SecretStore {
        SecretStore::open(Arc::new(MemoryKeyring::new())).unwrap()
    }

    #[test]
    fn seal_open_round_trip() {
        let s = store();
        let secret = b"wireguard-private-key-material";
        let sealed = s.seal(secret).unwrap();
        assert_ne!(
            sealed.as_slice(),
            secret,
            "sealed bytes are not the plaintext"
        );
        assert_eq!(s.unseal(&sealed).unwrap(), secret);
    }

    #[test]
    fn each_seal_uses_a_fresh_nonce() {
        let s = store();
        let a = s.seal(b"same").unwrap();
        let b = s.seal(b"same").unwrap();
        assert_ne!(a, b, "identical plaintext must seal to different bytes");
    }

    #[test]
    fn tampering_is_detected() {
        let s = store();
        let mut sealed = s.seal(b"important").unwrap();
        let last = sealed.len() - 1;
        sealed[last] ^= 0x01; // flip a bit in the tag
        assert!(s.unseal(&sealed).is_err(), "a modified blob must not open");
    }

    #[test]
    fn a_different_master_key_cannot_open() {
        let a = store();
        let b = store(); // independent keyring → different master key
        let sealed = a.seal(b"secret").unwrap();
        assert!(b.unseal(&sealed).is_err());
    }

    #[test]
    fn master_key_persists_across_reopen() {
        let keyring = Arc::new(MemoryKeyring::new());
        let sealed = SecretStore::open(keyring.clone())
            .unwrap()
            .seal(b"x")
            .unwrap();
        // A fresh store over the same keyring loads the same master key.
        let reopened = SecretStore::open(keyring).unwrap();
        assert_eq!(reopened.unseal(&sealed).unwrap(), b"x");
    }

    #[test]
    fn rotation_reseals_and_invalidates_the_old_key() {
        let mut s = store();
        let old_sealed = s.seal(b"rotate-me").unwrap();

        let prior = s.rotate().unwrap();
        // The new key cannot open the old blob...
        assert!(s.unseal(&old_sealed).is_err());
        // ...but the prior-key handle can, so we can re-seal it.
        let plaintext = prior.unseal(&old_sealed).unwrap();
        let new_sealed = s.seal(&plaintext).unwrap();
        assert_eq!(s.unseal(&new_sealed).unwrap(), b"rotate-me");
    }
}
