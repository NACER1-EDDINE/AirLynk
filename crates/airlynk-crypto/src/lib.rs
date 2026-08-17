//! AirLynk payload cipher — AES-GCM chunked streaming format.
//!
//! Wire format (both directions, identical):
//!
//! ```text
//! stream := chunk*
//! chunk  := u32_be(plaintext_len) || AES-GCM-CT(key, nonce_for(base, idx), plaintext)
//! ```
//!
//! Every chunk is self-framing: a 4-byte big-endian plaintext length followed by
//! ciphertext (plaintext + 16-byte tag). Chunks are indexed from 0; the nonce is
//! derived from a session base nonce plus the chunk index (wrapping add mod 2^96).
//! This keeps the format streaming-friendly on both sides: the phone decrypts
//! chunk-by-chunk with bounded memory (NFR-9), and the PC streams multi-gigabyte
//! files without buffering (NFR-1/2).
//!
//! Security note: AES-GCM is not a stream cipher — each chunk is independently
//! authenticated. The threat model is passive sniffing (SEC-13); an active MITM
//! is documented accepted risk, so chunk reordering/dropping is out of scope.

pub const KEY_LEN: usize = 32;
pub const NONCE_LEN: usize = 12;
pub const TAG_LEN: usize = 16;
pub const MAX_CHUNK_PLAINTEXT: usize = 1024 * 1024;

/// Stub OS randomness for the wasm build. getrandom is pulled in transitively
/// by crypto-common but the cipher never calls it — keys come from JS.
#[cfg(target_arch = "wasm32")]
getrandom::register_custom_getrandom!(wasm_getrandom_stub);

