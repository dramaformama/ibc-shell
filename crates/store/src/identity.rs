//! Identities (credential metadata), raw secret rows, vault meta and JSON settings.

use rusqlite::{params, Connection, OptionalExtension, Row};

use crate::catalog::{Identity, IdentityKind, IdentityMetaInput, IdentityMetaPatch, SecretRow};
use crate::db::{now_ms, query_all, Store};
use crate::ids::{IdentityId, SecretId};
use crate::{validate, Result, StoreError};

pub(crate) const IDENTITY_COLS: &str =
    "id,label,username,kind,key_algo,key_fingerprint,public_key,\
    password_secret_id,key_secret_id,passphrase_secret_id,created_at,updated_at";

fn kind_str(k: IdentityKind) -> &'static str {
    match k {
        IdentityKind::Password => "password",
        IdentityKind::Key => "key",
        IdentityKind::KeyPassword => "key_password",
        IdentityKind::Agent => "agent",
    }
}

fn parse_kind(s: &str) -> Option<IdentityKind> {
    Some(match s {
        "password" => IdentityKind::Password,
        "key" => IdentityKind::Key,
        "key_password" => IdentityKind::KeyPassword,
        "agent" => IdentityKind::Agent,
        _ => return None,
    })
}

pub(crate) fn identity_from_row(r: &Row<'_>) -> rusqlite::Result<Identity> {
    let kind: String = r.get(3)?;
    Ok(Identity {
        id: r.get(0)?,
        label: r.get(1)?,
        username: r.get(2)?,
        kind: parse_kind(&kind).ok_or(rusqlite::Error::InvalidColumnType(
            3,
            "kind".into(),
            rusqlite::types::Type::Text,
        ))?,
        key_algo: r.get(4)?,
        key_fingerprint: r.get(5)?,
        public_key: r.get(6)?,
        password_secret_id: r.get(7)?,
        key_secret_id: r.get(8)?,
        passphrase_secret_id: r.get(9)?,
        created_at: r.get(10)?,
        updated_at: r.get(11)?,
    })
}

pub(crate) fn identity_get(conn: &Connection, id: IdentityId) -> Result<Identity> {
    let sql = format!("SELECT {IDENTITY_COLS} FROM identity WHERE id=?1");
    Ok(conn
        .prepare_cached(&sql)?
        .query_row([id], identity_from_row)?)
}

/// Inserts an identity row (`verb` = `INSERT` or `INSERT OR IGNORE`).
pub(crate) fn insert_identity(conn: &Connection, i: &Identity, verb: &str) -> Result<bool> {
    let sql = format!(
        "{verb} INTO identity({IDENTITY_COLS}) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)"
    );
    let n = conn.execute(
        &sql,
        params![
            i.id,
            i.label,
            i.username,
            kind_str(i.kind),
            i.key_algo,
            i.key_fingerprint,
            i.public_key,
            i.password_secret_id,
            i.key_secret_id,
            i.passphrase_secret_id,
            i.created_at,
            i.updated_at
        ],
    )?;
    Ok(n > 0)
}

fn secret_ids_of(i: &Identity) -> [Option<SecretId>; 3] {
    [
        i.password_secret_id,
        i.key_secret_id,
        i.passphrase_secret_id,
    ]
}

impl Store {
    /// All identities, by label.
    pub fn identity_list(&self) -> Result<Vec<Identity>> {
        self.read(|c| {
            let sql =
                format!("SELECT {IDENTITY_COLS} FROM identity ORDER BY label COLLATE NOCASE, id");
            query_all(c, &sql, [], identity_from_row)
        })
    }

    /// One identity.
    pub fn identity_get(&self, id: IdentityId) -> Result<Identity> {
        self.read(|c| identity_get(c, id))
    }

    /// Creates identity metadata (secrets already stored by the vault).
    pub fn identity_create(&self, input: IdentityMetaInput) -> Result<Identity> {
        validate::name("label", &input.label)?;
        self.write(|tx| {
            let now = now_ms();
            let i = Identity {
                id: IdentityId::new_v7(),
                label: input.label,
                username: input.username,
                kind: input.kind,
                key_algo: input.key_algo,
                key_fingerprint: input.key_fingerprint,
                public_key: input.public_key,
                password_secret_id: input.password_secret_id,
                key_secret_id: input.key_secret_id,
                passphrase_secret_id: input.passphrase_secret_id,
                created_at: now,
                updated_at: now,
            };
            insert_identity(tx, &i, "INSERT")?;
            Ok(i)
        })
    }

