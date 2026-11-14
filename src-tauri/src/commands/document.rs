//! Open a document explicitly selected in either file pane (ADR-018).

use crate::api::dto::Endpoint;
use crate::api::{ApiError, ApiResult};
use crate::state::AppState;
use ibc_shell_vault::VaultState;
use tauri::State;

#[cfg(desktop)]
use super::sftp::local_leaf;
use super::sftp::sftp_of;
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

#[cfg(desktop)]
fn local_document(state: &AppState, path: &str) -> ApiResult<std::path::PathBuf> {
    let path = local_leaf(state, path)?
        .canonicalize()
        .map_err(|_| ApiError::NotFound)?;
    if !state.roots.allows(&path, false) {
        return Err(ApiError::Validation("local path not allowed".into()));
    }
    if !path.is_file() {
        return Err(ApiError::Unsupported);
    }
    Ok(path)
}

const PREVIEW_BLOCK: u32 = 5 << 20;

/// Plain-text preview and whether the file exceeded the read limit.
#[derive(serde::Serialize, specta::Type)]
pub struct FilePreview {
    text: String,
    truncated: bool,
}

#[cfg(desktop)]
fn read_local_preview(file: &Path, tail: bool, limit: u32) -> ApiResult<(Vec<u8>, bool)> {
    use std::io::{Read, Seek, SeekFrom};
    let failed = |_| ApiError::Internal("could not read document".into());
    let mut file = std::fs::File::open(file).map_err(failed)?;
    let metadata = file.metadata().map_err(failed)?;
    if !metadata.is_file() {
        return Err(ApiError::Unsupported);
    }
    let size = metadata.len();
    let start = if tail {
        size.saturating_sub(u64::from(limit))
    } else {
        0
    };
    file.seek(SeekFrom::Start(start)).map_err(failed)?;
    let mut bytes = Vec::new();
    file.take(u64::from(limit) + u64::from(!tail))
        .read_to_end(&mut bytes)
        .map_err(failed)?;
    let truncated = start != 0 || bytes.len() > limit as usize;
    bytes.truncate(limit as usize);
    Ok((bytes, truncated))
}

fn decode_preview(bytes: &[u8], truncated: bool, tail: bool) -> ApiResult<String> {
    let mut bytes = bytes;
    if tail && truncated {
        // The byte limit can land inside the first code point of the suffix.
        let skip = bytes
            .iter()
            .take(3)
            .take_while(|b| **b & 0xc0 == 0x80)
            .count();
        bytes = &bytes[skip..];
    } else {
        bytes = bytes.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(bytes);
    }
    let text = match std::str::from_utf8(bytes) {
        Ok(text) => text,
        Err(err) if truncated && !tail && err.error_len().is_none() => {
            std::str::from_utf8(&bytes[..err.valid_up_to()]).map_err(|_| ApiError::Unsupported)?
        }
        Err(_) => return Err(ApiError::Unsupported),
    };
    if text
        .chars()
        .any(|c| c.is_control() && !matches!(c, '\n' | '\r' | '\t'))
    {
        return Err(ApiError::Unsupported);
    }
    Ok(text.to_owned())
}

/// Reads the beginning or end of a selected text file without creating a copy.
#[tauri::command]
#[specta::specta]
pub async fn file_preview(
    state: State<'_, AppState>,
    endpoint: Endpoint,
    path: String,
    tail: bool,
    blocks: u8,
) -> ApiResult<FilePreview> {
    if state.vault.state()? != VaultState::Unlocked {
        return Err(ApiError::VaultLocked);
    }
    let limit = u32::from(blocks.clamp(1, 4)) * PREVIEW_BLOCK;
    let (bytes, truncated) = match endpoint {
        Endpoint::Local => {
            #[cfg(not(desktop))]
            return Err(ApiError::Unsupported);
            #[cfg(desktop)]
            {
                let file = local_document(&state, &path)?;
                tauri::async_runtime::spawn_blocking(move || read_local_preview(&file, tail, limit))
                    .await
                    .map_err(|_| ApiError::Internal("could not read document".into()))??
            }
        }
        Endpoint::Remote { sftp_id } => {
            let sftp = sftp_of(&state, sftp_id)?;
            let result = sftp.read_document(&path, limit, tail).await?;
            if sftp_of(&state, sftp_id).is_err() {
                return Err(ApiError::Cancelled);
            }
            result
        }
    };
    if state.vault.state()? != VaultState::Unlocked {
        return Err(ApiError::Cancelled);
    }
    Ok(FilePreview {
        text: decode_preview(&bytes, truncated, tail)?,
        truncated,
    })
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
            let file = local_document(&state, &path)?;
            if !document_format(&file) {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_utf8_at_preview_boundaries() {
        assert_eq!(
            decode_preview(b"\xef\xbb\xbftext", false, false).unwrap(),
            "text"
        );
        assert_eq!(decode_preview(b"text\xd0", true, false).unwrap(), "text");
        assert_eq!(decode_preview(b"\xb0text", true, true).unwrap(), "text");
        assert!(decode_preview(b"text\xd0", false, false).is_err());
        assert!(decode_preview(b"text\0", false, false).is_err());
    }
}