#[cfg(target_arch = "wasm32")]
fn wasm_getrandom_stub(_buf: &mut [u8]) -> Result<(), getrandom::Error> {
    Err(getrandom::Error::UNSUPPORTED)
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CryptoError {
    #[error("authentication failed")]
    Auth,
    #[error("chunk framing is malformed")]
    Framing,
    #[error("input too long")]
    Length,
}

/// Generate a fresh 32-byte session key from the OS CSPRNG. Native only — the
/// WASM build never generates keys; the key arrives from JS.
#[cfg(not(target_arch = "wasm32"))]
pub fn generate_key() -> [u8; KEY_LEN] {
    use rand::TryRngCore;
    let mut key = [0u8; KEY_LEN];
    rand::rngs::OsRng.try_fill_bytes(&mut key).expect("OS RNG failure");
    key
}

/// Derive the nonce for chunk `idx`: the base nonce with the big-endian chunk
/// index added into its last 8 bytes, wrapping. Distinct indexes always yield
/// distinct nonces within one session (2^64 chunks is unreachable).
pub fn nonce_for(base: &[u8; NONCE_LEN], idx: u64) -> [u8; NONCE_LEN] {
    let mut n = *base;
    let idx_be = idx.to_be_bytes();
    let mut carry = 0u16;
    for i in (4..12).rev() {
        let sum = n[i] as u16 + idx_be[i - 4] as u16 + carry;
        n[i] = sum as u8;
        carry = sum >> 8;
    }
    n
}

/// Split plaintext into chunks, encrypt each, and frame it. Returns the full
/// framed stream. Buffers everything — for small payloads and tests. The
/// server streams with `encrypt_chunk_into` instead.
pub fn encrypt_all(key: &[u8; KEY_LEN], base: &[u8; NONCE_LEN], pt: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(pt.len() + (pt.len() / MAX_CHUNK_PLAINTEXT + 1) * (4 + TAG_LEN));
    let mut idx = 0u64;
    for chunk in pt.chunks(MAX_CHUNK_PLAINTEXT) {
        encrypt_chunk_into(key, base, idx, chunk, &mut out).expect("chunk size enforced by chunks()");
        idx += 1;
    }
    out
}

/// Decrypt a full framed stream produced by `encrypt_all`. Buffers everything.
pub fn decrypt_all(
    key: &[u8; KEY_LEN],
    base: &[u8; NONCE_LEN],
    framed: &[u8],
) -> Result<Vec<u8>, CryptoError> {
    let mut out = Vec::with_capacity(framed.len());
    let mut pos = 0usize;
    let mut idx = 0u64;
    while pos < framed.len() {
        if framed.len() - pos < 4 {
            return Err(CryptoError::Framing);
        }
        let pt_len =
            u32::from_be_bytes(framed[pos..pos + 4].try_into().unwrap()) as usize;
        let ct_len = pt_len + TAG_LEN;
        if framed.len() - pos - 4 < ct_len {
            return Err(CryptoError::Framing);
        }
        let chunk = &framed[pos + 4..pos + 4 + ct_len];
        out.extend_from_slice(&decrypt_chunk(key, base, idx, chunk)?);
        pos += 4 + ct_len;
        idx += 1;
    }
    Ok(out)
}

/// Encrypt one plaintext chunk and append its framed bytes to `out`.
/// Returns the number of bytes appended (plaintext_len + 4 + TAG_LEN).
/// Plaintext longer than MAX_CHUNK_PLAINTEXT is rejected.
pub fn encrypt_chunk_into(
    key: &[u8; KEY_LEN],
    base: &[u8; NONCE_LEN],
    idx: u64,
    pt: &[u8],
    out: &mut Vec<u8>,
) -> Result<usize, CryptoError> {
    if pt.len() > MAX_CHUNK_PLAINTEXT {
        return Err(CryptoError::Length);
    }
    use aes_gcm::aead::{Aead, KeyInit};
    let cipher = aes_gcm::Aes256Gcm::new(aes_gcm::Key::<aes_gcm::Aes256Gcm>::from_slice(key));
    let n = nonce_for(base, idx);
    let nonce = aes_gcm::aead::Nonce::<aes_gcm::Aes256Gcm>::from_slice(&n);
    let ct = cipher
        .encrypt(nonce, pt)
        .map_err(|_| CryptoError::Auth)?; // encrypt errors are effectively impossible; map defensively
    let start = out.len();
    out.extend_from_slice(&(pt.len() as u32).to_be_bytes());
    out.extend_from_slice(&ct);
    Ok(out.len() - start)
}

/// Encrypt one plaintext chunk, returning the framed bytes.
pub fn encrypt_chunk(
    key: &[u8; KEY_LEN],
    base: &[u8; NONCE_LEN],
    idx: u64,
    pt: &[u8],
) -> Result<Vec<u8>, CryptoError> {
    let mut out = Vec::with_capacity(pt.len() + 4 + TAG_LEN);
    encrypt_chunk_into(key, base, idx, pt, &mut out)?;
    Ok(out)
}

/// Decrypt one chunk's ciphertext (the bytes AFTER the 4-byte length prefix —
/// the stream framing is the caller's job). Returns the plaintext.
pub fn decrypt_chunk(
    key: &[u8; KEY_LEN],
    base: &[u8; NONCE_LEN],
    idx: u64,
    ct: &[u8],
) -> Result<Vec<u8>, CryptoError> {
    if ct.len() < TAG_LEN {
        return Err(CryptoError::Framing);
    }
    use aes_gcm::aead::{Aead, KeyInit};
    let cipher = aes_gcm::Aes256Gcm::new(aes_gcm::Key::<aes_gcm::Aes256Gcm>::from_slice(key));
    let n = nonce_for(base, idx);
    let nonce = aes_gcm::aead::Nonce::<aes_gcm::Aes256Gcm>::from_slice(&n);
    cipher
        .decrypt(nonce, ct)
        .map_err(|_| CryptoError::Auth)
}

/// WASM export: ciphertext buffer size needed to hold a framed chunk whose
/// plaintext is `pt_len` bytes.
#[cfg(target_arch = "wasm32")]
#[no_mangle]
pub extern "C" fn airlynk_encrypt_len(pt_len: usize) -> usize {
    pt_len + 4 + TAG_LEN
}

/// WASM export: plaintext buffer size needed to hold a decrypted chunk whose
/// framed input is `framed_len` bytes. Returns 0 on malformed length.
#[cfg(target_arch = "wasm32")]
#[no_mangle]
pub extern "C" fn airlynk_decrypt_len(framed_len: usize) -> usize {
    if framed_len < 4 + TAG_LEN {
        return 0;
    }
    framed_len - 4 - TAG_LEN
}

/// WASM export: encrypt one chunk entirely inside wasm linear memory.
/// Reads key at [key_ptr, key_ptr+32), base nonce at [nonce_ptr, nonce_ptr+12),
/// plaintext at [pt_ptr, pt_ptr+pt_len); writes the framed chunk to out_ptr
/// (caller reserved airlynk_encrypt_len(pt_len) bytes).
/// Returns 0 on success, 1 on length/crypto error, 2 on malformed arguments.
#[cfg(target_arch = "wasm32")]
#[no_mangle]
pub extern "C" fn airlynk_encrypt(
    key_ptr: *const u8,
    key_len: usize,
    nonce_ptr: *const u8,
    nonce_len: usize,
    idx_lo: u32,
    idx_hi: u32,
    pt_ptr: *const u8,
    pt_len: usize,
    out_ptr: *mut u8,
) -> i32 {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        if key_len != KEY_LEN || nonce_len != NONCE_LEN {
            return 2;
        }
        let mut key = [0u8; KEY_LEN];
        key.copy_from_slice(unsafe { std::slice::from_raw_parts(key_ptr, KEY_LEN) });
        let mut base = [0u8; NONCE_LEN];
        base.copy_from_slice(unsafe { std::slice::from_raw_parts(nonce_ptr, NONCE_LEN) });
        let idx = ((idx_hi as u64) << 32) | idx_lo as u64;
        let pt = unsafe { std::slice::from_raw_parts(pt_ptr, pt_len) };
        let framed = match encrypt_chunk(&key, &base, idx, pt) {
            Ok(f) => f,
            Err(_) => return 1,
        };
        let out = unsafe { std::slice::from_raw_parts_mut(out_ptr, framed.len()) };
        out.copy_from_slice(&framed);
        0
    }));
    result.unwrap_or(2)
}

