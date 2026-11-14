//! OpenSSH client config (`~/.ssh/config`) reader for importing hosts. Pure: file access for
//! `Include` goes through a caller-supplied loader. Semantics follow `ssh_config(5)` where it
//! matters for an import: first value wins, `Host` patterns with `*`, `?` and `!`, `Include`
//! expanded in place, `Match` blocks skipped.

use crate::ForwardSpec;

/// Deepest `Include` nesting followed.
const MAX_INCLUDE_DEPTH: usize = 8;
/// Lines read across the file and every include.
const MAX_LINES: usize = 100_000;

/// One concrete `Host` alias with everything that applies to it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ConfigEntry {
    /// The alias (`Host` name without wildcards).
    pub alias: String,
    /// `HostName` (`%h` expanded), the alias when absent.
    pub hostname: String,
    /// `Port`.
    pub port: Option<u16>,
    /// `User`.
    pub user: Option<String>,
    /// `IdentityFile` values in order, `~` / `%d` NOT expanded (the caller knows the home).
    pub identity_files: Vec<String>,
    /// `ProxyJump` (comma-separated hops), `none` dropped.
    pub proxy_jump: Option<String>,
    /// `LocalForward` / `RemoteForward` / `DynamicForward` lines that parsed.
    pub forwards: Vec<ForwardSpec>,
    /// Keywords that matter for connecting but are not imported (e.g. `ProxyCommand`).
    pub unsupported: Vec<String>,
}

struct Block {
    /// `None` = lines before the first `Host` (apply to every host).
    patterns: Option<Vec<String>>,
    /// A `Match` block: never applied.
    skip: bool,
    settings: Vec<(String, Vec<String>)>,
}

/// Splits a line into keyword + arguments (`Key value`, `Key=value`, double quotes).
fn tokens(line: &str) -> Option<(String, Vec<String>)> {
    let line = line.trim();
    if line.is_empty() || line.starts_with('#') {
        return None;
    }
    let split = line
        .find(|c: char| c.is_whitespace() || c == '=')
        .unwrap_or(line.len());
    let key = line[..split].to_ascii_lowercase();
    let mut rest = line[split..].trim_start();
    if let Some(r) = rest.strip_prefix('=') {
        rest = r.trim_start();
    }
    let mut args = Vec::new();
    let mut cur = String::new();
    let (mut quoted, mut any) = (false, false);
    for c in rest.chars() {
        match c {
            '"' => {
                quoted = !quoted;
                any = true;
            }
            c if c.is_whitespace() && !quoted => {
                if any {
                    args.push(std::mem::take(&mut cur));
                    any = false;
                }
            }
            c => {
                cur.push(c);
                any = true;
            }
        }
    }
    if any {
        args.push(cur);
    }
    Some((key, args))
}

/// Reads lines, expanding `Include` in place.
fn flatten(text: &str, load: &dyn Fn(&str) -> Vec<String>, depth: usize, out: &mut Vec<String>) {
    for line in text.lines() {
        if out.len() >= MAX_LINES {
            return;
        }
        match tokens(line) {
            Some((k, args)) if k == "include" => {
                if depth >= MAX_INCLUDE_DEPTH {
                    continue;
                }
                for pattern in &args {
                    for inc in load(pattern) {
                        flatten(&inc, load, depth + 1, out);
                    }
                }
            }
            Some(_) => out.push(line.to_owned()),
            None => {}
        }
    }
}

/// `*` and `?` glob, case-insensitive.
fn glob(pattern: &str, text: &str) -> bool {
    let p: Vec<char> = pattern.to_lowercase().chars().collect();
    let t: Vec<char> = text.to_lowercase().chars().collect();
    let (mut pi, mut ti, mut star, mut mark) = (0, 0, None, 0);
    while ti < t.len() {
        if pi < p.len() && (p[pi] == '?' || p[pi] == t[ti]) {
            pi += 1;
            ti += 1;
        } else if pi < p.len() && p[pi] == '*' {
            star = Some(pi);
            mark = ti;
            pi += 1;
        } else if let Some(s) = star {
            pi = s + 1;
            mark += 1;
            ti = mark;
        } else {
            return false;
        }
    }
    p[pi..].iter().all(|&c| c == '*')
}

fn matches(patterns: &[String], alias: &str) -> bool {
    let mut hit = false;
    for p in patterns {
        if let Some(neg) = p.strip_prefix('!') {
            if glob(neg, alias) {
                return false;
            }
        } else if glob(p, alias) {
            hit = true;
        }
    }
    hit
}

fn is_pattern(p: &str) -> bool {
    p.contains(['*', '?', '!'])
}

