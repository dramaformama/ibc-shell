//! Tree, ordering, inheritance, search and perf tests.
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
    FolderDefaultsPatch, FolderDeleteMode, FolderId, FolderInput, FolderPatch, HostId, HostInput,
    HostPatch, NodeRef, Source, Store, StoreError, TagInput,
};

fn store() -> Store {
    Store::open_in_memory().unwrap()
}

fn folder(s: &Store, parent: Option<FolderId>, name: &str) -> FolderId {
    s.folder_create(FolderInput {
        parent_id: parent,
        name: name.into(),
        ..Default::default()
    })
    .unwrap()
    .id
}

fn host(s: &Store, f: Option<FolderId>, label: &str, hostname: &str) -> HostId {
    s.host_create(HostInput {
        folder_id: f,
        label: label.into(),
        hostname: hostname.into(),
        ..Default::default()
    })
    .unwrap()
    .id
}

#[test]
fn cycles_are_rejected() {
    let s = store();
    let a = folder(&s, None, "a");
    let b = folder(&s, Some(a), "b");
    let c = folder(&s, Some(b), "c");
    assert!(matches!(
        s.folder_move(a, Some(a), None),
        Err(StoreError::Cycle)
    ));
    assert!(matches!(
        s.folder_move(a, Some(c), None),
        Err(StoreError::Cycle)
    ));
    assert!(matches!(
        s.folder_move(b, Some(c), None),
        Err(StoreError::Cycle)
    ));
    // Legal move: c to root.
    s.folder_move(c, None, None).unwrap();
    let t = s.tree_list().unwrap();
    assert_eq!(
        t.folders.iter().find(|f| f.id == c).unwrap().parent_id,
        None
    );
}

#[test]
fn depth_is_capped_at_eight() {
    let s = store();
    let mut parent = None;
    let mut ids = Vec::new();
    for i in 0..8 {
        let id = folder(&s, parent, &format!("l{i}"));
        ids.push(id);
        parent = Some(id);
    }
    let r = s.folder_create(FolderInput {
        parent_id: parent,
        name: "l8".into(),
        ..Default::default()
    });
    assert!(matches!(r, Err(StoreError::TooDeep)));
    // Moving a 2-level subtree under the depth-7 folder would reach depth 9.
    let sub = folder(&s, None, "sub");
    folder(&s, Some(sub), "subchild");
    assert!(matches!(
        s.folder_move(sub, Some(ids[6]), None),
        Err(StoreError::TooDeep)
    ));
    // A single folder fits at depth 8.
    let leaf = folder(&s, None, "leaf");
    s.folder_move(leaf, Some(ids[6]), None).unwrap();
}

#[test]
fn delete_modes() {
    let s = store();
    let root = folder(&s, None, "root");
    let mid = folder(&s, Some(root), "mid");
    let inner = folder(&s, Some(mid), "inner");
    let h1 = host(&s, Some(mid), "h1", "h1.example");
    let h2 = host(&s, Some(inner), "h2", "h2.example");
    let h0 = host(&s, Some(root), "h0", "h0.example");

    s.folder_delete(mid, FolderDeleteMode::MoveToParent)
        .unwrap();
    let t = s.tree_list().unwrap();
    assert!(t.folders.iter().all(|f| f.id != mid));
    assert_eq!(
        t.folders.iter().find(|f| f.id == inner).unwrap().parent_id,
        Some(root)
    );
    assert_eq!(
        t.hosts.iter().find(|h| h.id == h1).unwrap().folder_id,
        Some(root)
    );
    assert_eq!(
        t.hosts.iter().find(|h| h.id == h2).unwrap().folder_id,
        Some(inner)
    );
    assert!(s.host_search("h1", 10).unwrap().contains(&h1));

    s.folder_delete(root, FolderDeleteMode::DeleteContents)
        .unwrap();
    let t = s.tree_list().unwrap();
    assert!(t.folders.is_empty() && t.hosts.is_empty());
    assert!(s.host_search("h0", 10).unwrap().is_empty());
    assert!(matches!(s.host_resolve(h0), Err(StoreError::NotFound)));
}