    /// Updates; replaced or cleared secret rows are deleted in the same transaction.
    pub fn identity_update(&self, id: IdentityId, patch: IdentityMetaPatch) -> Result<Identity> {
        if let Some(l) = &patch.label {
            validate::name("label", l)?;
        }
        self.write(|tx| {
            let old = identity_get(tx, id)?;
            let mut i = old.clone();
            if let Some(v) = patch.label {
                i.label = v;
            }
            if let Some(v) = patch.username {
                i.username = v;
            }
            if let Some(v) = patch.kind {
                i.kind = v;
            }
            if let Some(v) = patch.key_algo {
                i.key_algo = v;
            }
            if let Some(v) = patch.key_fingerprint {
                i.key_fingerprint = v;
            }
            if let Some(v) = patch.public_key {
                i.public_key = v;
            }
            if let Some(v) = patch.password_secret_id {
                i.password_secret_id = v;
            }
            if let Some(v) = patch.key_secret_id {
                i.key_secret_id = v;
            }
            if let Some(v) = patch.passphrase_secret_id {
                i.passphrase_secret_id = v;
            }
            i.updated_at = now_ms();
            tx.execute(
                "UPDATE identity SET label=?2, username=?3, kind=?4, key_algo=?5,
                 key_fingerprint=?6, public_key=?7, password_secret_id=?8, key_secret_id=?9,
                 passphrase_secret_id=?10, updated_at=?11 WHERE id=?1",
                params![
                    id,
                    i.label,
                    i.username,
                    kind_str(i.kind),
                    i.key_algo,
                    i.key_fingerprint,
                    i.public_key,
                    i.password_secret_id,
                    i.key_secret_id,
                    i.passphrase_secret_id,
                    i.updated_at
                ],
            )?;
            let kept = secret_ids_of(&i);
            for s in secret_ids_of(&old).into_iter().flatten() {
                if !kept.contains(&Some(s)) {
                    tx.execute("DELETE FROM vault_secret WHERE id=?1", [s])?;
                }
            }
            Ok(i)
        })
    }

    /// Deletes identity and its `vault_secret` rows in one transaction (invariant 5).
    pub fn identity_delete(&self, id: IdentityId) -> Result<()> {
        self.write(|tx| {
            let i = identity_get(tx, id)?;
            tx.execute("DELETE FROM identity WHERE id=?1", [id])?;
            for s in secret_ids_of(&i).into_iter().flatten() {
                tx.execute("DELETE FROM vault_secret WHERE id=?1", [s])?;
            }
            Ok(())
        })
    }

    // ---- settings ----

    /// Reads a setting.
    pub fn setting_get(&self, key: &str) -> Result<Option<serde_json::Value>> {
        let raw: Option<String> = self.read(|c| {
            Ok(
                c.query_row("SELECT v FROM setting WHERE k=?1", [key], |r| r.get(0))
                    .optional()?,
            )
        })?;
        raw.map(|s| {
            serde_json::from_str(&s).map_err(|e| StoreError::Db(format!("setting json: {e}")))
        })
        .transpose()
    }

    /// Writes a setting.
    pub fn setting_set(&self, key: &str, value: &serde_json::Value) -> Result<()> {
        let json = value.to_string();
        self.write(|tx| {
            tx.execute(
                "INSERT INTO setting(k, v) VALUES(?1, ?2) ON CONFLICT(k) DO UPDATE SET v=excluded.v",
                params![key, json],
            )?;
            Ok(())
        })
    }

    // ---- raw secret rows + vault meta ----

    /// Inserts (or replaces) a secret row.
    pub fn secret_put(&self, row: SecretRow) -> Result<()> {
        self.write(|tx| {
            tx.execute(
                "INSERT INTO vault_secret(id, nonce, ciphertext, created_at) VALUES(?1,?2,?3,?4)
                 ON CONFLICT(id) DO UPDATE SET nonce=excluded.nonce, ciphertext=excluded.ciphertext",
                params![row.id, row.nonce, row.ciphertext, row.created_at],
            )?;
            Ok(())
        })
    }

    /// Reads a secret row.
    pub fn secret_get(&self, id: SecretId) -> Result<Option<SecretRow>> {
        self.read(|c| {
            Ok(c.query_row(
                "SELECT id, nonce, ciphertext, created_at FROM vault_secret WHERE id=?1",
                [id],
                |r| {
                    Ok(SecretRow {
                        id: r.get(0)?,
                        nonce: r.get(1)?,
                        ciphertext: r.get(2)?,
                        created_at: r.get(3)?,
                    })
                },
            )
            .optional()?)
        })
    }

    /// Deletes a secret row.
    pub fn secret_delete(&self, id: SecretId) -> Result<()> {
        self.write(|tx| {
            tx.execute("DELETE FROM vault_secret WHERE id=?1", [id])?;
            Ok(())
        })
    }

    /// All secret ids (backup export).
    pub fn secret_ids(&self) -> Result<Vec<SecretId>> {
        self.read(|c| {
            query_all(
                c,
                "SELECT id FROM vault_secret ORDER BY created_at, id",
                [],
                |r| r.get(0),
            )
        })
    }

    /// Reads a `vault_meta` value.
    pub fn vault_meta_get(&self, k: &str) -> Result<Option<Vec<u8>>> {
        self.read(|c| {
            Ok(
                c.query_row("SELECT v FROM vault_meta WHERE k=?1", [k], |r| r.get(0))
                    .optional()?,
            )
        })
    }

    /// Writes a `vault_meta` value.
    pub fn vault_meta_set(&self, k: &str, v: &[u8]) -> Result<()> {
        self.write(|tx| {
            tx.execute(
                "INSERT INTO vault_meta(k, v) VALUES(?1, ?2) ON CONFLICT(k) DO UPDATE SET v=excluded.v",
                params![k, v],
            )?;
            Ok(())
        })
    }
}
