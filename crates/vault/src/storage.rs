//! Storage seam: the vault only needs raw meta/secret rows and snapshots. `Store` implements
//! it; tests use an in-memory implementation.

use ibc_shell_store::{ImportCounts, SecretId, SecretRow, Snapshot, Store, StoreError};

/// Persistence used by [`crate::Vault`].
pub trait VaultStorage: Send + Sync {
    /// Reads a `vault_meta` value.
    fn meta_get(&self, k: &str) -> Result<Option<Vec<u8>>, StoreError>;
    /// Writes a `vault_meta` value.
    fn meta_set(&self, k: &str, v: &[u8]) -> Result<(), StoreError>;
    /// Inserts a secret row.
    fn secret_put(&self, row: SecretRow) -> Result<(), StoreError>;
    /// Reads a secret row.
    fn secret_get(&self, id: SecretId) -> Result<Option<SecretRow>, StoreError>;
    /// Deletes a secret row.
    fn secret_delete(&self, id: SecretId) -> Result<(), StoreError>;
    /// All secret ids.
    fn secret_ids(&self) -> Result<Vec<SecretId>, StoreError>;
    /// Snapshot of non-secret data.
    fn export_snapshot(&self) -> Result<Snapshot, StoreError>;
    /// Merges a snapshot.
    fn import_snapshot(&self, s: Snapshot) -> Result<ImportCounts, StoreError>;
}

impl VaultStorage for Store {
    fn meta_get(&self, k: &str) -> Result<Option<Vec<u8>>, StoreError> {
        self.vault_meta_get(k)
    }
    fn meta_set(&self, k: &str, v: &[u8]) -> Result<(), StoreError> {
        self.vault_meta_set(k, v)
    }
    fn secret_put(&self, row: SecretRow) -> Result<(), StoreError> {
        Store::secret_put(self, row)
    }
    fn secret_get(&self, id: SecretId) -> Result<Option<SecretRow>, StoreError> {
        Store::secret_get(self, id)
    }
    fn secret_delete(&self, id: SecretId) -> Result<(), StoreError> {
        Store::secret_delete(self, id)
    }
    fn secret_ids(&self) -> Result<Vec<SecretId>, StoreError> {
        Store::secret_ids(self)
    }
    fn export_snapshot(&self) -> Result<Snapshot, StoreError> {
        Store::export_snapshot(self)
    }
    fn import_snapshot(&self, s: Snapshot) -> Result<ImportCounts, StoreError> {
        Store::import_snapshot(self, s)
    }
}
