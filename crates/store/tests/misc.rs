//! Identities, secrets, tags, known hosts, snapshot, file handling.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::many_single_char_names,
    clippy::assert_is_empty,
    clippy::assertions_on_result_states,
    missing_docs
)]

use ibc_store::{
    ForwardInput, ForwardKind, HostInput, IdentityKind, IdentityMetaInput, IdentityMetaPatch,
    KnownHostInput, SecretId, SecretRow, SnippetInput, Store, StoreError, TagInput,
};

fn secret(s: &Store) -> SecretId {
    let id = SecretId::new_v7();
    s.secret_put(SecretRow {
        id,
        nonce: vec![1; 24],
        ciphertext: vec![2; 40],
        created_at: 1,
    })
    .unwrap();
    id
}

fn identity(s: &Store, pw: SecretId) -> ibc_store::Identity {
    s.identity_create(IdentityMetaInput {
        label: "me".into(),
        username: Some("alice".into()),
        kind: IdentityKind::Password,
        key_algo: None,
        key_fingerprint: None,
        public_key: None,
        password_secret_id: Some(pw),
        key_secret_id: None,
        passphrase_secret_id: None,
    })
    .unwrap()
}

#[test]
fn identity_secret_lifecycle() {
    let s = Store::open_in_memory().unwrap();
    let (p1, p2) = (secret(&s), secret(&s));
    let i = identity(&s, p1);
    // Replacing the secret deletes the old row in the same transaction.
    s.identity_update(
        i.id,
        IdentityMetaPatch {
            password_secret_id: Some(Some(p2)),
            ..Default::default()
        },
    )
    .unwrap();
    assert!(s.secret_get(p1).unwrap().is_none());
    assert!(s.secret_get(p2).unwrap().is_some());
    // Username falls back to the identity's, reported as Default.
    let h = s
        .host_create(HostInput {
            label: "h".into(),
            hostname: "h.x".into(),
            identity_id: Some(i.id),
            ..Default::default()
        })
        .unwrap();
    let r = s.host_resolve(h.id).unwrap();
    assert_eq!(r.username.value.as_deref(), Some("alice"));
    assert_eq!(r.username.source, ibc_store::Source::Default);
    assert_eq!(r.identity_id.source, ibc_store::Source::Host);
    // Delete removes secrets; hosts keep working with the reference cleared.
    s.identity_delete(i.id).unwrap();
    assert!(s.secret_ids().unwrap().is_empty());
    assert_eq!(s.host_resolve(h.id).unwrap().identity_id.value, None);
    assert!(matches!(s.identity_get(i.id), Err(StoreError::NotFound)));
}

#[test]
fn tags_conflict_case_insensitive() {
    let s = Store::open_in_memory().unwrap();
    s.tag_create(TagInput {
        name: "Prod".into(),
        color: None,
    })
    .unwrap();
    assert!(matches!(
        s.tag_create(TagInput {
            name: "prod".into(),
            color: None
        }),
        Err(StoreError::Conflict(_))
    ));
}

#[test]
fn known_hosts_upsert_and_settings_meta() {
    let s = Store::open_in_memory().unwrap();
    let kh = |blob: u8| KnownHostInput {
        host_pattern: "[h.x]:2222".into(),
        key_type: "ssh-ed25519".into(),
        key_blob: vec![blob; 8],
        fingerprint: format!("SHA256:{blob}"),
    };
    s.known_host_upsert(kh(1)).unwrap();
    s.known_host_upsert(kh(2)).unwrap();
    let found = s.known_host_lookup("[h.x]:2222").unwrap();
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].key_blob, vec![2; 8]);
    assert_eq!(s.known_host_list().unwrap()[0].fingerprint, "SHA256:2");
    s.known_host_delete(found[0].id).unwrap();
    assert!(s.known_host_lookup("[h.x]:2222").unwrap().is_empty());

    assert!(s.setting_get("theme").unwrap().is_none());
    s.setting_set("theme", &serde_json::json!({"mode": "dark"}))
        .unwrap();
    s.setting_set("theme", &serde_json::json!({"mode": "light"}))
        .unwrap();
    assert_eq!(s.setting_get("theme").unwrap().unwrap()["mode"], "light");
    s.vault_meta_set("kdf", &[9, 9]).unwrap();
    assert_eq!(s.vault_meta_get("kdf").unwrap(), Some(vec![9, 9]));
    assert_eq!(s.vault_meta_get("none").unwrap(), None);
}

