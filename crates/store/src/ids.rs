//! Strongly typed UUID v7 identifiers. Stored as 16-byte BLOBs, exposed to the UI as
//! lowercase hyphenated strings.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

macro_rules! define_id {
    ($($(#[$doc:meta])* $name:ident),+ $(,)?) => {$(
        $(#[$doc])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, specta::Type)]
        #[serde(transparent)]
        pub struct $name(pub Uuid);

        impl $name {
            /// Generates a fresh time-ordered (v7) id.
            #[must_use]
            pub fn new_v7() -> Self {
                Self(Uuid::now_v7())
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                self.0.hyphenated().fmt(f)
            }
        }
    )+};
}

define_id!(
    /// Folder id.
    FolderId,
    /// Host id.
    HostId,
    /// Identity (credential metadata) id.
    IdentityId,
    /// Tag id.
    TagId,
    /// Snippet id.
    SnippetId,
    /// Port-forward rule id.
    ForwardId,
    /// Known-host entry id.
    KnownHostId,
    /// Encrypted secret row id (`vault_secret.id`).
    SecretId,
);