/// `[bind:]port` → (bind address, port). `*` / empty bind = all interfaces.
fn bind_of(s: &str) -> Option<(String, u16)> {
    let (addr, port) = match s.rsplit_once(':') {
        Some((a, p)) => (a.trim_matches(['[', ']']).to_owned(), p),
        None => ("127.0.0.1".to_owned(), s),
    };
    let addr = match addr.as_str() {
        "" | "*" => "0.0.0.0".to_owned(),
        "localhost" => "127.0.0.1".to_owned(),
        _ => addr,
    };
    Some((addr, port.parse().ok()?))
}

/// `host:port` (IPv6 in brackets) → (host, port).
fn dest_of(s: &str) -> Option<(String, u16)> {
    let (h, p) = s.rsplit_once(':')?;
    let h = h.trim_matches(['[', ']']);
    (!h.is_empty()).then(|| Some((h.to_owned(), p.parse().ok()?)))?
}

fn forward_of(key: &str, args: &[String]) -> Option<ForwardSpec> {
    let (bind_addr, bind_port) = bind_of(args.first()?)?;
    if key == "dynamicforward" {
        return Some(ForwardSpec::Dynamic {
            bind_addr,
            bind_port,
        });
    }
    let (dest_host, dest_port) = dest_of(args.get(1)?)?;
    Some(if key == "localforward" {
        ForwardSpec::Local {
            bind_addr,
            bind_port,
            dest_host,
            dest_port,
        }
    } else {
        ForwardSpec::Remote {
            bind_addr,
            bind_port,
            dest_host,
            dest_port,
        }
    })
}

fn apply(e: &mut ConfigEntry, seen: &mut Vec<String>, key: &str, args: &[String]) {
    let Some(first) = args.first() else { return };
    let once = !matches!(
        key,
        "identityfile" | "localforward" | "remoteforward" | "dynamicforward"
    );
    if once {
        if seen.iter().any(|k| k == key) {
            return;
        }
        seen.push(key.to_owned());
    }
    match key {
        "hostname" => e.hostname = first.replace("%h", &e.alias).replace("%%", "%"),
        "port" => e.port = first.parse().ok(),
        "user" => e.user = Some(first.clone()),
        "identityfile" if !first.eq_ignore_ascii_case("none") => {
            e.identity_files.push(first.clone());
        }
        "proxyjump" if !first.eq_ignore_ascii_case("none") => e.proxy_jump = Some(first.clone()),
        "localforward" | "remoteforward" | "dynamicforward" => {
            e.forwards.extend(forward_of(key, args));
        }
        "proxycommand" if !first.eq_ignore_ascii_case("none") => {
            e.unsupported.push("ProxyCommand".into());
        }
        _ => {}
    }
}

/// Parses `text`; `load(pattern)` returns the contents of the files an `Include` names.
#[must_use]
pub fn parse_ssh_config(text: &str, load: &dyn Fn(&str) -> Vec<String>) -> Vec<ConfigEntry> {
    let mut lines = Vec::new();
    flatten(text, load, 0, &mut lines);
    let mut blocks = vec![Block {
        patterns: None,
        skip: false,
        settings: Vec::new(),
    }];
    let mut aliases: Vec<String> = Vec::new();
    for line in &lines {
        let Some((key, args)) = tokens(line) else {
            continue;
        };
        match key.as_str() {
            "host" => {
                for a in args.iter().filter(|a| !is_pattern(a)) {
                    if !aliases.iter().any(|x| x.eq_ignore_ascii_case(a)) {
                        aliases.push(a.clone());
                    }
                }
                blocks.push(Block {
                    patterns: Some(args),
                    skip: false,
                    settings: Vec::new(),
                });
            }
            "match" => blocks.push(Block {
                patterns: None,
                skip: true,
                settings: Vec::new(),
            }),
            _ => {
                if let Some(b) = blocks.last_mut() {
                    b.settings.push((key, args));
                }
            }
        }
    }
    aliases
        .into_iter()
        .map(|alias| {
            let mut e = ConfigEntry {
                hostname: alias.clone(),
                alias,
                ..ConfigEntry::default()
            };
            let mut seen = Vec::new();
            for b in &blocks {
                let applies = !b.skip && b.patterns.as_ref().is_none_or(|p| matches(p, &e.alias));
                if applies {
                    for (k, a) in &b.settings {
                        apply(&mut e, &mut seen, k, a);
                    }
                }
            }
            e.unsupported.dedup();
            e
        })
        .collect()
}

#[cfg(test)]
#[path = "ssh_config_tests.rs"]
mod tests;