#[test]
fn snapshot_roundtrip_and_merge() {
    let a = Store::open_in_memory().unwrap();
    let f = a
        .folder_create(ibc_store::FolderInput {
            name: "f".into(),
            ..Default::default()
        })
        .unwrap();
    let t = a
        .tag_create(TagInput {
            name: "t".into(),
            color: None,
        })
        .unwrap();
    let h = a
        .host_create(HostInput {
            folder_id: Some(f.id),
            label: "Хост".into(),
            hostname: "h.x".into(),
            tag_ids: vec![t.id],
            ..Default::default()
        })
        .unwrap();
    a.forward_create(ForwardInput {
        host_id: h.id,
        kind: ForwardKind::Local,
        bind_addr: None,
        bind_port: 8080,
        dest_host: Some("localhost".into()),
        dest_port: Some(80),
        auto_start: false,
        label: None,
    })
    .unwrap();
    assert!(a
        .forward_create(ForwardInput {
            host_id: h.id,
            kind: ForwardKind::Remote,
            bind_addr: None,
            bind_port: 1,
            dest_host: None,
            dest_port: None,
            auto_start: false,
            label: None
        })
        .is_err());
    a.snippet_create(SnippetInput {
        folder_id: Some(f.id),
        name: "s".into(),
        body: "ls".into(),
    })
    .unwrap();
    a.snippet_create(SnippetInput {
        folder_id: None,
        name: "g".into(),
        body: "pwd".into(),
    })
    .unwrap();
    let snap = a.export_snapshot().unwrap();

    let b = Store::open_in_memory().unwrap();
    let n = b.import_snapshot(snap.clone()).unwrap();
    assert_eq!(
        (n.folders, n.hosts, n.tags, n.snippets, n.forwards),
        (1, 1, 1, 2, 1)
    );
    assert_eq!(b.tree_list().unwrap(), a.tree_list().unwrap());
    assert_eq!(b.host_search("хост", 5).unwrap(), vec![h.id]);
    assert_eq!(b.snippet_list(None).unwrap().len(), 1);
    assert_eq!(b.snippet_list(Some(f.id)).unwrap().len(), 2);
    // Merge again: everything exists, nothing is written.
    let n2 = b.import_snapshot(snap).unwrap();
    assert_eq!(
        (n2.folders, n2.hosts, n2.tags, n2.snippets, n2.forwards),
        (0, 0, 0, 0, 0)
    );
}

#[test]
fn secret_row_debug_hides_material() {
    let row = SecretRow {
        id: SecretId::new_v7(),
        nonce: vec![7; 24],
        ciphertext: vec![8; 4],
        created_at: 0,
    };
    let dbg = format!("{row:?}");
    assert!(!dbg.contains("ciphertext") && !dbg.contains("nonce"));
}

#[cfg(unix)]
#[test]
fn file_store_persists_with_0600() {
    use std::os::unix::fs::PermissionsExt;
    let dir = std::env::temp_dir().join(format!("ibc-store-{}", SecretId::new_v7()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("ibc.db");
    {
        let s = Store::open(&path).unwrap();
        s.host_create(HostInput {
            label: "h".into(),
            hostname: "h.x".into(),
            ..Default::default()
        })
        .unwrap();
        assert_eq!(s.tree_list().unwrap().hosts.len(), 1); // via a read connection
    }
    let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode, 0o600);
    let s = Store::open(&path).unwrap();
    assert_eq!(s.tree_list().unwrap().hosts.len(), 1);
    assert_eq!(s.host_search("h", 5).unwrap().len(), 1);
    drop(s);
    std::fs::remove_dir_all(&dir).unwrap();
}
