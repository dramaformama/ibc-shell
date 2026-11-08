//! Docker resources over an existing connection (ADR-016): compose projects, images, volumes,
//! networks and recent events. Read-only and passive, like `docker_list`; only the `docker` CLI
//! already on the server is used, no port is opened and nothing stays running.

use std::collections::HashMap;
use std::time::Duration;

use crate::docker::{run, sh, size, DockerAccess};
use crate::docker_more_types::{
    ComposeProject, ComposeService, DockerCompose, DockerEvent, DockerEvents, DockerImage,
    DockerImages, DockerNetwork, DockerNetworks, DockerVolume, DockerVolumes,
};
use crate::stats_parse::{sanitize, sections};
use crate::{Connection, SshError};

const TIMEOUT: Duration = Duration::from_secs(15);
/// `docker system df -v` walks every volume, so it gets longer.
const VOLUMES_TIMEOUT: Duration = Duration::from_secs(25);
const MAX_OUT: usize = 512 * 1024;
/// Rows of every list.
const MAX_ROWS: usize = 150;
/// Events returned.
const MAX_EVENTS: usize = 200;

/// `<T>` stands for a tab: keeps the scripts readable and the format strings free of escapes.
fn script(body: &str) -> String {
    let head = "export LC_ALL=C\ncommand -v docker >/dev/null 2>&1 || { echo '@@IBC:MISSING@@'; exit 0; }\n";
    format!("{head}{body}echo '@@IBC:END@@'\n").replace("<T>", "\t")
}

const IMAGES: &str = r"echo '@@IBC:MAIN@@'
docker images --format '{{.ID}}<T>{{.Repository}}<T>{{.Tag}}<T>{{.Size}}<T>{{.CreatedSince}}' 2>&1 | head -300
echo '@@IBC:USE@@'
docker ps -a --format '{{.Image}}' 2>/dev/null | head -1000
";

const VOLUMES: &str = r"echo '@@IBC:MAIN@@'
docker volume ls --format '{{.Name}}<T>{{.Driver}}' 2>&1 | head -300
echo '@@IBC:USE@@'
docker ps -a --no-trunc --format '{{.Mounts}}' 2>/dev/null | head -1000
echo '@@IBC:DF@@'
docker system df -v 2>/dev/null | awk '/^Local Volumes/{f=1;next} /^Build cache/{f=0} f' | head -400
";

const NETWORKS: &str = r"echo '@@IBC:MAIN@@'
docker network ls --format '{{.ID}}<T>{{.Name}}<T>{{.Driver}}<T>{{.Scope}}' 2>&1 | head -100
echo '@@IBC:INSPECT@@'
docker network inspect $(docker network ls -q | head -100) --format '{{.Id}}<T>{{range .IPAM.Config}}{{.Subnet}} {{end}}<T>{{len .Containers}}' 2>/dev/null
";

const COMPOSE: &str = r#"echo '@@IBC:MAIN@@'
docker ps -a --format '{{.ID}}<T>{{.Names}}<T>{{.State}}<T>{{.Status}}<T>{{.Label "com.docker.compose.project"}}<T>{{.Label "com.docker.compose.service"}}<T>{{.Label "com.docker.compose.project.working_dir"}}' 2>&1 | head -1000
"#;

const EVENTS: &str = r#"echo '@@IBC:MAIN@@'
U=$(date +%s)
docker events --since "$(($U - $1))" --until "$U" --filter type=container --filter type=image --filter type=volume --filter type=network --format '{{.Time}}<T>{{.Type}}<T>{{.Action}}<T>{{index .Actor.Attributes "name"}}' 2>&1 | tail -n 1000
"#;

/// `Some(state)` when the CLI cannot be used at all.
fn blocked(sec: &HashMap<&str, String>) -> Option<DockerAccess> {
    if sec.contains_key("MISSING") {
        return Some(DockerAccess::Missing);
    }
    let main = sec.get("MAIN")?.to_ascii_lowercase();
    if main.contains("permission denied") {
        Some(DockerAccess::Denied)
    } else if main.contains("cannot connect to the docker daemon")
        || main.contains("is the docker daemon running")
    {
        Some(DockerAccess::Down)
    } else {
        None
    }
}

fn fields(line: &str) -> Vec<&str> {
    line.split('\t').collect()
}

fn lines<'a>(sec: &'a HashMap<&str, String>, name: &str) -> impl Iterator<Item = &'a str> {
    sec.get(name).map_or("", String::as_str).lines()
}

fn hex12(id: &str) -> String {
    id.trim_start_matches("sha256:").chars().take(12).collect()
}

