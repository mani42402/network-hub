//! The desired-state store (§5.3, AD-4).
//!
//! All configuration NetworkHub applies — hotspot enabled with profile X, this
//! firewall rule set, this VPN connected — is stored here as **desired state**.
//! An API mutation writes desired state and returns; a reconciler then converges
//! the OS toward it. Each document carries a **revision** that increments on
//! every write, which gives reconcilers a cheap "has this changed since I last
//! looked?" check and gives the API optimistic-concurrency.
//!
//! For WP-M0.4 this is in-memory. Durability — surviving restart, which the
//! desired-state model requires — is added in WP-M0.5 by backing it with
//! `config.db`; the reconciler framework here neither knows nor cares which.

use std::collections::HashMap;
use std::sync::Mutex;

use serde::Serialize;
use serde::de::DeserializeOwned;
use tokio::sync::broadcast;

/// Identifies one desired-state document, e.g. `hotspot.profile.default` or
/// `firewall.ruleset`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct StateKey(pub String);

impl StateKey {
    /// Builds a key from any string-like value.
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }
}

impl std::fmt::Display for StateKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// A stored document: its serialized contents and its revision.
#[derive(Debug, Clone)]
struct StoredDoc {
    revision: u64,
    /// JSON so the store is type-agnostic; typed access serializes on the way in
    /// and deserializes on the way out.
    data: String,
}

/// A typed view of a document read from the store.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Versioned<T> {
    /// The document's revision at read time. A reconciler compares this against
    /// the revision it last converged to decide whether re-work is needed.
    pub revision: u64,
    /// The deserialized value.
    pub value: T,
}

/// The in-memory desired-state store.
#[derive(Debug)]
pub struct StateStore {
    docs: Mutex<HashMap<StateKey, StoredDoc>>,
    /// Broadcasts the key of every changed document, so reconcilers wake only for
    /// the state they care about.
    changes: broadcast::Sender<StateKey>,
}

impl Default for StateStore {
    fn default() -> Self {
        Self::new()
    }
}

impl StateStore {
    /// A new, empty store.
    #[must_use]
    pub fn new() -> Self {
        let (changes, _rx) = broadcast::channel(256);
        Self {
            docs: Mutex::new(HashMap::new()),
            changes,
        }
    }

    /// Writes a document, bumping its revision, and notifies watchers. Returns
    /// the new revision.
    ///
    /// # Errors
    /// Returns [`StoreError::Serialize`] if the value cannot be serialized.
    ///
    /// # Panics
    /// Panics only if the internal lock is poisoned (a prior panic while held).
    pub fn put<T: Serialize>(&self, key: &StateKey, value: &T) -> Result<u64, StoreError> {
        let data =
            serde_json::to_string(value).map_err(|e| StoreError::Serialize(e.to_string()))?;
        let revision = {
            let mut docs = self.docs.lock().expect("state store lock");
            let entry = docs.entry(key.clone()).or_insert(StoredDoc {
                revision: 0,
                data: String::new(),
            });
            entry.revision += 1;
            entry.data = data;
            entry.revision
        };
        // A send error only means no watchers; harmless.
        let _ = self.changes.send(key.clone());
        Ok(revision)
    }

    /// Reads a document, deserialized, or `None` if the key is absent.
    ///
    /// # Errors
    /// Returns [`StoreError::Deserialize`] if the stored bytes do not match `T`.
    ///
    /// # Panics
    /// Panics only if the internal lock is poisoned.
    pub fn get<T: DeserializeOwned>(
        &self,
        key: &StateKey,
    ) -> Result<Option<Versioned<T>>, StoreError> {
        let doc = {
            let docs = self.docs.lock().expect("state store lock");
            docs.get(key).cloned()
        };
        match doc {
            None => Ok(None),
            Some(doc) => {
                let value = serde_json::from_str(&doc.data)
                    .map_err(|e| StoreError::Deserialize(e.to_string()))?;
                Ok(Some(Versioned {
                    revision: doc.revision,
                    value,
                }))
            }
        }
    }

    /// The current revision of a document, or `None` if absent. Cheaper than a
    /// full read for a reconciler's "did it change?" check.
    ///
    /// # Panics
    /// Panics only if the internal lock is poisoned.
    #[must_use]
    pub fn revision(&self, key: &StateKey) -> Option<u64> {
        self.docs
            .lock()
            .expect("state store lock")
            .get(key)
            .map(|d| d.revision)
    }

    /// Removes a document, notifying watchers. Returns whether it existed.
    ///
    /// # Panics
    /// Panics only if the internal lock is poisoned.
    pub fn delete(&self, key: &StateKey) -> bool {
        let existed = self
            .docs
            .lock()
            .expect("state store lock")
            .remove(key)
            .is_some();
        if existed {
            let _ = self.changes.send(key.clone());
        }
        existed
    }

    /// Subscribes to change notifications. Each message is the key of a document
    /// that was written or deleted.
    #[must_use]
    pub fn watch(&self) -> broadcast::Receiver<StateKey> {
        self.changes.subscribe()
    }
}

/// A state-store error.
#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    /// The value could not be serialized for storage.
    #[error("serialize failed: {0}")]
    Serialize(String),
    /// The stored bytes did not deserialize into the requested type.
    #[error("deserialize failed: {0}")]
    Deserialize(String),
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn put_then_get_round_trips() {
        let store = StateStore::new();
        let key = StateKey::new("hotspot.enabled");
        assert_eq!(store.put(&key, &true).unwrap(), 1);
        let got: Versioned<bool> = store.get(&key).unwrap().unwrap();
        assert_eq!(got.revision, 1);
        assert!(got.value);
    }

    #[test]
    fn revision_increments_on_each_write() {
        let store = StateStore::new();
        let key = StateKey::new("k");
        assert_eq!(store.put(&key, &1u32).unwrap(), 1);
        assert_eq!(store.put(&key, &2u32).unwrap(), 2);
        assert_eq!(store.revision(&key), Some(2));
    }

    #[test]
    fn missing_key_is_none() {
        let store = StateStore::new();
        assert!(
            store
                .get::<bool>(&StateKey::new("absent"))
                .unwrap()
                .is_none()
        );
        assert_eq!(store.revision(&StateKey::new("absent")), None);
    }

    #[test]
    fn type_mismatch_is_an_error_not_a_panic() {
        let store = StateStore::new();
        let key = StateKey::new("k");
        store.put(&key, &"a string").unwrap();
        assert!(matches!(
            store.get::<u32>(&key),
            Err(StoreError::Deserialize(_))
        ));
    }

    #[tokio::test]
    async fn writes_notify_watchers() {
        let store = StateStore::new();
        let mut watch = store.watch();
        let key = StateKey::new("k");
        store.put(&key, &1u32).unwrap();
        assert_eq!(watch.recv().await.unwrap(), key);
    }

    #[test]
    fn delete_removes_and_reports() {
        let store = StateStore::new();
        let key = StateKey::new("k");
        store.put(&key, &1u32).unwrap();
        assert!(store.delete(&key));
        assert!(!store.delete(&key));
        assert!(store.get::<u32>(&key).unwrap().is_none());
    }
}
