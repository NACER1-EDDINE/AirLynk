//! Static assets for the phone client (SEC-12: fully self-contained).
//!
//! The cipher is compiled to wasm by build.rs into OUT_DIR and embedded here,
//! so the phone gets the exact same code as the native build.

use axum::http::{header, HeaderValue, StatusCode};
use axum::response::Response;

/// The phone-side AES-GCM cipher (SEC-14). Built from the same Rust source as
/// the native cipher; the cross-check in scripts/wasm-cross-check.mjs proves
/// byte-for-byte agreement.
static AESGCM_WASM: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/aesgcm.wasm"));

pub async fn wasm() -> Response {
    let mut builder = Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/wasm")
        .header(header::CONTENT_LENGTH, AESGCM_WASM.len().to_string());
    builder = builder.header(header::CACHE_CONTROL, HeaderValue::from_static("public, max-age=86400"));
    builder.body(axum::body::Body::from(AESGCM_WASM.to_vec())).unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wasm_is_embedded_and_sized() {
        // Magic header for a wasm module: \0asm, version 1.
        assert_eq!(&AESGCM_WASM[..4], b"\0asm");
        // The lean build (custom getrandom stub, no wasm-bindgen) is ~43 KB.
        assert!(AESGCM_WASM.len() > 10_000);
        assert!(AESGCM_WASM.len() < 1_000_000);
    }
}
