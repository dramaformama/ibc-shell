//! Open a document explicitly selected in either file pane (ADR-018).

use crate::api::dto::Endpoint;
use crate::api::{ApiError, ApiResult};
use crate::state::AppState;
use ibc_shell_vault::VaultState;
use tauri::State;

#[cfg(desktop)]
use super::sftp::{local_leaf, sftp_of};
#[cfg(desktop)]
use std::path::Path;
#[cfg(desktop)]
use std::sync::Mutex;
#[cfg(desktop)]
use tauri::Manager;

/// Keeps remote copies available to viewers until the application exits.
#[cfg(desktop)]
#[derive(Default)]
pub(crate) struct OpenDocuments(Mutex<Vec<tempfile::TempDir>>);

#[cfg(desktop)]
impl OpenDocuments {
    pub(crate) fn clear(&self) {
        self.0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clear();
    }
}

#[cfg(desktop)]
fn document_format(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| {
            matches!(
                ext.to_ascii_lowercase().as_str(),
                "txt"
                    | "md"
                    | "log"
                    | "conf"
                    | "ini"
                    | "json"
                    | "yaml"
                    | "yml"
                    | "toml"
                    | "csv"
                    | "tsv"
                    | "pdf"
                    | "rtf"
                    | "docx"
                    | "xlsx"
                    | "pptx"
                    | "odt"
                    | "ods"
                    | "png"
                    | "jpg"
                    | "jpeg"
                    | "gif"
                    | "webp"
                    | "bmp"
                    | "tiff"
                    | "heic"
                    | "mp3"
                    | "wav"
                    | "flac"
                    | "ogg"
                    | "m4a"
                    | "mp4"
                    | "mov"
                    | "mkv"
                    | "webm"
            )
        })
}

#[cfg(desktop)]
fn open_document(path: &Path) -> ApiResult<()> {
    let failed = || ApiError::Internal("could not open document".into());
    #[cfg(windows)]
    {
        open::that_detached(dunce::simplified(path)).map_err(|_| failed())
    }
    #[cfg(not(windows))]
    {
        use std::process::{Command, Stdio};
        let launcher = if cfg!(target_os = "macos") {
            "/usr/bin/open"
        } else {
            "xdg-open"
        };
        let status = Command::new(launcher)
            .arg(path)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map_err(|_| failed())?;
        if status.success() {
            Ok(())
        } else {
            Err(failed())
        }
    }
}

/// Opens a regular local document or a temporary remote copy in the default app.
#[tauri::command]
#[specta::specta]
pub async fn file_open(
    state: State<'_, AppState>,
    endpoint: Endpoint,
    path: String,
    app: tauri::AppHandle,
) -> ApiResult<()> {
    if state.vault.state()? != VaultState::Unlocked {
        return Err(ApiError::VaultLocked);
    }
    #[cfg(not(desktop))]
    {
        let _ = (app, endpoint, path);
        Err(ApiError::Unsupported)
    }
    #[cfg(desktop)]
    match endpoint {
        Endpoint::Local => {
            let file = local_leaf(&state, &path)?
                .canonicalize()
                .map_err(|_| ApiError::NotFound)?;
            if !state.roots.allows(&file, false) {
                return Err(ApiError::Validation("local path not allowed".into()));
            }
            if !file.is_file() || !document_format(&file) {
                return Err(ApiError::Unsupported);
            }
            tauri::async_runtime::spawn_blocking(move || open_document(&file))
                .await
                .map_err(|_| ApiError::Internal("could not open document".into()))?
        }
        Endpoint::Remote { sftp_id } => {
            let sftp = sftp_of(&state, sftp_id)?;
            let remote = sftp.realpath(&path).await?;
            let name = ibc_shell_sftp::sanitize::escape_name(
                remote.rsplit('/').next().unwrap_or_default(),
            )?;
            if !document_format(Path::new(&name)) {
                return Err(ApiError::Unsupported);
            }

            let cache_error = |_| ApiError::Internal("could not create document copy".into());
            let mut temp = tempfile::Builder::new();
            temp.prefix("ibc-document-");
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                temp.permissions(std::fs::Permissions::from_mode(0o700));
            }
            let dir = temp.tempdir().map_err(cache_error)?;
            let file = dir.path().join(name);
            let dest = std::fs::File::create_new(&file).map_err(cache_error)?;
            sftp.copy_document(&remote, dest, 256 << 20).await?;
            if state.vault.state()? != VaultState::Unlocked || sftp_of(&state, sftp_id).is_err() {
                return Err(ApiError::Cancelled);
            }
            tauri::async_runtime::spawn_blocking(move || open_document(&file))
                .await
                .map_err(|_| ApiError::Internal("could not open document".into()))??;
            app.state::<OpenDocuments>()
                .0
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .push(dir);
            Ok(())
        }
    }
}
