//! Bounded download of a single document for local viewing.

use std::sync::atomic::AtomicU64;
use std::sync::Arc;

use russh_sftp::protocol::OpenFlags;
use tokio::io::BufWriter;
use tokio_util::sync::CancellationToken;

use crate::copy::{copy, Plan};
use crate::remote::{kind_of, map_err};
use crate::side::{Sink, Source};
use crate::{EntryKind, Sftp, SftpError};

impl Sftp {
    /// Copies one regular file into a caller-owned local file, up to `max_bytes`.
    /// Returns Unsupported for other entry types or an unknown or oversized length.
    pub async fn copy_document(
        self: &Arc<Self>,
        path: &str,
        dst: std::fs::File,
        max_bytes: u64,
    ) -> Result<(), SftpError> {
        let path = self.realpath(path).await?;
        if self.stat(&path).await?.kind != EntryKind::File {
            return Err(SftpError::Unimplemented);
        }

        let handle = self.open_file(&path, OpenFlags::READ).await?;
        let attrs = self.raw().fstat(handle.clone()).await;
        let source = Source::Remote(self.clone(), handle);
        let size = attrs.map_err(|err| map_err(&err)).and_then(|a| {
            a.attrs
                .size
                .filter(|&size| {
                    size <= max_bytes
                        && kind_of(a.attrs.permissions.unwrap_or(0)) == EntryKind::File
                })
                .ok_or(SftpError::Unimplemented)
        });
        let size = match size {
            Ok(size) => size,
            Err(err) => {
                source.close().await;
                return Err(err);
            }
        };
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
        sftp.close().await;
    }
}
