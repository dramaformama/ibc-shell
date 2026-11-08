//! Result types of the Docker resource commands (ADR-016).

use serde::{Deserialize, Serialize};

use crate::docker::DockerAccess;

/// One image.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, specta::Type)]
pub struct DockerImage {
    /// Short id (12 hex chars).
    pub id: String,
    /// `repository:tag` (`<none>` for untagged).
    pub name: String,
    /// Size in bytes, if the CLI's figure could be read.
    pub size_bytes: Option<u64>,
    /// Human age, e.g. `3 weeks ago`.
    pub created: String,
    /// Containers (running or not) created from it; 0 = unused.
    pub containers: u32,
}

/// Result of [`docker_images`].
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, specta::Type)]
pub struct DockerImages {
    /// Access state; the list is empty unless `Ok`.
    pub access: DockerAccess,
    /// Images, used first, then biggest first.
    pub images: Vec<DockerImage>,
}

/// One volume.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, specta::Type)]
pub struct DockerVolume {
    /// Name.
    pub name: String,
    /// Driver (`local`).
    pub driver: String,
    /// Disk use in bytes when the daemon reports it.
    pub size_bytes: Option<u64>,
    /// Containers that mount it.
    pub containers: u32,
}

/// Result of [`docker_volumes`].
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, specta::Type)]
pub struct DockerVolumes {
    /// Access state; the list is empty unless `Ok`.
    pub access: DockerAccess,
    /// Volumes, used first, then by name.
    pub volumes: Vec<DockerVolume>,
}

/// One network.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, specta::Type)]
pub struct DockerNetwork {
    /// Short id (12 hex chars).
    pub id: String,
    /// Name.
    pub name: String,
    /// Driver (`bridge`, `host`, `overlay`, …).
    pub driver: String,
    /// `local` or `swarm`.
    pub scope: String,
    /// Subnets, space separated; empty when none.
    pub subnet: String,
    /// Attached containers.
    pub containers: u32,
}

/// Result of [`docker_networks`].
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, specta::Type)]
pub struct DockerNetworks {
    /// Access state; the list is empty unless `Ok`.
    pub access: DockerAccess,
    /// Networks, with containers first, then by name.
    pub networks: Vec<DockerNetwork>,
}

/// One container of a compose project.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, specta::Type)]
pub struct ComposeService {
    /// Service name from the compose file.
    pub service: String,
    /// Container name.
    pub container: String,
    /// `running`, `exited`, …
    pub state: String,
    /// Human status.
    pub status: String,
}

/// A compose project, found by the labels compose puts on its containers.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, specta::Type)]
pub struct ComposeProject {
    /// Project name.
    pub name: String,
    /// Directory of the compose file on the server, if labelled.
    pub dir: Option<String>,
    /// Running containers.
    pub running: u32,
    /// All containers.
    pub total: u32,
    /// Containers by service.
    pub services: Vec<ComposeService>,
}

/// Result of [`docker_compose`].
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, specta::Type)]
pub struct DockerCompose {
    /// Access state; the list is empty unless `Ok`.
    pub access: DockerAccess,
    /// Projects, busiest first.
    pub projects: Vec<ComposeProject>,
}

/// One daemon event.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, specta::Type)]
pub struct DockerEvent {
    /// Unix millis (server clock).
    pub time: i64,
    /// `container`, `image`, `volume` or `network`.
    pub kind: String,
    /// `start`, `die`, `oom`, `pull`, … (no arguments).
    pub action: String,
    /// Name of the thing, if the event carries one.
    pub name: String,
}

/// Result of [`docker_events`].
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, specta::Type)]
pub struct DockerEvents {
    /// Access state; the list is empty unless `Ok`.
    pub access: DockerAccess,
    /// Newest first, ≤ 200.
    pub events: Vec<DockerEvent>,
}