#[test]
fn inheritance_over_three_levels() {
    let s = store();
    let a = folder(&s, None, "a");
    let b = folder(&s, Some(a), "b");
    let c = folder(&s, Some(b), "c");
    s.folder_update(
        a,
        FolderPatch {
            defaults: FolderDefaultsPatch {
                username: Some(Some("root".into())),
                port: Some(Some(2200)),
                startup_cmd: Some(Some("tmux".into())),
                ..Default::default()
            },
            ..Default::default()
        },
    )
    .unwrap();
    s.folder_update(
        b,
        FolderPatch {
            defaults: FolderDefaultsPatch {
                port: Some(Some(2222)),
                keepalive_s: Some(Some(10)),
                ..Default::default()
            },
            ..Default::default()
        },
    )
    .unwrap();
    let h = host(&s, Some(c), "h", "10.0.0.1");

    let r = s.host_resolve(h).unwrap();
    assert_eq!((r.port.value, r.port.source), (2222, Source::Folder(b)));
    assert_eq!(r.username.value.as_deref(), Some("root"));
    assert_eq!(r.username.source, Source::Folder(a));
    assert_eq!(
        (r.keepalive_s.value, r.keepalive_s.source),
        (10, Source::Folder(b))
    );
    assert_eq!(r.startup_cmd.source, Source::Folder(a));
    assert_eq!(r.term_theme.source, Source::Default);

    // Host override wins; clearing (Some(None)) returns to inheritance.
    s.host_update(
        h,
        HostPatch {
            port: Some(Some(22022)),
            ..Default::default()
        },
    )
    .unwrap();
    let r = s.host_resolve(h).unwrap();
    assert_eq!((r.port.value, r.port.source), (22022, Source::Host));
    s.host_update(
        h,
        HostPatch {
            port: Some(None),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(s.host_resolve(h).unwrap().port.source, Source::Folder(b));

    // Defaults at the root.
    let lone = host(&s, None, "lone", "lone.example");
    let r = s.host_resolve(lone).unwrap();
    assert_eq!((r.port.value, r.port.source), (22, Source::Default));
    assert_eq!(
        (r.keepalive_s.value, r.keepalive_s.source),
        (30, Source::Default)
    );
    assert_eq!(r.username.value, None);
}

#[test]
fn flag_inheritance_and_normalization() {
    let s = store();
    let a = folder(&s, None, "a");
    let b = folder(&s, Some(a), "b");
    s.folder_update(
        a,
        FolderPatch {
            country_code: Some(Some("DE".into())),
            ..Default::default()
        },
    )
    .unwrap();
    let h = host(&s, Some(b), "h", "h.example");
    let r = s.host_resolve(h).unwrap();
    assert_eq!(r.country_code.value.as_deref(), Some("de"));
    assert_eq!(r.country_code.source, Source::Folder(a));
    s.folder_update(
        b,
        FolderPatch {
            country_code: Some(Some("fr".into())),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(
        s.host_resolve(h).unwrap().country_code.source,
        Source::Folder(b)
    );
    s.host_update(
        h,
        HostPatch {
            country_code: Some(Some("JP".into())),
            ..Default::default()
        },
    )
    .unwrap();
    let r = s.host_resolve(h).unwrap();
    assert_eq!(
        (r.country_code.value.as_deref(), r.country_code.source),
        (Some("jp"), Source::Host)
    );
    let bad = s.host_update(
        h,
        HostPatch {
            country_code: Some(Some("1!".into())),
            ..Default::default()
        },
    );
    assert!(matches!(bad, Err(StoreError::Validation(_))));
}

#[test]
fn patch_tri_state() {
    let s = store();
    let h = s
        .host_create(HostInput {
            label: "h".into(),
            hostname: "h.example".into(),
            username: Some("u".into()),
            notes: Some("n".into()),
            ..Default::default()
        })
        .unwrap();
    // Untouched.
    let h2 = s
        .host_update(
            h.id,
            HostPatch {
                label: Some("h2".into()),
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(
        (h2.username.as_deref(), h2.notes.as_deref()),
        (Some("u"), Some("n"))
    );
    // Cleared.
    let h3 = s
        .host_update(
            h.id,
            HostPatch {
                username: Some(None),
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!((h3.username, h3.notes.as_deref()), (None, Some("n")));
    // Set.
    let h4 = s
        .host_update(
            h.id,
            HostPatch {
                username: Some(Some("x".into())),
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(h4.username.as_deref(), Some("x"));
}

#[test]
fn jump_chain_resolution_and_limits() {
    let s = store();
    let [a, b, c, d, e] =
        ["a", "b", "c", "d", "e"].map(|n| host(&s, None, n, &format!("{n}.example")));
    let jump = |h: HostId, j: HostId| {
        s.host_update(
            h,
            HostPatch {
                jump_host_id: Some(Some(j)),
                ..Default::default()
            },
        )
    };
    jump(a, b).unwrap();
    jump(b, c).unwrap();
    assert_eq!(s.host_resolve(a).unwrap().jump_chain, vec![c, b]);
    // Cycle and self-reference rejected.
    assert!(matches!(jump(c, a), Err(StoreError::JumpCycle)));
    assert!(matches!(jump(c, c), Err(StoreError::JumpCycle)));
    // Exactly 3 hops OK, 4 rejected.
    jump(c, d).unwrap();
    assert_eq!(s.host_resolve(a).unwrap().jump_chain, vec![d, c, b]);
    assert!(matches!(jump(d, e), Err(StoreError::JumpCycle)));
    // Inherited through a folder default.
    let f = folder(&s, None, "f");
    s.folder_update(
        f,
        FolderPatch {
            defaults: FolderDefaultsPatch {
                jump_host_id: Some(Some(e)),
                ..Default::default()
            },
            ..Default::default()
        },
    )
    .unwrap();
    let inside = host(&s, Some(f), "inside", "in.example");
    assert_eq!(s.host_resolve(inside).unwrap().jump_chain, vec![e]);
}

#[test]
fn ordering_move_and_duplicate() {
    let s = store();
    let a = host(&s, None, "a", "a.x");
    let b = host(&s, None, "b", "b.x");
    let c = host(&s, None, "c", "c.x");
    let order = |s: &Store| {
        s.tree_list()
            .unwrap()
            .hosts
            .iter()
            .map(|h| h.label.clone())
            .collect::<Vec<_>>()
    };
    assert_eq!(order(&s), ["a", "b", "c"]);
    s.host_move(c, None, None).unwrap();
    assert_eq!(order(&s), ["c", "a", "b"]);
    s.host_move(c, None, Some(NodeRef::Host(a))).unwrap();
    assert_eq!(order(&s), ["a", "c", "b"]);
    let d = s.host_duplicate(a).unwrap();
    assert_eq!(d.label, "a copy");
    assert_eq!(order(&s), ["a", "a copy", "c", "b"]);
    let _ = b;
    // Anchor outside the target folder is rejected.
    let f = folder(&s, None, "f");
    assert!(matches!(
        s.host_move(a, Some(f), Some(NodeRef::Host(b))),
        Err(StoreError::Validation(_))
    ));
}

#[test]
fn search_cyrillic_latin_tags_and_sync() {
    let s = store();
    let h1 = host(&s, None, "Сервер Москва", "msk.example.com");
    let h2 = host(&s, None, "Frankfurt DB", "fra-db.example.com");
    let h3 = host(&s, None, "other", "10.1.2.3");
    assert_eq!(s.host_search("москва", 10).unwrap(), vec![h1]);
    assert_eq!(s.host_search("СЕРВ", 10).unwrap(), vec![h1]);
    assert_eq!(s.host_search("frank", 10).unwrap(), vec![h2]);
    assert_eq!(s.host_search("fra-db", 10).unwrap(), vec![h2]);
    assert_eq!(s.host_search("10.1.2", 10).unwrap(), vec![h3]);
    assert!(s.host_search("   ", 10).unwrap().is_empty());
    assert!(s.host_search("\"*()", 10).unwrap().is_empty());

    // Rename updates the index.
    s.host_update(
        h1,
        HostPatch {
            label: Some("Хост Питер".into()),
            ..Default::default()
        },
    )
    .unwrap();
    assert!(s.host_search("москва", 10).unwrap().is_empty());
    assert_eq!(s.host_search("питер", 10).unwrap(), vec![h1]);

    // Tags and notes are searchable and follow tag renames / deletes.
    let t = s
        .tag_create(TagInput {
            name: "production".into(),
            color: None,
        })
        .unwrap();
    s.host_update(
        h2,
        HostPatch {
            tag_ids: Some(vec![t.id]),
            notes: Some(Some("резервная копия".into())),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(s.host_search("production", 10).unwrap(), vec![h2]);
    assert_eq!(s.host_search("копия", 10).unwrap(), vec![h2]);
    s.tag_update(
        t.id,
        ibc_store::TagPatch {
            name: Some("staging".into()),
            ..Default::default()
        },
    )
    .unwrap();
    assert!(s.host_search("production", 10).unwrap().is_empty());
    assert_eq!(s.host_search("staging", 10).unwrap(), vec![h2]);
    s.tag_delete(t.id).unwrap();
    assert!(s.host_search("staging", 10).unwrap().is_empty());
    assert_eq!(s.host_search("frank", 10).unwrap(), vec![h2]);

    s.host_delete(&[h2]).unwrap();
    assert!(s.host_search("frank", 10).unwrap().is_empty());
    assert_eq!(s.host_search("example", 1).unwrap(), vec![h1]);
}

#[test]
#[ignore = "perf bench: run with --release -- --ignored"]
fn bench_tree_list_5000_hosts() {
    let s = store();
    let mut folders = Vec::new();
    for i in 0..200 {
        let parent = if i < 20 { None } else { Some(folders[i % 20]) };
        folders.push(folder(&s, parent, &format!("f{i}")));
    }
    for i in 0..5000 {
        host(
            &s,
            Some(folders[i % 200]),
            &format!("host {i}"),
            &format!("h{i}.example.com"),
        );
    }
    s.tree_list().unwrap(); // warm up
    let mut best = u128::MAX;
    for _ in 0..10 {
        let t = std::time::Instant::now();
        let tree = s.tree_list().unwrap();
        best = best.min(t.elapsed().as_micros());
        assert_eq!(tree.hosts.len(), 5000);
    }
    println!("tree_list best: {best} us");
    assert!(best <= 15_000, "tree_list took {best} us");
}