pub(crate) fn parse_images(text: &str) -> DockerImages {
    let sec = sections(text);
    if let Some(access) = blocked(&sec) {
        return DockerImages {
            access,
            images: vec![],
        };
    }
    let used: Vec<String> = lines(&sec, "USE")
        .map(|l| l.trim().trim_start_matches("sha256:").to_string())
        .filter(|l| !l.is_empty())
        .collect();
    let mut images: Vec<DockerImage> = lines(&sec, "MAIN")
        .filter_map(|line| {
            let [id, repo, tag, sz, created] = fields(line)[..] else {
                return None;
            };
            let id = hex12(id.trim());
            if id.len() != 12 || !id.bytes().all(|b| b.is_ascii_hexdigit()) {
                return None;
            }
            let name = if tag == "<none>" || tag.is_empty() {
                repo.to_string()
            } else {
                format!("{repo}:{tag}")
            };
            let containers = used
                .iter()
                .filter(|u| {
                    u.starts_with(&id)
                        || **u == name
                        || (tag == "latest" && u.as_str() == repo)
                        || (id.starts_with(u.as_str()) && u.len() >= 8)
                })
                .count();
            Some(DockerImage {
                id,
                name: sanitize(&name),
                size_bytes: size(sz),
                created: sanitize(created),
                containers: u32::try_from(containers).unwrap_or(u32::MAX),
            })
        })
        .collect();
    images.sort_by(|a, b| {
        (a.containers == 0)
            .cmp(&(b.containers == 0))
            .then(b.size_bytes.cmp(&a.size_bytes))
            .then(a.name.cmp(&b.name))
    });
    images.truncate(MAX_ROWS);
    DockerImages {
        access: DockerAccess::Ok,
        images,
    }
}

pub(crate) fn parse_volumes(text: &str) -> DockerVolumes {
    let sec = sections(text);
    if let Some(access) = blocked(&sec) {
        return DockerVolumes {
            access,
            volumes: vec![],
        };
    }
    // `docker system df -v`: `NAME  LINKS  SIZE` under "Local Volumes space usage".
    let mut sizes: HashMap<&str, u64> = HashMap::new();
    for line in lines(&sec, "DF") {
        let f: Vec<&str> = line.split_whitespace().collect();
        if let [name, _links, sz] = f[..] {
            if let Some(b) = size(sz) {
                sizes.insert(name, b);
            }
        }
    }
    let mounts: Vec<&str> = lines(&sec, "USE").collect();
    let mut volumes: Vec<DockerVolume> = lines(&sec, "MAIN")
        .filter_map(|line| {
            let [name, driver] = fields(line)[..] else {
                return None;
            };
            let name = name.trim();
            if name.is_empty() {
                return None;
            }
            let containers = mounts
                .iter()
                .filter(|m| m.split(',').any(|part| part.trim() == name))
                .count();
            Some(DockerVolume {
                name: sanitize(name),
                driver: sanitize(driver),
                size_bytes: sizes.get(name).copied(),
                containers: u32::try_from(containers).unwrap_or(u32::MAX),
            })
        })
        .collect();
    volumes.sort_by(|a, b| {
        (a.containers == 0)
            .cmp(&(b.containers == 0))
            .then(a.name.cmp(&b.name))
    });
    volumes.truncate(MAX_ROWS);
    DockerVolumes {
        access: DockerAccess::Ok,
        volumes,
    }
}

pub(crate) fn parse_networks(text: &str) -> DockerNetworks {
    let sec = sections(text);
    if let Some(access) = blocked(&sec) {
        return DockerNetworks {
            access,
            networks: vec![],
        };
    }
    let mut detail: HashMap<String, (String, u32)> = HashMap::new();
    for line in lines(&sec, "INSPECT") {
        if let [id, subnet, n] = fields(line)[..] {
            detail.insert(
                hex12(id.trim()),
                (sanitize(subnet), n.trim().parse().unwrap_or(0)),
            );
        }
    }
    let mut networks: Vec<DockerNetwork> = lines(&sec, "MAIN")
        .filter_map(|line| {
            let [id, name, driver, scope] = fields(line)[..] else {
                return None;
            };
            let id = hex12(id.trim());
            if id.len() != 12 {
                return None;
            }
            let (subnet, containers) = detail.get(&id).cloned().unwrap_or_default();
            Some(DockerNetwork {
                id,
                name: sanitize(name),
                driver: sanitize(driver),
                scope: sanitize(scope),
                subnet,
                containers,
            })
        })
        .collect();
    networks.sort_by(|a, b| {
        (a.containers == 0)
            .cmp(&(b.containers == 0))
            .then(a.name.cmp(&b.name))
    });
    networks.truncate(MAX_ROWS);
    DockerNetworks {
        access: DockerAccess::Ok,
        networks,
    }
}

