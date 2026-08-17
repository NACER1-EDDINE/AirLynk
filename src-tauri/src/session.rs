//! Session registry — the single source of truth for live sessions (SEC-1..3,
//! SEC-9, SEC-10, FR-14..17, FR-26, FR-27).
//!
//! The Rust side owns all state; the webview holds display state only. Tokens
//! are 256-bit CSPRNG values, URL-safe base64, compared in constant time.
//! Sessions carry the AES key, the base nonce, the short human-comparable
//! display code, the file manifest, upload caps, and activity timestamps.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};

use airlynk_crypto::{generate_key, NONCE_LEN};
use subtle::ConstantTimeEq;

/// Unambiguous display alphabet — no 0/O, no 1/I/L (FR-26).
pub const CODE_ALPHABET: &[u8; 32] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";

pub const DEFAULT_INACTIVITY_TIMEOUT: Duration = Duration::from_secs(10 * 60);

/// Per-session upload caps (SEC-10).
#[derive(Debug, Clone, Copy)]
pub struct UploadCaps {
    pub max_files: u32,
    pub max_bytes: u64,
}

impl Default for UploadCaps {
    fn default() -> Self {
        Self {
            max_files: 500,
            max_bytes: 64 * 1024 * 1024 * 1024, // 64 GiB per session
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionKind {
    Send,
    Receive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileStatus {
    Pending,
    Active,
    Done,
    Failed,
}

/// One file in a session manifest. For Send sessions `path` is the source
/// file to stream; for Receive sessions it is the quarantined upload that
/// moves to Downloads on success.
#[derive(Debug)]
pub struct SessionFile {
    pub id: u32,
    pub original_name: String,
    pub size: u64,
    pub path: Option<PathBuf>,
    pub sent_bytes: AtomicU64,
    pub status: Mutex<FileStatus>,
}

impl SessionFile {
    pub fn new(id: u32, name: impl Into<String>, size: u64, path: Option<PathBuf>) -> Self {
        Self {
            id,
            original_name: name.into(),
            size,
            path,
            sent_bytes: AtomicU64::new(0),
            status: Mutex::new(FileStatus::Pending),
        }
    }
}

impl Clone for SessionFile {
    /// Snapshot clone: atomics and locks are copied by value.
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            original_name: self.original_name.clone(),
            size: self.size,
            path: self.path.clone(),
            sent_bytes: AtomicU64::new(self.sent_bytes.load(Ordering::Relaxed)),
            status: Mutex::new(*self.status.lock().unwrap()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SessionError {
    #[error("session cap exceeded")]
    CapExceeded,
}

pub struct Session {
    pub id: u64,
    /// URL-safe base64 of the 256-bit token (SEC-1). Never shown to users.
    pub token: String,
    /// Short human-comparable code rendered identically on both devices (FR-26).
    pub display_code: String,
    pub kind: SessionKind,
    /// AES-256 session key; travels only in the QR URL fragment (SEC-13).
    pub key: [u8; 32],
    /// Base nonce for chunk encryption.
    pub base_nonce: [u8; NONCE_LEN],
    pub caps: RwLock<UploadCaps>,
    files: RwLock<Vec<SessionFile>>,
    upload_count: AtomicU64,
    upload_bytes: AtomicU64,
    cancelled: AtomicBool,
    last_activity: Mutex<Instant>,
    created: Instant,
}

impl Session {
    /// A token is valid only if it matches a live session; mark activity so
    /// the expiry sweep does not reap an in-use session (SEC-9).
    pub fn touch(&self) {
        if let Ok(mut t) = self.last_activity.lock() {
            *t = Instant::now();
        }
    }

    pub fn last_activity(&self) -> Instant {
        self.last_activity.lock().map(|t| *t).unwrap_or(self.created)
    }

    pub fn is_expired(&self, now: Instant, timeout: Duration) -> bool {
        now.duration_since(self.last_activity()) > timeout
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Relaxed)
    }

    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Relaxed);
        self.touch();
    }

    pub fn files(&self) -> Vec<SessionFile> {
        self.files.read().unwrap().clone()
    }

    pub fn file_by_id(&self, id: u32) -> Option<SessionFile> {
        self.files
            .read()
            .unwrap()
            .iter()
            .find(|f| f.id == id)
            .cloned()
    }

    pub fn set_file_status(&self, id: u32, status: FileStatus) {
        let files = self.files.read().unwrap();
        if let Some(f) = files.iter().find(|f| f.id == id) {
            *f.status.lock().unwrap() = status;
        }
    }

    pub fn set_file_sent(&self, id: u32, bytes: u64) {
        let files = self.files.read().unwrap();
        if let Some(f) = files.iter().find(|f| f.id == id) {
            f.sent_bytes.store(bytes, Ordering::Relaxed);
        }
    }

    /// Register an upload (quarantine path + size). Enforces per-session caps
    /// (SEC-10) and returns the file id.
    pub fn register_upload(&self, name: &str, size: u64, path: PathBuf) -> Result<u32, SessionError> {
        let caps = *self.caps.read().unwrap();
        // File-count cap, taken atomically so concurrent uploads cannot both
        // exceed it.
        let id = loop {
            let cur = self.upload_count.load(Ordering::Relaxed);
            if cur >= caps.max_files as u64 {
                return Err(SessionError::CapExceeded);
            }
            match self.upload_count.compare_exchange(
                cur,
                cur + 1,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break cur as u32,
                Err(_) => continue,
            }
        };
        // Byte cap. If it fails, the id is left consumed — harmless: caps are
        // soft limits for a hostile client, not an accounting boundary.
        self.reserve_bytes(size)?;
        self.files.write().unwrap().push(SessionFile::new(id, name, size, Some(path)));
        self.touch();
        Ok(id)
    }

    pub fn upload_bytes(&self) -> u64 {
        self.upload_bytes.load(Ordering::Relaxed)
    }

    /// Atomically reserve `size` bytes against the per-session cap.
    fn reserve_bytes(&self, size: u64) -> Result<(), SessionError> {
        let caps = *self.caps.read().unwrap();
        let current = self.upload_bytes.load(Ordering::Relaxed);
        if current + size > caps.max_bytes {
            return Err(SessionError::CapExceeded);
        }
        match self.upload_bytes.compare_exchange(
            current,
            current + size,
            Ordering::Relaxed,
            Ordering::Relaxed,
        ) {
            Ok(_) => Ok(()),
            Err(_) => self.reserve_bytes(size), // retry on contention
        }
    }
}

pub struct SessionRegistry {
    sessions: RwLock<Vec<Arc<Session>>>,
    next_id: AtomicU64,
    timeout: Duration,
}

impl SessionRegistry {
    pub fn new(timeout: Duration) -> Self {
        Self {
            sessions: RwLock::new(Vec::new()),
            next_id: AtomicU64::new(1),
            timeout,
        }
    }

    /// Create a Send session from a manifest of source files.
    pub fn create_send_session(&self, files: Vec<(String, u64, PathBuf)>) -> Arc<Session> {
        self.spawn(|id| {
            let mut manifest = Vec::with_capacity(files.len());
            for (i, (name, size, path)) in files.into_iter().enumerate() {
                manifest.push(SessionFile::new(i as u32, name, size, Some(path)));
            }
            (SessionKind::Send, manifest, UploadCaps::default())
        })
    }

    /// Create a Receive session (files arrive via upload).
    pub fn create_receive_session(&self) -> Arc<Session> {
        self.spawn(|_| (SessionKind::Receive, Vec::new(), UploadCaps::default()))
    }

    /// Shared construction: generate a unique token + display code, build the
    /// session, and register it under the write lock.
    fn spawn(
        &self,
        build: impl FnOnce(u64) -> (SessionKind, Vec<SessionFile>, UploadCaps),
    ) -> Arc<Session> {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let (kind, files, caps) = build(id);
        let mut sessions = self.sessions.write().unwrap();
        // Display codes are short (2^20 space); regenerate on collision.
        let (token, display_code) = loop {
            let token = Self::new_token();
            let code = Self::display_code_from(&token);
            if !sessions.iter().any(|s| s.display_code == code) {
                break (token, code);
            }
        };
        let session = Arc::new(Session {
            id,
            token,
            display_code,
            kind,
            key: generate_key(),
            base_nonce: random_nonce(),
            caps: RwLock::new(caps),
            files: RwLock::new(files),
            upload_count: AtomicU64::new(0),
            upload_bytes: AtomicU64::new(0),
            cancelled: AtomicBool::new(false),
            last_activity: Mutex::new(Instant::now()),
            created: Instant::now(),
        });
        sessions.push(session.clone());
        session
    }

    /// Constant-time lookup by full token string (SEC-3). Touches activity.
    pub fn find_by_token(&self, token: &str) -> Option<Arc<Session>> {
        let sessions = self.sessions.read().unwrap();
        let token_bytes = token.as_bytes();
        for s in sessions.iter() {
            if s.token.as_bytes().ct_eq(token_bytes).into() {
                s.touch();
                return Some(s.clone());
            }
        }
        None
    }

    pub fn remove(&self, session: &Session) -> bool {
        let mut sessions = self.sessions.write().unwrap();
        let before = sessions.len();
        sessions.retain(|s| s.id != session.id);
        sessions.len() != before
    }

    pub fn is_empty(&self) -> bool {
        self.sessions.read().unwrap().is_empty()
    }

    pub fn len(&self) -> usize {
        self.sessions.read().unwrap().len()
    }

    /// Remove sessions idle longer than the registry timeout. Returns the
    /// removed session ids so the server can be shut down with the last one.
    pub fn prune_expired(&self) -> Vec<u64> {
        let now = Instant::now();
        let mut sessions = self.sessions.write().unwrap();
        let mut expired = Vec::new();
        sessions.retain(|s| {
            let dead = s.is_expired(now, self.timeout);
            if dead {
                expired.push(s.id);
            }
            !dead
        });
        expired
    }

    /// Generate a 256-bit CSPRNG token as URL-safe base64 (SEC-1).
    fn new_token() -> String {
        use rand::TryRngCore;
        use base64::Engine;
        let mut bytes = [0u8; 32];
        rand::rngs::OsRng
            .try_fill_bytes(&mut bytes)
            .expect("OS RNG failure");
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
    }

    /// Derive a display code from the token: 4 chars from the unambiguous
    /// alphabet, rendered as `XX-XX` (FR-26). 32 chars × 4 = 2^20 combos; the
    /// registry checks for collisions.
    fn display_code_from(token: &str) -> String {
        use base64::Engine;
        let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(token)
            .expect("token is always valid base64");
        let pick = |i: usize| CODE_ALPHABET[(bytes[i] as usize) % CODE_ALPHABET.len()] as char;
        format!("{}{}-{}{}", pick(0), pick(1), pick(2), pick(3))
    }
}

/// Fresh 12-byte base nonce for chunk encryption (native only).
fn random_nonce() -> [u8; NONCE_LEN] {
    use rand::TryRngCore;
    let mut nonce = [0u8; NONCE_LEN];
    rand::rngs::OsRng
        .try_fill_bytes(&mut nonce)
        .expect("OS RNG failure");
    nonce
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp_dir(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("airlynk-session-{tag}-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn tokens_are_256_bit_urlsafe_and_unique() {
        let a = SessionRegistry::new_token();
        let b = SessionRegistry::new_token();
        assert_eq!(a.len(), 43); // 32 bytes -> 43 base64url chars, no padding
        assert!(a.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'));
        assert_ne!(a, b);
    }

    #[test]
    fn display_code_uses_only_unambiguous_alphabet() {
        let reg = SessionRegistry::new(DEFAULT_INACTIVITY_TIMEOUT);
        let s = reg.create_receive_session();
        let code = &s.display_code;
        assert_eq!(code.len(), 5); // XX-XX
        assert_eq!(code.as_bytes()[2], b'-');
        for c in code.chars().filter(|c| *c != '-') {
            assert!(
                CODE_ALPHABET.contains(&(c as u8)),
                "forbidden character in display code: {c}"
            );
        }
        assert!(!code.contains('0') && !code.contains('O') && !code.contains('1') && !code.contains('I') && !code.contains('L'));
    }

    #[test]
    fn send_session_manifest_round_trips() {
        let reg = SessionRegistry::new(DEFAULT_INACTIVITY_TIMEOUT);
        let dir = tmp_dir("send");
        let p1 = dir.join("a.txt");
        let p2 = dir.join("b.txt");
        std::fs::write(&p1, b"aaa").unwrap();
        std::fs::write(&p2, b"bbbb").unwrap();
        let s = reg.create_send_session(vec![
            ("a.txt".into(), 3, p1.clone()),
            ("b.txt".into(), 4, p2.clone()),
        ]);
        assert_eq!(s.kind, SessionKind::Send);
        let files = s.files();
        assert_eq!(files.len(), 2);
        assert_eq!(files[0].original_name, "a.txt");
        assert_eq!(files[0].size, 3);
        assert_eq!(files[0].path.as_deref(), Some(p1.as_path()));
    }

    #[test]
    fn find_by_token_round_trips_and_unknown_returns_none() {
        let reg = SessionRegistry::new(DEFAULT_INACTIVITY_TIMEOUT);
        let s = reg.create_receive_session();
        let found = reg.find_by_token(&s.token).expect("session should be found");
        assert_eq!(found.id, s.id);
        assert_eq!(found.token, s.token);
        assert!(reg.find_by_token("AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA").is_none());
    }

    #[test]
    fn find_among_many_sessions_returns_the_right_one() {
        let reg = SessionRegistry::new(DEFAULT_INACTIVITY_TIMEOUT);
        let mut tokens = Vec::new();
        for _ in 0..50 {
            let s = reg.create_receive_session();
            tokens.push(s.token.clone());
        }
        for t in &tokens {
            let found = reg.find_by_token(t).expect("each token findable");
            assert_eq!(&found.token, t);
        }
        assert_eq!(reg.len(), 50);
    }

    #[test]
    fn expiry_prunes_idle_sessions_only() {
        let reg = SessionRegistry::new(Duration::from_millis(50));
        let s = reg.create_receive_session();
        std::thread::sleep(Duration::from_millis(80));
        let pruned = reg.prune_expired();
        assert!(pruned.contains(&s.id));
        assert!(reg.is_empty());
    }

    #[test]
    fn touch_keeps_session_alive() {
        let reg = SessionRegistry::new(Duration::from_millis(60));
        let s = reg.create_receive_session();
        std::thread::sleep(Duration::from_millis(40));
        s.touch();
        std::thread::sleep(Duration::from_millis(40));
        let pruned = reg.prune_expired();
        assert!(!pruned.contains(&s.id), "touched session must not expire");
    }

    #[test]
    fn remove_drops_session() {
        let reg = SessionRegistry::new(DEFAULT_INACTIVITY_TIMEOUT);
        let s = reg.create_receive_session();
        assert!(reg.remove(&s));
        assert!(reg.is_empty());
        assert!(reg.find_by_token(&s.token).is_none());
        assert!(!reg.remove(&s));
    }

    #[test]
    fn upload_caps_enforced_per_session() {
        let reg = SessionRegistry::new(DEFAULT_INACTIVITY_TIMEOUT);
        let s = reg.create_receive_session();
        let caps = UploadCaps {
            max_files: 2,
            max_bytes: 1000,
        };
        *s.caps.write().unwrap() = caps;
        let dir = tmp_dir("caps");
        let p = dir.join("q.bin");
        std::fs::write(&p, b"x").unwrap();
        let id1 = s.register_upload("one.bin", 400, p.clone()).unwrap();
        let id2 = s.register_upload("two.bin", 400, p.clone()).unwrap();
        assert_ne!(id1, id2);
        // file cap exceeded
        assert_eq!(s.register_upload("three.bin", 1, p.clone()), Err(SessionError::CapExceeded));
        // byte cap exceeded (reserve would exceed 1000)
        let s2 = reg.create_receive_session();
        *s2.caps.write().unwrap() = caps;
        s2.register_upload("a.bin", 600, p.clone()).unwrap();
        assert_eq!(s2.register_upload("b.bin", 500, p.clone()), Err(SessionError::CapExceeded));
    }

    #[test]
    fn cancel_flag_flips_and_touches() {
        let reg = SessionRegistry::new(DEFAULT_INACTIVITY_TIMEOUT);
        let s = reg.create_receive_session();
        assert!(!s.is_cancelled());
        s.cancel();
        assert!(s.is_cancelled());
    }

    #[test]
    fn file_status_and_progress_mutations() {
        let reg = SessionRegistry::new(DEFAULT_INACTIVITY_TIMEOUT);
        let dir = tmp_dir("status");
        let p = dir.join("f.bin");
        std::fs::write(&p, b"data").unwrap();
        let s = reg.create_send_session(vec![("f.bin".into(), 4, p)]);
        let id = s.files()[0].id;
        s.set_file_status(id, FileStatus::Active);
        s.set_file_sent(id, 4);
        s.set_file_status(id, FileStatus::Done);
        let f = s.file_by_id(id).unwrap();
        assert_eq!(*f.status.lock().unwrap(), FileStatus::Done);
        assert_eq!(f.sent_bytes.load(Ordering::Relaxed), 4);
        assert!(s.file_by_id(9999).is_none());
    }

    #[test]
    fn display_code_is_deterministic_per_token() {
        let reg = SessionRegistry::new(DEFAULT_INACTIVITY_TIMEOUT);
        let s = reg.create_receive_session();
        let c1 = SessionRegistry::display_code_from(&s.token);
        let c2 = SessionRegistry::display_code_from(&s.token);
        assert_eq!(c1, c2);
        assert_eq!(c1, s.display_code);
    }
}
