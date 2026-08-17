//! HTTP server for phone sessions (FR-14..17, FR-26..27, SEC-2/8/10/12).
//!
//! Routes (all token-gated; unknown tokens get an indistinguishable 404):
//!   GET  /s/<token>          Send listing page (Phase 4 replaces the shell)
//!   GET  /s/<token>/f/<id>   Encrypted download stream (download.rs)
//!   GET  /r/<token>          Receive page (Phase 4 replaces the shell)
//!   POST /r/<token>          Encrypted multipart upload (upload.rs)
//!   GET  /aesgcm.wasm        The cipher, from OUT_DIR (assets.rs)
//!
//! The phone client must be fully self-contained (SEC-12): the listing page
//! references only inline CSS and the /aesgcm.wasm served from the binary.

pub mod assets;
pub mod download;
pub mod upload;

use std::path::PathBuf;
use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use axum::routing::get;
use base64::Engine;
use serde::Serialize;
use axum::Router;

use crate::session::{Session, SessionKind, SessionRegistry};

#[derive(Clone)]
pub struct AppState {
    pub registry: Arc<SessionRegistry>,
    /// Known-folder Downloads; resolved by the shell in Phase 5 (FR-11).
    pub downloads_dir: PathBuf,
    /// Quarantine for in-flight uploads; partial uploads never appear in
    /// Downloads (SEC-5).
    pub quarantine_dir: PathBuf,
}

pub fn app(state: AppState) -> Router {
    use axum::extract::DefaultBodyLimit;
    Router::new()
        .route("/s/{token}", get(listing))
        .route("/s/{token}/f/{id}", get(download::download_file))
        // The upload route does its own per-session byte accounting (SEC-10);
        // axum's default 2 MiB body limit would otherwise reject large files.
        .route(
            "/r/{token}",
            get(receive_page)
                .post(upload::upload)
                .layer(DefaultBodyLimit::disable()),
        )
        .route("/aesgcm.wasm", get(assets::wasm))
        .with_state(state)
}

/// Resolve a session by token for the given kind, or an indistinguishable 404
/// (FR-16, SEC-2). Touches activity so an in-use session is not reaped.
fn resolve_session(
    state: &AppState,
    kind: SessionKind,
    token: &str,
) -> Result<Arc<Session>, StatusCode> {
    let session = state
        .registry
        .find_by_token(token)
        .ok_or(StatusCode::NOT_FOUND)?;
    if session.kind != kind {
        return Err(StatusCode::NOT_FOUND);
    }
    Ok(session)
}

async fn listing(
    State(state): State<AppState>,
    Path(token): Path<String>,
) -> Result<Response, StatusCode> {
    let session = resolve_session(&state, SessionKind::Send, &token)?;
    let files: Vec<PhoneFile> = session
        .files()
        .into_iter()
        .map(|f| PhoneFile {
            id: f.id,
            name: f.original_name,
            size: f.size,
        })
        .collect();
    let files_b64 = base64::engine::general_purpose::STANDARD_NO_PAD
        .encode(serde_json::to_vec(&files).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?);
    let nonce = nonce_js_array(&session.base_nonce);

    Ok(Html(render_send_page(
        &token,
        &session.display_code,
        &files_b64,
        &nonce,
    ))
    .into_response())
}

async fn receive_page(
    State(state): State<AppState>,
    Path(token): Path<String>,
) -> Result<Response, StatusCode> {
    let session = resolve_session(&state, SessionKind::Receive, &token)?;
    let nonce = nonce_js_array(&session.base_nonce);
    Ok(Html(render_receive_page(
        &token,
        &session.display_code,
        &nonce,
    ))
    .into_response())
}

#[derive(Serialize)]
struct PhoneFile {
    id: u32,
    name: String,
    size: u64,
}

fn nonce_js_array(base: &[u8; 12]) -> String {
    base.iter()
        .map(u8::to_string)
        .collect::<Vec<_>>()
        .join(",")
}

fn shell_styles() -> &'static str {
    r#"*{box-sizing:border-box}body{margin:0;font-family:Segoe UI,system-ui,sans-serif;background:#0e1116;color:#e8ecf0}
