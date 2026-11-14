//! Bounded download of a single document for local viewing.

use std::sync::atomic::AtomicU64;
use std::sync::Arc;

use futures_util::stream::{self, StreamExt, TryStreamExt};
use russh_sftp::protocol::OpenFlags;
use tokio::io::BufWriter;
use tokio_util::sync::CancellationToken;

use crate::copy::{copy, read_range, Plan};
use crate::remote::{kind_of, map_err};
use crate::side::{Sink, Source};
use crate::{EntryKind, Sftp, SftpError};

impl Sftp {
    /// Reads a bounded segment of a regular file into memory.
    pub async fn read_document(
        &self,
        path: &str,
        limit: u32,
        tail: bool,
    ) -> Result<(Vec<u8>, bool), SftpError> {
        let (handle, size) = self.open_document(path).await?;
        let len = u32::try_from(size).unwrap_or(limit).min(limit);
        let start = if tail { size - u64::from(len) } else { 0 };
        let chunk = self.rw_limits().0.unwrap_or(32_768).clamp(1, 32_768);
        let offsets = (0..len).step_by(chunk as usize);
        let result = stream::iter(offsets)
            .map(|offset| {
                let count = (len - offset).min(chunk);
                let handle = &handle;
                async move {
                    let bytes = read_range(self, handle, start + u64::from(offset), count).await?;
                    if bytes.len() != count as usize {
                        return Err(SftpError::Protocol("source changed during preview".into()));
                    }
                    Ok(bytes)
                }
            })
            .buffered(16)
            .try_fold(
                Vec::with_capacity(len as usize),
                |mut output, bytes| async move {
                    output.extend_from_slice(&bytes);
                    Ok(output)
                },
            )
            .await;
        let _ = self.raw().close(handle).await;
        result.map(|bytes| (bytes, size > u64::from(len)))
    }

    async fn open_document(&self, path: &str) -> Result<(String, u64), SftpError> {
        let path = self.realpath(path).await?;
        if self.stat(&path).await?.kind != EntryKind::File {
            return Err(SftpError::Unimplemented);
        }

        let handle = self.open_file(&path, OpenFlags::READ).await?;
        let size = self
            .raw()
            .fstat(handle.clone())
            .await
            .map_err(|err| map_err(&err))
            .and_then(|a| {
                a.attrs
                    .size
                    .filter(|_| kind_of(a.attrs.permissions.unwrap_or(0)) == EntryKind::File)
                    .ok_or(SftpError::Unimplemented)
            });
        match size {
            Ok(size) => Ok((handle, size)),
            Err(err) => {
                let _ = self.raw().close(handle).await;
                Err(err)
            }
        }
    }

    /// Copies one regular file into a caller-owned local file, up to `max_bytes`.
    /// Returns Unsupported for other entry types or an unknown or oversized length.
    pub async fn copy_document(
        self: &Arc<Self>,
        path: &str,
        dst: std::fs::File,
        max_bytes: u64,
    ) -> Result<(), SftpError> {
        let (handle, size) = self.open_document(path).await?;
        let source = Source::Remote(self.clone(), handle);
        if size > max_bytes {
            source.close().await;
            return Err(SftpError::Unimplemented);
        }
        let sink = Sink::Local(BufWriter::new(tokio::fs::File::from_std(dst)));
        let chunk = self.rw_limits().0.unwrap_or(32_768).clamp(1, 32_768);
        copy(
            source,
            sink,
            Plan {
                chunk,
                depth: 16,
                start: 0,
                size,
            },
            &CancellationToken::new(),
            &AtomicU64::new(0),
        )
        .await
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::path::Path;
    use std::process::Stdio;

    #[tokio::test]
    async fn copies_only_bounded_regular_documents() {
        let Some(bin) = [
            "/usr/libexec/sftp-server",
            "/usr/lib/openssh/sftp-server",
            "/usr/libexec/openssh/sftp-server",
            "/usr/lib/ssh/sftp-server",
        ]
        .into_iter()
        .find(|bin| Path::new(bin).is_file()) else {
            eprintln!("sftp-server not found, skipping");
            return;
        };
        let dir = tempfile::tempdir().unwrap();
        let mut server = tokio::process::Command::new(bin)
            .current_dir(dir.path())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .unwrap();
        let stream = tokio::io::join(server.stdout.take().unwrap(), server.stdin.take().unwrap());
        let sftp = Arc::new(Sftp::from_stream(stream).await.unwrap());
        let original = "Файл с пробелами.txt";
        let body = "Содержимое\n".repeat(8000);
        std::fs::write(dir.path().join(original), &body).unwrap();
        std::os::unix::fs::symlink(original, dir.path().join("link.txt")).unwrap();
        std::fs::write(dir.path().join("empty.txt"), []).unwrap();

        let dest = dir.path().join("copy.txt");
        let file = || std::fs::File::create(&dest).unwrap();
        sftp.copy_document("link.txt", file(), body.len() as u64)
            .await
            .unwrap();
        assert_eq!(std::fs::read_to_string(&dest).unwrap(), body);
        sftp.copy_document("empty.txt", file(), 0).await.unwrap();
        assert_eq!(std::fs::metadata(&dest).unwrap().len(), 0);
        assert!(matches!(
            sftp.copy_document(original, file(), 8).await,
            Err(SftpError::Unimplemented)
        ));
        assert!(matches!(
            sftp.copy_document(".", file(), 8).await,
            Err(SftpError::Unimplemented)
        ));
        assert_eq!(
            sftp.read_document("link.txt", 39_000, false).await.unwrap(),
            (body.as_bytes()[..39_000].to_vec(), true)
        );
        assert_eq!(
            sftp.read_document(original, 8, true).await.unwrap(),
            (body.as_bytes()[body.len() - 8..].to_vec(), true)
        );
        assert_eq!(
            sftp.read_document("empty.txt", 8, true).await.unwrap(),
            (Vec::new(), false)
        );
        sftp.close().await;
    }
}