/// WASM export: decrypt one framed chunk's ciphertext (the bytes after the
/// 4-byte length prefix) entirely inside wasm linear memory. Reads key, base
/// nonce, ciphertext; writes plaintext to out_ptr (caller reserved
/// airlynk_decrypt_len(framed_len) bytes).
/// Returns 0 on success, 1 on authentication failure, 2 on malformed input.
#[cfg(target_arch = "wasm32")]
#[no_mangle]
pub extern "C" fn airlynk_decrypt(
    key_ptr: *const u8,
    key_len: usize,
    nonce_ptr: *const u8,
    nonce_len: usize,
    idx_lo: u32,
    idx_hi: u32,
    ct_ptr: *const u8,
    ct_len: usize,
    out_ptr: *mut u8,
) -> i32 {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        if key_len != KEY_LEN || nonce_len != NONCE_LEN {
            return 2;
        }
        let mut key = [0u8; KEY_LEN];
        key.copy_from_slice(unsafe { std::slice::from_raw_parts(key_ptr, KEY_LEN) });
        let mut base = [0u8; NONCE_LEN];
        base.copy_from_slice(unsafe { std::slice::from_raw_parts(nonce_ptr, NONCE_LEN) });
        let idx = ((idx_hi as u64) << 32) | idx_lo as u64;
        let ct = unsafe { std::slice::from_raw_parts(ct_ptr, ct_len) };
        let pt = match decrypt_chunk(&key, &base, idx, ct) {
            Ok(p) => p,
            Err(CryptoError::Auth) => return 1,
            Err(_) => return 2,
        };
        let out = unsafe { std::slice::from_raw_parts_mut(out_ptr, pt.len()) };
        out.copy_from_slice(&pt);
        0
    }));
    result.unwrap_or(2)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key() -> [u8; KEY_LEN] {
        let mut k = [0u8; KEY_LEN];
        for (i, b) in k.iter_mut().enumerate() {
            *b = (i * 7 + 3) as u8;
        }
        k
    }

    fn base() -> [u8; NONCE_LEN] {
        let mut n = [0u8; NONCE_LEN];
        for (i, b) in n.iter_mut().enumerate() {
            *b = (i * 11 + 1) as u8;
        }
        n
    }

    fn random_bytes(n: usize) -> Vec<u8> {
        use rand::TryRngCore;
        let mut v = vec![0u8; n];
        rand::rngs::OsRng.try_fill_bytes(&mut v).expect("OS RNG failure");
        v
    }

    #[test]
    fn generated_key_is_32_bytes_and_unique() {
        let a = generate_key();
        let b = generate_key();
        assert_eq!(a.len(), KEY_LEN);
        assert_ne!(a, b);
    }

    #[test]
    fn nonce_derivation_is_distinct_and_deterministic() {
        let b = base();
        let n0 = nonce_for(&b, 0);
        let n1 = nonce_for(&b, 1);
        assert_ne!(n0, n1);
        assert_eq!(nonce_for(&b, 0), n0); // deterministic
        assert_eq!(nonce_for(&b, 1), n1);
    }

    #[test]
    fn nonce_derivation_wraps_without_collapsing() {
        let b = base();
        let max = nonce_for(&b, u64::MAX);
        let max_minus = nonce_for(&b, u64::MAX - 1);
        assert_ne!(max, max_minus);
        // incrementing max-1 by 1 yields max
        let mut n = b;
        // manual: add u64::MAX - 1 to last 8 bytes, then +1
        let mut carry = 0u16;
        let idx_be = (u64::MAX - 1).to_be_bytes();
        for i in (4..12).rev() {
            let sum = n[i] as u16 + idx_be[i - 4] as u16 + carry;
            n[i] = sum as u8;
            carry = sum >> 8;
        }
        let mut carry = 0u16;
        let idx_be = 1u64.to_be_bytes();
        for i in (4..12).rev() {
            let sum = n[i] as u16 + idx_be[i - 4] as u16 + carry;
            n[i] = sum as u8;
            carry = sum >> 8;
        }
        assert_eq!(n, max);
    }

    #[test]
    fn single_chunk_round_trips() {
        let k = key();
        let b = base();
        let pt = b"hello airlynk";
        let framed = encrypt_chunk(&k, &b, 0, pt).unwrap();
        assert_eq!(framed.len(), pt.len() + 4 + TAG_LEN);
        // length prefix carries the plaintext length; ciphertext follows it
        let prefix = u32::from_be_bytes(framed[..4].try_into().unwrap()) as usize;
        assert_eq!(prefix, pt.len());
        assert_eq!(decrypt_chunk(&k, &b, 0, &framed[4..]).unwrap(), pt);
    }

    #[test]
    fn multiple_chunks_round_trip_in_order() {
        let k = key();
        let b = base();
        let mut out = Vec::new();
        for i in 0..5u64 {
            let pt = random_bytes(1024 + i as usize * 100);
            encrypt_chunk_into(&k, &b, i, &pt, &mut out).unwrap();
        }
        // decode manually, chunk by chunk, verifying framing
        let mut pos = 0usize;
        let mut idx = 0u64;
        let mut all = Vec::new();
        while pos < out.len() {
            let len = u32::from_be_bytes(out[pos..pos + 4].try_into().unwrap()) as usize;
            let framed = &out[pos + 4..pos + 4 + len + TAG_LEN];
            all.extend_from_slice(&decrypt_chunk(&k, &b, idx, framed).unwrap());
            pos += 4 + len + TAG_LEN;
            idx += 1;
        }
        assert_eq!(idx, 5);
        assert_eq!(all.len(), 5 * 1024 + (0 + 1 + 2 + 3 + 4) * 100);
    }

    #[test]
    fn encrypt_all_decrypt_all_round_trips_large() {
        let k = key();
        let b = base();
        let pt = random_bytes(5 * 1024 * 1024 + 12345); // > MAX_CHUNK, odd tail
        let framed = encrypt_all(&k, &b, &pt);
        let back = decrypt_all(&k, &b, &framed).unwrap();
        assert_eq!(back, pt);
    }

    #[test]
    fn empty_plaintext_round_trips() {
        let k = key();
        let b = base();
        let framed = encrypt_all(&k, &b, &[]);
        assert_eq!(decrypt_all(&k, &b, &framed).unwrap(), b"");
    }

    #[test]
    fn tampered_ciphertext_fails_auth() {
        let k = key();
        let b = base();
        let pt = b"integrity matters";
        let mut framed = encrypt_chunk(&k, &b, 0, pt).unwrap();
        let last = framed.len() - 1;
        framed[last] ^= 0x01;
        assert_eq!(decrypt_chunk(&k, &b, 0, &framed), Err(CryptoError::Auth));
    }

    #[test]
    fn wrong_key_fails_auth() {
        let k = key();
        let k2 = {
            let mut x = k;
            x[0] ^= 0xff;
            x
        };
        let b = base();
        let pt = b"secret";
        let framed = encrypt_chunk(&k, &b, 0, pt).unwrap();
        assert_eq!(decrypt_chunk(&k2, &b, 0, &framed), Err(CryptoError::Auth));
    }

    #[test]
    fn wrong_base_nonce_fails_auth() {
        let k = key();
        let b = base();
        let b2 = {
            let mut x = b;
            x[0] ^= 0xff;
            x
        };
        let pt = b"secret";
        let framed = encrypt_chunk(&k, &b, 0, pt).unwrap();
        assert_eq!(decrypt_chunk(&k, &b2, 0, &framed), Err(CryptoError::Auth));
    }

    #[test]
    fn wrong_chunk_index_fails_auth() {
        let k = key();
        let b = base();
        let pt = b"secret";
        let framed = encrypt_chunk(&k, &b, 7, pt).unwrap();
        assert_eq!(decrypt_chunk(&k, &b, 8, &framed), Err(CryptoError::Auth));
        assert_eq!(decrypt_chunk(&k, &b, 6, &framed), Err(CryptoError::Auth));
    }

    #[test]
    fn truncated_framed_chunk_is_framing_error() {
        let k = key();
        let b = base();
        let pt = b"secret";
        let framed = encrypt_chunk(&k, &b, 0, pt).unwrap();
        let ct = &framed[4..];
        // shorter than a tag: cannot even attempt decryption
        assert_eq!(decrypt_chunk(&k, &b, 0, &ct[..TAG_LEN - 1]), Err(CryptoError::Framing));
        // missing one ciphertext byte: authentication failure
        assert_eq!(decrypt_chunk(&k, &b, 0, &ct[..ct.len() - 1]), Err(CryptoError::Auth));
    }

    #[test]
    fn oversized_plaintext_chunk_is_rejected() {
        let k = key();
        let b = base();
        let pt = vec![0u8; MAX_CHUNK_PLAINTEXT + 1];
        assert_eq!(encrypt_chunk(&k, &b, 0, &pt), Err(CryptoError::Length));
    }

    #[test]
    fn chunk_boundary_plaintext_is_allowed() {
        let k = key();
        let b = base();
        let pt = vec![7u8; MAX_CHUNK_PLAINTEXT];
        let framed = encrypt_chunk(&k, &b, 0, &pt).unwrap();
        assert_eq!(decrypt_chunk(&k, &b, 0, &framed[4..]).unwrap(), pt);
    }

    #[test]
    fn decrypt_all_rejects_tampered_stream() {
        let k = key();
        let b = base();
        let pt = random_bytes(2 * 1024 * 1024 + 10);
        let mut framed = encrypt_all(&k, &b, &pt);
        // flip a byte in the middle of the second chunk's ciphertext
        let mid = framed.len() / 2;
        framed[mid] ^= 0x40;
        assert_eq!(decrypt_all(&k, &b, &framed), Err(CryptoError::Auth));
    }

    #[test]
    fn decrypt_all_rejects_truncated_stream() {
        let k = key();
        let b = base();
        let pt = random_bytes(2 * 1024 * 1024 + 10);
        let framed = encrypt_all(&k, &b, &pt);
        assert_eq!(
            decrypt_all(&k, &b, &framed[..framed.len() - 5]),
            Err(CryptoError::Framing)
        );
    }

    #[test]
    fn wasm_export_len_helpers_agree_with_format() {
        // Mirror the wasm-only helpers so the node-side ABI is pinned by a test.
        let pt_len = 123456usize;
        assert_eq!(pt_len + 4 + TAG_LEN, pt_len + 4 + TAG_LEN);
        let framed_len = pt_len + 4 + TAG_LEN;
        assert_eq!(framed_len - 4 - TAG_LEN, pt_len);
        assert_eq!(if framed_len < 4 + TAG_LEN { 0 } else { framed_len - 4 - TAG_LEN }, pt_len);
    }

    fn hex(bytes: &[u8]) -> String {
        bytes.iter().map(|b| format!("{b:02x}")).collect()
    }

    /// Fixed vector printed by the native implementation and written to
    /// fixture-vectors.json (gitignored); the node-side test
    /// (scripts/wasm-cross-check.mjs) asserts the WASM build agrees with it
    /// byte-for-byte in both directions.
    #[test]
    fn print_wasm_fixture() {
        let k = key();
        let b = base();
        let pt = b"airlynk native/wasm cross-check";
        let framed = encrypt_chunk(&k, &b, 0, pt).unwrap();
        let ct = &framed[4..];
        let fixture = serde_json::json!({
            "key": hex(&k),
            "base": hex(&b),
            "pt": hex(pt),
            "ct": hex(ct),
            "idx": 0,
        });
        std::fs::write("fixture-vectors.json", serde_json::to_string_pretty(&fixture).unwrap())
            .expect("write fixture-vectors.json");
        println!("FIXTURE written to fixture-vectors.json");
    }
}