pub(crate) fn parse_compose(text: &str) -> DockerCompose {
    let sec = sections(text);
    if let Some(access) = blocked(&sec) {
        return DockerCompose {
            access,
            projects: vec![],
        };
    }
    let mut by_name: HashMap<String, ComposeProject> = HashMap::new();
    for line in lines(&sec, "MAIN") {
        let [_id, container, state, status, project, service, dir] = fields(line)[..] else {
            continue;
        };
        let project = sanitize(project);
        if project.is_empty() {
            continue;
        }
        let p = by_name
            .entry(project.clone())
            .or_insert_with(|| ComposeProject {
                name: project,
                dir: None,
                running: 0,
                total: 0,
                services: vec![],
            });
        let dir = sanitize(dir);
        if p.dir.is_none() && !dir.is_empty() {
            p.dir = Some(dir);
        }
        p.total += 1;
        if state.trim() == "running" {
            p.running += 1;
        }
        p.services.push(ComposeService {
            service: sanitize(service),
            container: sanitize(container),
            state: sanitize(state),
            status: sanitize(status),
        });
    }
    let mut projects: Vec<ComposeProject> = by_name.into_values().collect();
    for p in &mut projects {
        p.services.sort_by(|a, b| {
            a.service
                .cmp(&b.service)
                .then(a.container.cmp(&b.container))
        });
    }
    projects.sort_by(|a, b| b.running.cmp(&a.running).then(a.name.cmp(&b.name)));
    projects.truncate(MAX_ROWS);
    DockerCompose {
        access: DockerAccess::Ok,
        projects,
    }
}

/// Noisy actions that say nothing useful in a list (health probes, exec, tty).
fn noisy(action: &str) -> bool {
    ["exec_", "health_status", "attach", "resize", "top", "copy"]
        .iter()
        .any(|p| action.starts_with(p))
}

pub(crate) fn parse_events(text: &str) -> DockerEvents {
    let sec = sections(text);
    if let Some(access) = blocked(&sec) {
        return DockerEvents {
            access,
            events: vec![],
        };
    }
    let mut events: Vec<DockerEvent> = lines(&sec, "MAIN")
        .filter_map(|line| {
            let [time, kind, action, name] = fields(line)[..] else {
                return None;
            };
            let secs: i64 = time.trim().parse().ok()?;
            // Some actions carry the command line after a colon; it is dropped.
            let action = sanitize(action.split(':').next().unwrap_or(""));
            if action.is_empty() || noisy(&action) {
                return None;
            }
            Some(DockerEvent {
                time: secs.checked_mul(1000)?,
                kind: sanitize(kind),
                action,
                name: sanitize(name),
            })
        })
        .collect();
    events.reverse();
    events.truncate(MAX_EVENTS);
    DockerEvents {
        access: DockerAccess::Ok,
        events,
    }
}

/// Compose projects (by labels) with their containers.
///
/// Errors: [`SshError::Timeout`] after 15 s, network errors of the exec channel.
pub async fn docker_compose(conn: &Connection) -> Result<DockerCompose, SshError> {
    let text = run(conn, &sh(&script(COMPOSE), &[]), MAX_OUT, TIMEOUT).await?;
    Ok(parse_compose(&text))
}

/// Images with size, age and the number of containers using each.
///
/// Errors: as [`docker_compose`].
pub async fn docker_images(conn: &Connection) -> Result<DockerImages, SshError> {
    let text = run(conn, &sh(&script(IMAGES), &[]), MAX_OUT, TIMEOUT).await?;
    Ok(parse_images(&text))
}

/// Volumes with disk use and the number of containers mounting each (25 s).
///
/// Errors: as [`docker_compose`].
pub async fn docker_volumes(conn: &Connection) -> Result<DockerVolumes, SshError> {
    let text = run(conn, &sh(&script(VOLUMES), &[]), MAX_OUT, VOLUMES_TIMEOUT).await?;
    Ok(parse_volumes(&text))
}

/// Networks with subnets and attached containers.
///
/// Errors: as [`docker_compose`].
pub async fn docker_networks(conn: &Connection) -> Result<DockerNetworks, SshError> {
    let text = run(conn, &sh(&script(NETWORKS), &[]), MAX_OUT, TIMEOUT).await?;
    Ok(parse_networks(&text))
}

/// Daemon events of the last `window_s` seconds (10–86 400), newest first, ≤ 200. The CLI is
/// told `--until now`, so it returns by itself and nothing keeps running on the server.
///
/// Errors: as [`docker_compose`].
pub async fn docker_events(conn: &Connection, window_s: u32) -> Result<DockerEvents, SshError> {
    let w = window_s.clamp(10, 86_400).to_string();
    let text = run(conn, &sh(&script(EVENTS), &[&w]), MAX_OUT, TIMEOUT).await?;
    Ok(parse_events(&text))
}

#[cfg(test)]
#[path = "docker_more_tests.rs"]
mod tests;