main{max-width:760px;margin:0 auto;padding:20px 16px 28px}h1{margin:0 0 8px;font-size:22px}p{margin:0 0 12px;color:#aeb7c2}
.chip{display:inline-block;padding:6px 10px;border:1px solid #8b939c;border-radius:999px;font-size:12px;letter-spacing:.08em}
.card{margin-top:14px;padding:14px;border:1px solid #2f3640;border-radius:10px;background:#141920}
.row{display:flex;justify-content:space-between;gap:12px;align-items:center;padding:10px 0;border-bottom:1px solid #232a34}
.row:last-child{border-bottom:none}.muted{color:#8b939c;font-size:12px}
button{border:1px solid #3b83f6;background:#3b83f6;color:#fff;padding:8px 12px;border-radius:8px;font-weight:600}
button:disabled{opacity:.55}input[type=file]{width:100%;padding:10px;border-radius:8px;border:1px solid #2f3640;background:#0f141b;color:#e8ecf0}
progress{width:100%;height:12px}.status{margin-top:10px;font-size:13px;line-height:1.5;white-space:pre-wrap}"#
}

fn wasm_crypto_runtime() -> &'static str {
    r#"const B64URL = s => s.replace(/-/g,'+').replace(/_/g,'/');
function decodeKeyFromHash(){
  const raw = decodeURIComponent((location.hash||'').replace(/^#/, '').trim());
  if(!raw) throw new Error('Missing key in QR URL fragment.');
  const b64 = B64URL(raw); const pad = '='.repeat((4 - b64.length % 4) % 4);
  const bytes = Uint8Array.from(atob(b64 + pad), c => c.charCodeAt(0));
  if(bytes.length !== 32) throw new Error('Session key must be 32 bytes.');
  return bytes;
}
async function loadCipher(){
  const response = await fetch('/aesgcm.wasm', { cache: 'no-store' });
  if(!response.ok) throw new Error('Unable to load encryption module.');
  const bytes = await response.arrayBuffer();
  const { instance } = await WebAssembly.instantiate(bytes, {});
  const e = instance.exports;
  if(!e.memory || !e.airlynk_encrypt || !e.airlynk_decrypt) throw new Error('Invalid cipher module.');
  const reserve = n => { const pages = Math.ceil((n + 65536) / 65536); if (e.memory.buffer.byteLength < n) e.memory.grow(Math.ceil((n - e.memory.buffer.byteLength) / 65536)); return 0; };
  return {
    encryptChunk(key, nonce, idx, plain){
      const outLen = e.airlynk_encrypt_len(plain.length);
      const needed = 44 + plain.length + outLen;
      reserve(needed);
      const mem = new Uint8Array(e.memory.buffer);
      const keyPtr = 0, noncePtr = 32, plainPtr = 44, outPtr = 44 + plain.length;
      mem.set(key, keyPtr); mem.set(nonce, noncePtr); mem.set(plain, plainPtr);
      const rc = e.airlynk_encrypt(keyPtr, key.length, noncePtr, nonce.length, idx >>> 0, Math.floor(idx / 4294967296), plainPtr, plain.length, outPtr);
      if (rc !== 0) throw new Error('Encryption failed.');
      return mem.slice(outPtr, outPtr + outLen);
    },
    decryptChunk(key, nonce, idx, framed){
      const outLen = e.airlynk_decrypt_len(framed.length);
      if (!outLen) throw new Error('Malformed encrypted frame.');
      const needed = 44 + framed.length + outLen;
      reserve(needed);
      const mem = new Uint8Array(e.memory.buffer);
      const keyPtr = 0, noncePtr = 32, framePtr = 44, outPtr = 44 + framed.length;
      mem.set(key, keyPtr); mem.set(nonce, noncePtr); mem.set(framed, framePtr);
      const rc = e.airlynk_decrypt(keyPtr, key.length, noncePtr, nonce.length, idx >>> 0, Math.floor(idx / 4294967296), framePtr, framed.length, outPtr);
      if (rc !== 0) throw new Error('Decryption failed. Check the QR session key.');
      return mem.slice(outPtr, outPtr + outLen);
    }
  };
}"#
}

fn render_send_page(token: &str, display_code: &str, files_b64: &str, nonce: &str) -> String {
    format!(
        r#"<!doctype html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>AirLynk</title><style>{styles}</style></head>
<body><main><h1>Download from AirLynk</h1><p>Session code</p><p class="chip">{code}</p>
<div class="card"><div id="list"></div><div id="status" class="status muted">Pick a file to download and decrypt on this phone.</div></div></main>
<script>
const TOKEN = "{token}";
const NONCE = new Uint8Array([{nonce}]);
const FILES = JSON.parse(atob("{files_b64}"));
const fmt = n => n < 1024 ? `${{n}} B` : n < 1024*1024 ? `${{(n/1024).toFixed(1)}} KB` : `${{(n/1024/1024).toFixed(1)}} MB`;
{runtime}
const statusEl = document.getElementById('status');
const listEl = document.getElementById('list');
function setStatus(text, muted=false){{ statusEl.textContent=text; statusEl.className='status'+(muted?' muted':''); }}
for (const f of FILES) {{
  const row = document.createElement('div'); row.className='row';
  const left = document.createElement('div'); left.innerHTML = `<div>${{f.name}}</div><div class="muted">${{fmt(f.size)}}</div>`;
  const btn = document.createElement('button'); btn.textContent='Download'; btn.onclick=() => downloadFile(f, btn);
  row.append(left, btn); listEl.append(row);
}}
async function downloadFile(file, btn) {{
  btn.disabled = true;
  try {{
    setStatus(`Downloading ${{file.name}}...`, true);
    const key = decodeKeyFromHash();
    const cipher = await loadCipher();
    const res = await fetch(`/s/${{TOKEN}}/f/${{file.id}}`);
    if (!res.ok) throw new Error('Download failed.');
    const total = Number(res.headers.get('content-length') || 0);
    const reader = res.body.getReader();
    const chunks = []; let received = 0;
    while (true) {{
      const r = await reader.read(); if (r.done) break;
      chunks.push(r.value); received += r.value.length;
      if (total > 0) setStatus(`Downloading ${{file.name}}... ${{Math.round(received*100/total)}}%`, true);
    }}
    const framed = new Uint8Array(received); let off = 0;
    for (const c of chunks) {{ framed.set(c, off); off += c.length; }}
    const plainParts = []; let pos = 0; let idx = 0;
    while (pos < framed.length) {{
      if (pos + 4 > framed.length) throw new Error('Malformed encrypted stream.');
      const ptLen = (framed[pos]<<24) | (framed[pos+1]<<16) | (framed[pos+2]<<8) | framed[pos+3];
      const frameLen = 4 + ptLen + 16;
      if (pos + frameLen > framed.length) throw new Error('Malformed encrypted stream.');
      const part = cipher.decryptChunk(key, NONCE, idx++, framed.slice(pos, pos + frameLen));
      plainParts.push(part); pos += frameLen;
    }}
    const blob = new Blob(plainParts, {{ type:'application/octet-stream' }});
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a'); a.href = url; a.download = file.name; a.click();
    URL.revokeObjectURL(url);
    setStatus(`Downloaded ${{file.name}} successfully.`);
  }} catch (e) {{
    setStatus(e?.message || 'Download failed.');
  }} finally {{ btn.disabled = false; }}
}}
</script></body></html>"#,
        styles = shell_styles(),
        code = display_code,
        token = token,
        nonce = nonce,
        files_b64 = files_b64,
        runtime = wasm_crypto_runtime()
    )
}

fn render_receive_page(token: &str, display_code: &str, nonce: &str) -> String {
    format!(
        r#"<!doctype html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>AirLynk</title><style>{styles}</style></head>
<body><main><h1>Upload to AirLynk</h1><p>Session code</p><p class="chip">{code}</p>
<div class="card"><input id="picker" type="file" multiple><div style="height:10px"></div><button id="send">Upload selected files</button>
<div style="height:10px"></div><progress id="progress" max="100" value="0"></progress><div id="status" class="status muted">Select files, then upload.</div></div></main>
<script>
const TOKEN = "{token}";
const NONCE = new Uint8Array([{nonce}]);
const CHUNK = 1024 * 1024;
{runtime}
const picker = document.getElementById('picker');
const send = document.getElementById('send');
const progress = document.getElementById('progress');
const statusEl = document.getElementById('status');
function setStatus(text, muted=false){{ statusEl.textContent=text; statusEl.className='status'+(muted?' muted':''); }}
async function encryptFile(file, key, cipher) {{
  const raw = new Uint8Array(await file.arrayBuffer());
  const parts = []; let idx = 0;
  for (let i = 0; i < raw.length; i += CHUNK) {{
    const plain = raw.slice(i, Math.min(i + CHUNK, raw.length));
    parts.push(cipher.encryptChunk(key, NONCE, idx++, plain));
  }}
  return new Blob(parts, {{ type: 'application/octet-stream' }});
}}
send.onclick = async () => {{
  if (!picker.files || picker.files.length === 0) {{ setStatus('Pick at least one file first.'); return; }}
  send.disabled = true;
  try {{
    progress.value = 0;
    const key = decodeKeyFromHash();
    const cipher = await loadCipher();
    const form = new FormData();
    for (const file of picker.files) {{
      setStatus(`Encrypting ${{file.name}}...`, true);
      const encrypted = await encryptFile(file, key, cipher);
      form.append('files', encrypted, file.name);
    }}
    setStatus('Uploading...', true);
    await new Promise((resolve, reject) => {{
      const xhr = new XMLHttpRequest();
      xhr.open('POST', `/r/${{TOKEN}}`);
      xhr.upload.onprogress = (e) => {{ if (e.lengthComputable) progress.value = Math.round((e.loaded / e.total) * 100); }};
      xhr.onload = () => xhr.status >= 200 && xhr.status < 300 ? resolve(xhr.responseText) : reject(new Error('Upload failed.'));
      xhr.onerror = () => reject(new Error('Upload failed.'));
      xhr.send(form);
    }});
    setStatus('Upload finished. You can close this page now.');
  }} catch (e) {{
    setStatus(e?.message || 'Upload failed.');
  }} finally {{
    send.disabled = false;
  }}
}};
</script></body></html>"#,
        styles = shell_styles(),
        code = display_code,
        token = token,
        nonce = nonce,
        runtime = wasm_crypto_runtime()
    )
}

/// Constant-time 404 body so unknown and expired tokens are indistinguishable
/// even in the response body (FR-16): routes use `StatusCode::NOT_FOUND` with
/// a plain "Not found" body via `resolve_session`.

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::{FileStatus, SessionKind, DEFAULT_INACTIVITY_TIMEOUT};
    use std::sync::atomic::{AtomicU64, Ordering};
    use tokio::net::TcpListener;

    static STATE_SEQ: AtomicU64 = AtomicU64::new(0);

    fn state() -> (AppState, Arc<SessionRegistry>) {
        let seq = STATE_SEQ.fetch_add(1, Ordering::Relaxed);
        let registry = Arc::new(SessionRegistry::new(DEFAULT_INACTIVITY_TIMEOUT));
        let downloads = std::env::temp_dir().join(format!("airlynk-dl-{}-{seq}", std::process::id()));
        let quarantine = std::env::temp_dir().join(format!("airlynk-q-{}-{seq}", std::process::id()));
        std::fs::create_dir_all(&downloads).unwrap();
        std::fs::create_dir_all(&quarantine).unwrap();
        (
            AppState {
                registry: registry.clone(),
                downloads_dir: downloads,
                quarantine_dir: quarantine,
            },
            registry,
        )
    }

    #[tokio::test]
    async fn unknown_token_gets_plain_404_everywhere() {
        let (st, _) = state();
        let app = app(st);
        use tower::ServiceExt;
        for path in [
            "/s/AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
            "/s/AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA/f/0",
            "/r/AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
        ] {
            let resp = app
                .clone()
                .oneshot(
                    axum::http::Request::builder()
                        .uri(path)
                        .body(axum::body::Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(resp.status(), StatusCode::NOT_FOUND, "{path}");
        }
    }

    #[tokio::test]
    async fn wrong_kind_token_is_indistinguishable_404() {
        let (st, reg) = state();
        let send = reg.create_send_session(vec![]);
        let recv = reg.create_receive_session();
        let app = app(st);
        use tower::ServiceExt;
        // A Send token must not open the receive page.
        let resp = app
            .clone()
            .oneshot(
                axum::http::Request::builder()
                    .uri(format!("/r/{}", send.token))
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
        // A Receive token must not open the send listing.
        let resp = app
            .clone()
            .oneshot(
                axum::http::Request::builder()
                    .uri(format!("/s/{}", recv.token))
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn send_listing_serves_for_valid_token() {
        let (st, reg) = state();
        let s = reg.create_send_session(vec![]);
        let app = app(st);
        use tower::ServiceExt;
        let resp = app
            .oneshot(
                axum::http::Request::builder()
                    .uri(format!("/s/{}", s.token))
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    /// Deterministic pseudo-random bytes for test payloads.
    fn payload(n: usize, seed: u8) -> Vec<u8> {
        let mut v = Vec::with_capacity(n);
        let mut x = seed as u64;
        for _ in 0..n {
            x = x.wrapping_mul(1103515245).wrapping_add(12345);
            v.push((x >> 16) as u8);
        }
        v
    }

    async fn collect(resp: Response) -> Vec<u8> {
        use http_body_util::BodyExt;
        let body = resp.into_body();
        let collected = body.collect().await.unwrap();
        collected.to_bytes().to_vec()
    }

    #[tokio::test]
    async fn download_round_trips_encrypted_and_decrypts() {
        let (st, reg) = state();
        let dir = std::env::temp_dir().join(format!("airlynk-src-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let src = dir.join("holiday-photo.jpg");
        let pt = payload(3 * 1024 * 1024 + 777, 42);
        std::fs::write(&src, &pt).unwrap();
        let size = pt.len() as u64;

        let s = reg.create_send_session(vec![("holiday-photo.jpg".into(), size, src)]);
        let app = app(st);
        use tower::ServiceExt;
        let resp = app
            .oneshot(
                axum::http::Request::builder()
                    .uri(format!("/s/{}/f/0", s.token))
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        // Content-Length must equal the deterministic ciphertext size.
        let cl = resp
            .headers()
            .get(axum::http::header::CONTENT_LENGTH)
            .unwrap()
            .to_str()
            .unwrap()
            .parse::<u64>()
            .unwrap();
        let full = size / airlynk_crypto::MAX_CHUNK_PLAINTEXT as u64;
        let rem = size % airlynk_crypto::MAX_CHUNK_PLAINTEXT as u64;
        let expected = full * (4 + airlynk_crypto::MAX_CHUNK_PLAINTEXT as u64 + airlynk_crypto::TAG_LEN as u64)
            + if rem > 0 { 4 + rem + airlynk_crypto::TAG_LEN as u64 } else { 0 };
        assert_eq!(cl, expected);

        let body = collect(resp).await;
        let decrypted = airlynk_crypto::decrypt_all(&s.key, &s.base_nonce, &body).unwrap();
        assert_eq!(decrypted, pt);
        // Wire bytes must NOT contain plaintext (SEC-13).
        assert!(!body.windows(pt.len().min(64)).any(|w| w == &pt[..pt.len().min(64)]));
    }

    #[tokio::test]
    async fn download_unknown_file_id_is_404() {
        let (st, reg) = state();
        let dir = std::env::temp_dir().join(format!("airlynk-src-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let src = dir.join("a.bin");
        std::fs::write(&src, b"data").unwrap();
        let s = reg.create_send_session(vec![("a.bin".into(), 4, src)]);
        let app = app(st);
        use tower::ServiceExt;
        let resp = app
            .oneshot(
                axum::http::Request::builder()
                    .uri(format!("/s/{}/f/999", s.token))
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    /// Build a multipart/form-data body with one or more file parts.
    fn multipart_body(boundary: &str, parts: &[(&str, &str, &[u8])]) -> Vec<u8> {
        let mut body = Vec::new();
        for (name, filename, data) in parts {
            body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
            body.extend_from_slice(
                format!("Content-Disposition: form-data; name=\"{name}\"; filename=\"{filename}\"\r\n")
                    .as_bytes(),
            );
            body.extend_from_slice(b"Content-Type: application/octet-stream\r\n\r\n");
            body.extend_from_slice(data);
            body.extend_from_slice(b"\r\n");
        }
        body.extend_from_slice(format!("--{boundary}--\r\n").as_bytes());
        body
    }

    fn upload_request(token: &str, body: Vec<u8>, boundary: &str) -> axum::http::Request<axum::body::Body> {
        axum::http::Request::builder()
            .method("POST")
            .uri(format!("/r/{token}"))
            .header(
                axum::http::header::CONTENT_TYPE,
                format!("multipart/form-data; boundary={boundary}"),
            )
            .body(axum::body::Body::from(body))
            .unwrap()
    }

    #[tokio::test]
    async fn upload_round_trips_to_downloads_byte_for_byte() {
        let (st, reg) = state();
        let s = reg.create_receive_session();
        let pt = payload(2 * 1024 * 1024 + 999, 7);
        let framed = airlynk_crypto::encrypt_all(&s.key, &s.base_nonce, &pt);
        let body = multipart_body("xyz", &[("file", "holiday.jpg", &framed)]);
        let app = app(st.clone());
        use tower::ServiceExt;
        let resp = app.oneshot(upload_request(&s.token, body, "xyz")).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body_bytes = collect(resp).await;
        let json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
        assert_eq!(json[0]["status"], "done");

        let dest = st.downloads_dir.join("holiday.jpg");
        assert_eq!(std::fs::read(&dest).unwrap(), pt);
        // Manifest registered with the final path.
        let file = s.file_by_id(json[0]["id"].as_u64().unwrap() as u32).unwrap();
        assert_eq!(file.path.as_deref(), Some(dest.as_path()));
        assert_eq!(*file.status.lock().unwrap(), FileStatus::Done);
    }

    #[tokio::test]
    async fn upload_collision_suffixes_instead_of_overwriting() {
        let (st, reg) = state();
        let s = reg.create_receive_session();
        let pt = payload(1000, 1);
        let framed = airlynk_crypto::encrypt_all(&s.key, &s.base_nonce, &pt);
        std::fs::write(st.downloads_dir.join("clip.mp4"), b"first").unwrap();
        let body = multipart_body("b1", &[("file", "clip.mp4", &framed)]);
        let app = app(st.clone());
        use tower::ServiceExt;
        let resp = app.oneshot(upload_request(&s.token, body, "b1")).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(std::fs::read(st.downloads_dir.join("clip.mp4")).unwrap(), b"first");
        assert_eq!(std::fs::read(st.downloads_dir.join("clip (1).mp4")).unwrap(), pt);
    }

    #[tokio::test]
    async fn upload_sanitizes_hostile_filenames() {
        let (st, reg) = state();
        let s = reg.create_receive_session();
        let pt = payload(500, 9);
        let framed = airlynk_crypto::encrypt_all(&s.key, &s.base_nonce, &pt);
        let body = multipart_body("b2", &[("file", "../../evil.txt", &framed)]);
        let app = app(st.clone());
        use tower::ServiceExt;
        let resp = app.oneshot(upload_request(&s.token, body, "b2")).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        // SEC-4/6: the file lands inside Downloads under a sanitized name.
        assert_eq!(std::fs::read(st.downloads_dir.join("evil.txt")).unwrap(), pt);
        assert!(!st.downloads_dir.join("..").join("evil.txt").exists());
    }

    #[tokio::test]
    async fn upload_tampered_ciphertext_is_rejected_and_leaves_no_file() {
        let (st, reg) = state();
        let s = reg.create_receive_session();
        let pt = payload(2000, 3);
        let mut framed = airlynk_crypto::encrypt_all(&s.key, &s.base_nonce, &pt);
        let mid = framed.len() / 2;
        framed[mid] ^= 0x80;
        let body = multipart_body("b3", &[("file", "evil.bin", &framed)]);
        let app = app(st.clone());
        use tower::ServiceExt;
        let resp = app.oneshot(upload_request(&s.token, body, "b3")).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let json: serde_json::Value = serde_json::from_slice(&collect(resp).await).unwrap();
        assert_eq!(json[0]["status"], "failed");
        assert!(!st.downloads_dir.join("evil.bin").exists());
        // Quarantine must be empty of the failed file.
        let leftovers: Vec<_> = std::fs::read_dir(&st.quarantine_dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .collect();
        assert!(leftovers.is_empty(), "quarantine must be cleaned on failure");
    }

    #[tokio::test]
    async fn upload_unknown_token_is_404() {
        let (st, _) = state();
        let body = multipart_body("b4", &[("file", "x.bin", b"junk")]);
        let app = app(st);
        use tower::ServiceExt;
        let resp = app
            .oneshot(upload_request("AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA", body, "b4"))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn wasm_asset_is_served_and_valid() {
        let (st, _) = state();
        let app = app(st);
        use tower::ServiceExt;
        let resp = app
            .oneshot(
                axum::http::Request::builder()
                    .uri("/aesgcm.wasm")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let bytes = collect(resp).await;
        assert_eq!(&bytes[..4], b"\0asm");
    }

    /// Current process working set (Windows). Used to prove flat memory: the
    /// RSS delta during a multi-GB transfer must stay far below the file size
    /// (NFR-1/2 exit criterion).
    fn working_set_bytes() -> u64 {
        use windows_sys::Win32::System::ProcessStatus::{
            GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS,
        };
        use windows_sys::Win32::System::Threading::GetCurrentProcess;
        unsafe {
            let h = GetCurrentProcess();
            let mut pmc: PROCESS_MEMORY_COUNTERS = std::mem::zeroed();
            pmc.cb = std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32;
            GetProcessMemoryInfo(h, &mut pmc, pmc.cb);
            pmc.WorkingSetSize as u64
        }
    }

    /// Exit criterion: a multi-GB file moves end-to-end over a real socket
    /// with flat memory. Slower in debug; run explicitly with
    /// `cargo test -p airlynk -- --ignored`.
    #[tokio::test]
    #[ignore = "multi-GB transfer; run explicitly as an exit-criteria check"]
    async fn multi_gb_download_streams_with_flat_memory() {
        let (st, reg) = state();
        let size = 2u64 * 1024 * 1024 * 1024; // 2 GiB
        let dir = std::env::temp_dir().join(format!("airlynk-big-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let src = dir.join("big.bin");
        let f = tokio::fs::File::create(&src).await.unwrap();
        f.set_len(size).await.unwrap(); // all zeros; sparse-ish, cheap
        drop(f);

        let s = reg.create_send_session(vec![("big.bin".into(), size, src)]);

        // Real socket server on an ephemeral port.
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let server_app = app(st.clone());
        let server_task = tokio::spawn(async move {
            let _ = axum::serve(listener, server_app).await;
        });

        let before = working_set_bytes();
        let mut stream = tokio::net::TcpStream::connect(format!("127.0.0.1:{port}"))
            .await
            .unwrap();
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        stream
            .write_all(
                format!("GET /s/{}/f/0 HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\n\r\n", s.token)
                    .as_bytes(),
            )
            .await
            .unwrap();

        // Read headers.
        let mut head = Vec::new();
        let mut byte = [0u8; 1];
        while !head.ends_with(b"\r\n\r\n") {
            stream.read_exact(&mut byte).await.unwrap();
            head.push(byte[0]);
        }
        let head_str = String::from_utf8_lossy(&head);
        assert!(
            head_str.starts_with("HTTP/1.1 200"),
            "unexpected head: {head_str}"
        );
        let cl = head_str
            .lines()
            .find_map(|l| {
                l.to_ascii_lowercase()
                    .strip_prefix("content-length:")
                    .map(|v| v.trim().parse::<u64>().unwrap())
            })
            .expect("content-length present");

        // Stream the body, frame-by-frame, decrypting and discarding.
        // Memory must stay flat: no buffer scales with the file size.
        let mut pending = Vec::with_capacity(4 + airlynk_crypto::MAX_CHUNK_PLAINTEXT + 16);
        let mut total = 0u64;
        let mut decrypted = 0u64;
        let mut idx = 0u64;
        let mut buf = [0u8; 1 << 16];
        loop {
            let n = stream.read(&mut buf).await.unwrap();
            if n == 0 {
                break;
            }
            total += n as u64;
            pending.extend_from_slice(&buf[..n]);
            loop {
                if pending.len() < 4 {
                    break;
                }
                let pt_len =
                    u32::from_be_bytes(pending[..4].try_into().unwrap()) as usize;
                if pt_len > airlynk_crypto::MAX_CHUNK_PLAINTEXT {
                    panic!("oversized frame in stream");
                }
                if pending.len() < 4 + pt_len + airlynk_crypto::TAG_LEN {
                    break;
                }
                let ct = pending[4..4 + pt_len + airlynk_crypto::TAG_LEN].to_vec();
                pending.drain(..4 + pt_len + airlynk_crypto::TAG_LEN);
                let pt = airlynk_crypto::decrypt_chunk(&s.key, &s.base_nonce, idx, &ct)
                    .unwrap_or_else(|e| panic!("decrypt chunk {idx}: {e}"));
                assert_eq!(pt.len(), pt_len);
                assert!(pt.iter().all(|b| *b == 0), "chunk {idx} not zeros");
                decrypted += pt_len as u64;
                idx += 1;
            }
        }
        let after = working_set_bytes();

        assert_eq!(total, cl, "body length must match Content-Length");
        assert_eq!(decrypted, size, "all plaintext must round-trip");
        let delta = after.saturating_sub(before);
        assert!(
            delta < 300 * 1024 * 1024,
            "memory grew by {delta} bytes for a {size}-byte file — not flat"
        );
        eprintln!("OK: {size} bytes streamed, RSS delta {delta} bytes");

        server_task.abort();
        let _ = server_task.await;
    }
}
