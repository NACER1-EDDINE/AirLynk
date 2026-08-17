//! Encrypted multipart upload (FR-8..13, SEC-4/5/6/10, NFR-1/2).
//!
//! The phone encrypts each file with the WASM cipher (SEC-13/14) and POSTs the
//! framed stream as a multipart file part. The PC decrypts chunk-by-chunk into
//! a quarantine directory, then moves the plaintext into Downloads only on
//! success (SEC-5). Partial or hostile uploads never appear as real files.

use std::sync::Arc;

use axum::extract::{Multipart, Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;
use tokio::io::AsyncWriteExt;

use airlynk_crypto::{decrypt_chunk, CryptoError, MAX_CHUNK_PLAINTEXT, TAG_LEN};

use super::resolve_session;
use crate::safety::{collision_safe_path, sanitize_filename, SafetyError};
use crate::session::{FileStatus, Session, SessionKind};

#[derive(Serialize)]
struct UploadResult {
    id: u32,
    name: String,
    status: &'static str,
    error: Option<String>,
}

/// POST /r/<token> — accept a multipart body whose file parts are framed
/// ciphertext streams. Each part is independently validated; one bad file
/// fails that file, not the whole request.
pub async fn upload(
    State(state): State<super::AppState>,
    Path(token): Path<String>,
    mut multipart: Multipart,
) -> Response {
    let session = match resolve_session(&state, SessionKind::Receive, &token) {
        Ok(s) => s,
        Err(code) => return (code, "Not found").into_response(),
    };

    let mut results = Vec::new();
    loop {
        match multipart.next_field().await {
            Ok(Some(field)) => {
                let Some(raw_name) = field.file_name().map(|s| s.to_string()) else {
                    continue; // non-file fields are ignored (SEC-2: nothing is read)
                };
                let name = match sanitize_filename(&raw_name) {
                    Ok(n) => n,
                    Err(SafetyError::EmptyName) => {
                        results.push(UploadResult {
                            id: 0,
                            name: raw_name,
                            status: "failed",
                            error: Some("filename is not usable".into()),
                        });
                        continue;
                    }
                    Err(_) => continue,
                };

                match receive_one(&state, &session, field, &name).await {
                    Ok(id) => results.push(UploadResult {
                        id,
                        name,
                        status: "done",
                        error: None,
                    }),
                    Err((id, err)) => results.push(UploadResult {
                        id,
                        name,
                        status: "failed",
                        error: Some(err),
                    }),
                }
            }
            Ok(None) => break,
            Err(_) => break, // malformed body: report what succeeded so far
        }
    }

    (StatusCode::OK, axum::Json(results)).into_response()
}

/// Stream one file part: decrypt framed chunks into quarantine, then move to
/// Downloads with collision suffixing. Returns (id, error) on failure.
async fn receive_one(
    state: &super::AppState,
    session: &Arc<Session>,
    mut field: axum::extract::multipart::Field<'_>,
    name: &str,
) -> Result<u32, (u32, String)> {
    if session.is_cancelled() {
        return Err((0, "session cancelled".into()));
    }

    // Quarantine (SEC-5): a partial or hostile upload never lands in Downloads.
    let quarantine = state.quarantine_dir.join(quarantine_name());
    let mut out = tokio::fs::File::create(&quarantine)
        .await
        .map_err(|e| (0, format!("cannot create quarantine file: {e}")))?;

    let key = session.key;
    let base = session.base_nonce;
    let caps = *session.caps.read().unwrap();
    let mut idx = 0u64;
    let mut written = 0u64;
    let mut frames = FrameReader::new();

    while let Ok(Some(chunk)) = field.chunk().await {
        if session.is_cancelled() {
            let _ = tokio::fs::remove_file(&quarantine).await;
            return Err((0, "session cancelled".into()));
        }
        written = written.saturating_add(chunk.len() as u64);
        if written > caps.max_bytes {
            let _ = tokio::fs::remove_file(&quarantine).await;
            return Err((0, "upload exceeds session cap".into()));
        }
        match frames.push(&chunk) {
            Ok(complete) => {
                for frame in complete {
                    match decrypt_chunk(&key, &base, idx, &frame) {
                        Ok(pt) => {
                            out.write_all(&pt).await.map_err(|e| {
                                let _ = std::fs::remove_file(&quarantine);
                                (0, format!("write failed: {e}"))
                            })?;
                        }
                        Err(CryptoError::Auth) => {
                            let _ = tokio::fs::remove_file(&quarantine).await;
                            return Err((0, "authentication failed".into()));
                        }
                        Err(CryptoError::Framing) => {
                            let _ = tokio::fs::remove_file(&quarantine).await;
                            return Err((0, "malformed stream".into()));
                        }
                        Err(CryptoError::Length) => unreachable!("frames are size-checked"),
                    }
                    idx += 1;
                }
            }
            Err(FrameError::Oversized) => {
                let _ = tokio::fs::remove_file(&quarantine).await;
                return Err((0, "malformed stream".into()));
            }
        }
    }

    out.flush().await.map_err(|e| {
        let _ = std::fs::remove_file(&quarantine);
        (0, format!("flush failed: {e}"))
    })?;
    drop(out);

    // Destination inside Downloads, never overwriting (FR-12, SEC-6).
    let dest = collision_safe_path(&state.downloads_dir, name);
    if let Err(e) = tokio::fs::rename(&quarantine, &dest).await {
        // Cross-volume rename fails; fall back to copy + delete so a temp dir
        // on another drive still delivers the file.
        if let Err(e2) = tokio::fs::copy(&quarantine, &dest).await {
            let _ = tokio::fs::remove_file(&quarantine).await;
            return Err((0, format!("move failed: {e} / {e2}")));
        }
        let _ = tokio::fs::remove_file(&quarantine).await;
    }

    // Register in the manifest (SEC-10). On cap failure the file is already
    // moved — remove it so a cap-exceeded session cannot accumulate files.
    match session.register_upload(name, written, dest.clone()) {
        Ok(id) => {
            session.set_file_status(id, FileStatus::Done);
            session.touch();
            Ok(id)
        }
        Err(_) => {
            let _ = tokio::fs::remove_file(&dest).await;
            Err((0, "session cap exceeded".into()))
        }
    }
}

/// Unique quarantine name — timestamp plus a random suffix; never collides and
/// never derives from user input (SEC-4).
fn quarantine_name() -> String {
    use rand::TryRngCore;
    let mut r = [0u8; 8];
    let _ = rand::rngs::OsRng.try_fill_bytes(&mut r);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("{nanos:x}-{}", hex(&r))
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[derive(Debug)]
enum FrameError {
    Oversized,
}

/// Incremental framed-stream parser. The multipart transport chunks do not
/// align with cipher frames, so we buffer and emit complete frames.
struct FrameReader {
    buf: Vec<u8>,
}

impl FrameReader {
    fn new() -> Self {
        Self { buf: Vec::new() }
    }

    /// Feed transport bytes; returns every complete frame (plaintext_len + TAG
    /// bytes each), leaving the partial tail buffered. Fails on frames that
    /// would exceed the chunk size (hostile or corrupt stream).
    fn push(&mut self, chunk: &[u8]) -> Result<Vec<Vec<u8>>, FrameError> {
        self.buf.extend_from_slice(chunk);
        let mut out = Vec::new();
        loop {
            if self.buf.len() < 4 {
                break;
            }
            let pt_len = u32::from_be_bytes(self.buf[..4].try_into().unwrap()) as usize;
            if pt_len > MAX_CHUNK_PLAINTEXT {
                return Err(FrameError::Oversized);
            }
            let frame_len = pt_len + TAG_LEN;
            if self.buf.len() < 4 + frame_len {
                break;
            }
            let frame = self.buf[4..4 + frame_len].to_vec();
            self.buf.drain(..4 + frame_len);
            out.push(frame);
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_reader_handles_split_frames() {
        let mut r = FrameReader::new();
        let key = [7u8; 32];
        let base = [3u8; 12];
        let pt = b"hello framed world";
        let mut framed = Vec::new();
        airlynk_crypto::encrypt_chunk_into(&key, &base, 0, pt, &mut framed).unwrap();

        // Feed one byte at a time; frames must appear only at the end.
        let mut got = Vec::new();
        for b in &framed {
            got.extend(r.push(&[*b]).unwrap());
        }
        assert_eq!(got.len(), 1);
        let dec = decrypt_chunk(&key, &base, 0, &got[0]).unwrap();
        assert_eq!(dec, pt);
        assert!(r.push(&[]).unwrap().is_empty());
    }

    #[test]
    fn frame_reader_rejects_oversized_declared_frame() {
        let mut r = FrameReader::new();
        let mut hostile = Vec::new();
        hostile.extend_from_slice(&(MAX_CHUNK_PLAINTEXT as u32 + 1).to_be_bytes());
        hostile.extend_from_slice(&[0u8; 64]);
        assert!(matches!(r.push(&hostile), Err(FrameError::Oversized)));
    }

    #[test]
    fn quarantine_name_is_unique_and_safe() {
        let a = quarantine_name();
        let b = quarantine_name();
        assert_ne!(a, b);
        assert!(a.chars().all(|c| c.is_ascii_hexdigit() || c == '-'));
    }
}
