use axum::body::Body;
use bytes::Bytes;
use std::io::SeekFrom;
use std::path::PathBuf;
use tokio::io::{AsyncReadExt, AsyncSeekExt};

const CHUNK: usize = 64 * 1024;

/// Stream `path` bytes [start, end] (inclusive) as a chunked response body.
/// Reads incrementally — never buffers the whole file (Invariant 1).
pub fn file_body(path: PathBuf, start: u64, end: u64) -> Body {
    let total = end - start + 1;
    let stream = futures::stream::unfold(
        (path, start, total, None),
        |(path, pos, remaining, file): (PathBuf, u64, u64, Option<tokio::fs::File>)| async move {
            if remaining == 0 {
                return None;
            }
            let mut file = match file {
                Some(file) => file,
                None => {
                    let mut file = tokio::fs::File::open(&path).await.ok()?;
                    file.seek(SeekFrom::Start(pos)).await.ok()?;
                    file
                }
            };
            let to_read = remaining.min(CHUNK as u64) as usize;
            let mut buf = vec![0u8; to_read];
            let read = file.read(&mut buf).await.ok()?;
            if read == 0 {
                return None;
            }
            buf.truncate(read);
            Some((
                Ok::<Bytes, std::io::Error>(Bytes::from(buf)),
                (path, pos + read as u64, remaining - read as u64, Some(file)),
            ))
        },
    );
    Body::from_stream(stream)
}
