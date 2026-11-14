#![allow(clippy::unwrap_used, clippy::panic)]
use super::*;

fn parse(text: &str) -> Vec<ConfigEntry> {
    parse_ssh_config(text, &|_| Vec::new())
}

fn one<'a>(all: &'a [ConfigEntry], alias: &str) -> &'a ConfigEntry {
    all.iter().find(|e| e.alias == alias).unwrap()
}

#[test]
fn basic_hosts_and_first_value_wins() {
    let all = parse(
        "# comment\nHost web\n  HostName 10.0.0.5\n  User deploy\n  Port 2222\n  Port 22\n\
         Host db db2\n  HostName=db.internal\nHost *\n  User root\n  IdentityFile ~/.ssh/id_ed25519\n",
    );
    assert_eq!(all.len(), 3);
    let web = one(&all, "web");
    assert_eq!(web.hostname, "10.0.0.5");
    assert_eq!(
        web.user.as_deref(),
        Some("deploy"),
        "Host * must not override"
    );
    assert_eq!(web.port, Some(2222));
    assert_eq!(web.identity_files, ["~/.ssh/id_ed25519"]);
    let db2 = one(&all, "db2");
    assert_eq!(db2.hostname, "db.internal");
    assert_eq!(db2.user.as_deref(), Some("root"));
}

#[test]
fn wildcards_negation_and_global_section() {
    let all = parse(
        "User everyone\nHost prod-* !prod-legacy\n  Port 2200\nHost prod-api prod-legacy\n  HostName %h.example.com\n",
    );
    let api = one(&all, "prod-api");
    assert_eq!(api.port, Some(2200));
    assert_eq!(api.hostname, "prod-api.example.com");
    assert_eq!(api.user.as_deref(), Some("everyone"));
    assert_eq!(one(&all, "prod-legacy").port, None);
    assert!(all.iter().all(|e| !e.alias.contains('*')));
}

#[test]
fn match_blocks_are_skipped_and_quotes_work() {
    let all =
        parse("Host a\n  IdentityFile \"~/My Keys/id a\"\nMatch host a\n  User nobody\nHost b\n");
    let a = one(&all, "a");
    assert_eq!(a.identity_files, ["~/My Keys/id a"]);
    assert_eq!(a.user, None);
    assert_eq!(one(&all, "b").hostname, "b");
}

#[test]
fn include_is_expanded_in_place_and_bounded() {
    let load = |p: &str| match p {
        "conf.d/*" => vec!["Host inc\n  HostName included\n".to_owned()],
        "loop" => vec!["Include loop\nHost deep\n".to_owned()],
        _ => Vec::new(),
    };
    let all = parse_ssh_config("Include conf.d/*\nInclude loop\nHost top\n", &load);
    assert_eq!(one(&all, "inc").hostname, "included");
    assert!(
        all.iter().any(|e| e.alias == "deep"),
        "recursion stops at the depth cap"
    );
    assert!(all.iter().any(|e| e.alias == "top"));
}

#[test]
fn proxy_jump_forwards_and_unsupported() {
    let all = parse(
        "Host app\n  ProxyJump bastion\n  LocalForward 5432 db:5432\n  LocalForward *:8080 [::1]:80\n\
         RemoteForward 9000 localhost:3000\n  DynamicForward 1080\n  LocalForward broken\n\
         Host old\n  ProxyCommand ssh -W %h:%p gw\n  ProxyJump none\n",
    );
    let app = one(&all, "app");
    assert_eq!(app.proxy_jump.as_deref(), Some("bastion"));
    assert_eq!(
        app.forwards,
        [
            ForwardSpec::Local {
                bind_addr: "127.0.0.1".into(),
                bind_port: 5432,
                dest_host: "db".into(),
                dest_port: 5432
            },
            ForwardSpec::Local {
                bind_addr: "0.0.0.0".into(),
                bind_port: 8080,
                dest_host: "::1".into(),
                dest_port: 80
            },
            ForwardSpec::Remote {
                bind_addr: "127.0.0.1".into(),
                bind_port: 9000,
                dest_host: "localhost".into(),
                dest_port: 3000
            },
            ForwardSpec::Dynamic {
                bind_addr: "127.0.0.1".into(),
                bind_port: 1080
            },
        ]
    );
    let old = one(&all, "old");
    assert_eq!(old.unsupported, ["ProxyCommand"]);
    assert_eq!(old.proxy_jump, None);
}

#[test]
fn glob_edge_cases() {
    assert!(glob("*", ""));
    assert!(glob("a*b?c", "aXXbYc"));
    assert!(!glob("a*b?c", "aXXbc"));
    assert!(glob("WEB*", "web1"));
}
