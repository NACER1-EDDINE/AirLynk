//! Encrypted download streaming (FR-2, FR-5, SEC-8, NFR-1/2/5).

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::{header, HeaderValue, StatusCode};
use axum::response::Response;
use bytes::Bytes;
use futures_util::Stream;
use tokio::io::AsyncReadExt;

use airlynk_crypto::{encrypt_chunk_into, MAX_CHUNK_PLAINTEXT};

use super::{resolve_session, AppState};
use crate::session::{FileStatus, Session, SessionKind};

/// GET /s/<token>/f/<id> — stream a registered file as encrypted chunks.
/// SEC-8: the path comes from the session manifest, never from the caller.
pub async fn download_file(
    State(state): State<AppState>,
    Path((token, id)): Path<(String, u32)>,
) -> Result<Response, StatusCode> {
    let session = resolve_session(&state, SessionKind::Send, &token)?;
    let file = session.file_by_id(id).ok_or(StatusCode::NOT_FOUND)?;
    let path = file.path.clone().ok_or(StatusCode::NOT_FOUND)?;

    let f = tokio::fs::File::open(&path)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;
    let meta = f
        .metadata()
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;
    let size = meta.len();

    // Ciphertext size is deterministic: every full chunk is 4 + MAX + TAG,
    // the tail (if any) is 4 + rem + TAG.
    let full = size / MAX_CHUNK_PLAINTEXT as u64;
    let rem = size % MAX_CHUNK_PLAINTEXT as u64;
    let total = full * (4 + MAX_CHUNK_PLAINTEXT as u64 + airlynk_crypto::TAG_LEN as u64)
        + if rem > 0 { 4 + rem + airlynk_crypto::TAG_LEN as u64 } else { 0 };

    let filename = sanitize_header_name(&file.original_name);
    let key = session.key;
    let stream = encrypted_stream(f, session, id, key, size);

    let mut builder = Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/octet-stream")
        .header(header::CONTENT_LENGTH, total.to_string());
    if let Ok(v) = HeaderValue::from_str(&format!("attachment; filename=\"{filename}\"")) {
        builder = builder.header(header::CONTENT_DISPOSITION, v);
    }
    Ok(builder
        .body(axum::body::Body::from_stream(stream))
        .unwrap())
}

/// Stream the file in 1 MiB plaintext chunks, each encrypted and framed on the
/// fly — memory stays flat regardless of file size (NFR-1/2). Cancellation
/// (FR-17) truncates the stream, which is the honest signal to the phone.
fn encrypted_stream(
    file: tokio::fs::File,
    session: Arc<Session>,
    file_id: u32,
    key: [u8; 32],
    size: u64,
) -> impl Stream<Item = Result<Bytes, std::io::Error>> {
    let base = session.base_nonce;
    futures_util::stream::unfold(
        (file, session, file_id, key, base, size, 0u64, vec![0u8; MAX_CHUNK_PLAINTEXT]),
        move |(mut file, session, file_id, key, base, mut remaining, mut idx, mut buffer)| async move {
            if remaining == 0 || session.is_cancelled() {
                return None;
            }
            let want = remaining.min(buffer.len() as u64) as usize;
            let mut filled = 0usize;
            while filled < want {
                match file.read(&mut buffer[filled..want]).await {
                    Ok(0) => break, // EOF before `want`: truncated source
                    Ok(n) => filled += n,
                    Err(e) => {
                        return Some((
                            Err(e),
                            (file, session, file_id, key, base, remaining, idx, buffer),
                        ))
                    }
                }
            }

            let mut framed = Vec::with_capacity(filled + 4 + airlynk_crypto::TAG_LEN);
            if encrypt_chunk_into(&key, &base, idx, &buffer[..filled], &mut framed).is_err() {
                return Some((
                    Err(std::io::Error::other("encrypt failed")),
                    (file, session, file_id, key, base, remaining, idx, buffer),
                ));
            }

            remaining -= filled as u64;
            idx += 1;
            let sent = size.saturating_sub(remaining);
            session.set_file_sent(file_id, sent);
            session.touch();
            if remaining == 0 {
                session.set_file_status(file_id, FileStatus::Done);
            }
            Some((
                Ok(Bytes::from(framed)),
                (file, session, file_id, key, base, remaining, idx, buffer),
            ))
        },
    )
}

fn sanitize_header_name(name: &str) -> String {
    name.chars()
        .map(|c| match c {
            '"' | '\\' | '\n' | '\r' => '_',
            c if c.is_control() => '_',
            c => c,
        })
        .collect()
}
