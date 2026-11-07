//! Pipelined single-file copy: a producer task reads ordered chunks into a bounded channel
//! (backpressure = the 8 MiB buffer), the consumer writes them (remote sinks keep up to
//! `depth` write requests in flight).

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use futures_util::stream::{FuturesOrdered, FuturesUnordered, StreamExt};
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::remote::{is_eof, map_err};
use crate::side::{Sink, Source};
use crate::SftpError;

/// Bounded buffer between reader and writer.
pub(crate) const BUFFER_BYTES: usize = 8 << 20;

/// Copy parameters.
#[derive(Clone, Copy)]
pub(crate) struct Plan {
    pub chunk: u32,
    pub depth: usize,
    pub start: u64,
    pub size: u64,
}

type Chunk = Result<(u64, Vec<u8>), SftpError>;

pub(crate) async fn read_range(
    sftp: &crate::Sftp,
    handle: &str,
    off: u64,
    len: u32,
) -> Result<Vec<u8>, SftpError> {
    let mut out: Vec<u8> = Vec::new();
    while out.len() < len as usize {
        let want = len - u32::try_from(out.len()).unwrap_or(len);
        let pos = off + out.len() as u64;
        match sftp.raw().read(handle.to_owned(), pos, want).await {
            Ok(d) if d.data.is_empty() => break,
            Ok(d) => out.extend_from_slice(&d.data),
            Err(e) if is_eof(&e) => break,
            Err(e) => return Err(map_err(&e)),
        }
    }
    Ok(out)
}

fn changed() -> SftpError {
    SftpError::Protocol("source changed during transfer".into())
}

async fn produce(src: Source, tx: mpsc::Sender<Chunk>, p: Plan) {
    let r = match src {
        Source::Local(mut f) => {
            let r = produce_local(&mut f, &tx, p).await;
            drop(f);
            r
        }
        Source::Remote(s, h) => {
            let r = produce_remote(&s, &h, &tx, p).await;
            let _ = s.raw().close(h).await;
            r
        }
    };
    if let Err(e) = r {
        let _ = tx.send(Err(e)).await;
    }
}

async fn produce_local(
    f: &mut tokio::fs::File,
    tx: &mpsc::Sender<Chunk>,
    p: Plan,
) -> Result<(), SftpError> {
    f.seek(std::io::SeekFrom::Start(p.start)).await?;
    let mut off = p.start;
    while off < p.size {
        let want = usize::try_from((p.size - off).min(u64::from(p.chunk))).unwrap_or(0);
        let mut buf = vec![0u8; want];
        let mut got = 0;
        while got < want {
            let n = f.read(&mut buf[got..]).await?;
            if n == 0 {
                return Err(changed());
            }
            got += n;
        }
        if tx.send(Ok((off, buf))).await.is_err() {
            return Ok(());
        }
        off += want as u64;
    }
    Ok(())
}

async fn produce_remote(
    s: &crate::Sftp,
    h: &str,
    tx: &mpsc::Sender<Chunk>,
    p: Plan,
) -> Result<(), SftpError> {
    let mut pending = FuturesOrdered::new();
    let mut next = p.start;
    loop {
        while next < p.size && pending.len() < p.depth {
            let len = u32::try_from((p.size - next).min(u64::from(p.chunk))).unwrap_or(p.chunk);
            let off = next;
            pending.push_back(async move { (off, len, read_range(s, h, off, len).await) });
            next += u64::from(len);
        }
        let Some((off, len, res)) = pending.next().await else {
            return Ok(());
        };
        let data = res?;
        if data.len() != len as usize {
            return Err(changed());
        }
        if tx.send(Ok((off, data))).await.is_err() {
            return Ok(());
        }
    }
}

/// Copies `[plan.start, plan.size)` from `src` to `dst` (already positioned at `plan.start`).
/// `progress` counts confirmed bytes. Consumes both ends.
pub(crate) async fn copy(
    src: Source,
    dst: Sink,
    p: Plan,
    cancel: &CancellationToken,
    progress: &AtomicU64,
) -> Result<(), SftpError> {
    let cap = (BUFFER_BYTES / p.chunk.max(1) as usize).max(2);
    let (tx, mut rx) = mpsc::channel::<Chunk>(cap);
    let producer = tokio::spawn(produce(src, tx, p));
    let res = consume(&mut rx, dst, p, cancel, progress).await;
    drop(rx);
    let _ = producer.await;
    res
}

async fn consume(
    rx: &mut mpsc::Receiver<Chunk>,
    mut dst: Sink,
    p: Plan,
    cancel: &CancellationToken,
    progress: &AtomicU64,
) -> Result<(), SftpError> {
    let mut inflight = FuturesUnordered::new();
    let result: Result<(), SftpError> = async {
        loop {
            tokio::select! {
                biased;
                () = cancel.cancelled() => {
                    // Let already-issued writes land so the partial file is a clean prefix.
                    while let Some(r) = inflight.next().await {
                        if let Ok(n) = r {
                            progress.fetch_add(n, Ordering::Relaxed);
                        }
                    }
                    return Err(SftpError::Cancelled);
                }
                Some(r) = inflight.next(), if !inflight.is_empty() => {
                    let n: u64 = r?;
                    progress.fetch_add(n, Ordering::Relaxed);
                }
                item = rx.recv(), if inflight.len() < p.depth => {
                    let Some(item) = item else { break };
                    let (off, data) = item?;
                    match &mut dst {
                        Sink::Local(w) => {
                            w.write_all(&data).await?;
                            progress.fetch_add(data.len() as u64, Ordering::Relaxed);
                        }
                        Sink::Remote(s, h) => {
                            let raw = Arc::clone(s.raw());
                            let h = h.clone();
                            inflight.push(async move {
                                let n = data.len() as u64;
                                raw.write(h, off, data).await.map_err(|e| map_err(&e))?;
                                Ok::<u64, SftpError>(n)
                            });
                        }
                    }
                }
            }
        }
        while let Some(r) = inflight.next().await {
            let n: u64 = r?;
            progress.fetch_add(n, Ordering::Relaxed);
        }
        Ok(())
    }
    .await;
    drop(inflight);
    match result {
        Ok(()) => dst.finish().await,
        Err(e) => {
            let _ = dst.finish().await;
            Err(e)
        }
    }
}

/// Reads up to `len` bytes at `off` from a source (used to verify a partial file's tail).
pub(crate) async fn read_at(src: Source, off: u64, len: u32) -> Result<Vec<u8>, SftpError> {
    match src {
        Source::Local(mut f) => {
            f.seek(std::io::SeekFrom::Start(off)).await?;
            let mut buf = vec![0u8; len as usize];
            let mut got = 0;
            while got < buf.len() {
                let n = f.read(&mut buf[got..]).await?;
                if n == 0 {
                    break;
                }
                got += n;
            }
            buf.truncate(got);
            Ok(buf)
        }
        Source::Remote(s, h) => {
            let r = read_range(&s, &h, off, len).await;
            let _ = s.raw().close(h).await;
            r
        }
    }
}
