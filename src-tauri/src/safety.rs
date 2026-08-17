//! Filename sanitization and destination canonicalization (SEC-4, SEC-6).
//!
//! Every uploaded filename passes through `sanitize_filename` before it touches
//! the filesystem, and the final destination is verified inside Downloads via
//! `ensure_within`. Tests were written first (TDD) and enumerate the exact
//! hostile inputs from SEC-4.

use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SafetyError {
    #[error("filename is empty after sanitization")]
    EmptyName,
    #[error("destination escapes the allowed directory")]
    Escape,
    #[error("filesystem error: {0}")]
    Io(String),
}

/// Maximum filename component length we will ever create.
/// Windows NTFS allows 255 UTF-16 units; we stay under that in characters.
pub const MAX_NAME_CHARS: usize = 200;

/// Strip everything that could turn a name into a path or a Windows hazard.
///
/// Rules (SEC-4): drop path separators and `.`/`..` segments, keep only the
/// final component; remove drive-letter prefixes; strip control characters;
/// replace Windows-forbidden characters (`<>:"|?*`) with `_`; strip trailing
/// dots and spaces; prefix reserved device names (`CON`, `PRN`, `AUX`, `NUL`,
/// `COM1-9`, `LPT1-9`, with or without extension) with `_`; reject what is
/// empty or all dots after cleaning.
pub fn sanitize_filename(name: &str) -> Result<String, SafetyError> {
    // Normalize separators and split into segments; drop empties, "." and "..".
    let normalized = name.replace('\\', "/");
    let mut segments: Vec<&str> = normalized
        .split('/')
        .filter(|s| !s.is_empty() && *s != "." && *s != "..")
        .collect();

    // Keep only the final component: "a/b/c.txt" -> "c.txt".
    let mut cleaned = segments.pop().unwrap_or("").to_string();

    // Drive letter: "C:evil.txt" -> "evil.txt".
    if cleaned.len() >= 2 && cleaned.as_bytes()[1] == b':' {
        cleaned = cleaned[2..].to_string();
    }

    // Strip control characters; replace Windows-forbidden chars with '_'.
    cleaned = cleaned
        .chars()
        .map(|c| match c {
            '\u{0000}'..='\u{001f}' | '\u{007f}' => '_',
            '<' | '>' | '"' | ':' | '|' | '?' | '*' => '_',
            other => other,
        })
        .collect();

    // Trailing dots and spaces are illegal on Windows and can alias weirdly.
    cleaned = cleaned.trim_end_matches(['.', ' ']).to_string();

    if cleaned.is_empty() || cleaned.chars().all(|c| c == '.') {
        return Err(SafetyError::EmptyName);
    }

    // Reserved device names, case-insensitive, with or without extension.
    if reserved_device_name(&cleaned) {
        cleaned = format!("_{cleaned}");
    }

    // Hard cap on length so a hostile client cannot create absurd names.
    if cleaned.chars().count() > MAX_NAME_CHARS {
        cleaned = cleaned.chars().take(MAX_NAME_CHARS).collect();
    }

    Ok(cleaned)
}

/// True when the (already separator-free) name is a reserved Windows device
/// name: CON, PRN, AUX, NUL, COM1..COM9, LPT1..LPT9, case-insensitively,
/// with or without a single extension.
fn reserved_device_name(name: &str) -> bool {
    let stem = name.split('.').next().unwrap_or("");
    if !(3..=4).contains(&stem.len()) {
        return false;
    }
    let upper = stem.to_ascii_uppercase();
    match upper.as_str() {
        "CON" | "PRN" | "AUX" | "NUL" => true,
        _ => {
            if upper.len() != 4 {
                return false;
            }
            let (prefix, digit) = upper.split_at(3);
            digit.len() == 1
                && digit.chars().next().unwrap().is_ascii_digit()
                && matches!(prefix, "COM" | "LPT")
        }
    }
}

/// SEC-6: verify a destination path stays inside `root`, after resolving.
/// `name` must already be sanitized (no separators). We canonicalize the root
/// (resolving symlinks) and join the name, so a poisoned component cannot
/// climb out even if it slipped past sanitization.
pub fn ensure_within(root: &Path, name: &str) -> Result<PathBuf, SafetyError> {
    let canonical_root = root
        .canonicalize()
        .map_err(|e| SafetyError::Io(e.to_string()))?;
    let candidate = canonical_root.join(name);
    if !candidate.starts_with(&canonical_root) {
        return Err(SafetyError::Escape);
    }
    Ok(candidate)
}

/// FR-12: resolve a destination-name collision by suffixing —
/// `photo.jpg` → `photo (1).jpg` → `photo (2).jpg` — never overwriting.
/// `name` must already be sanitized. `exists()` is case-insensitive on NTFS,
/// matching the filesystem's own collision semantics. Returns the first path
/// that does not exist.
pub fn collision_safe_path(dir: &Path, name: &str) -> PathBuf {
    let candidate = dir.join(name);
    if !candidate.exists() {
        return candidate;
    }
    let (stem, ext) = match name.rfind('.') {
        Some(i) if i > 0 => (name[..i].to_string(), name[i..].to_string()),
        _ => (name.to_string(), String::new()),
    };
    for n in 1..10_000u32 {
        let candidate = dir.join(format!("{stem} ({n}){ext}"));
        if !candidate.exists() {
            return candidate;
        }
    }
    // Practically unreachable; keep the invariant "never overwrite" by
    // returning a name that cannot exist.
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    dir.join(format!("{stem} ({nanos}){ext}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static ROOT_SEQ: AtomicU64 = AtomicU64::new(0);

    /// Unique per test: parallel tests must never share a root, or one test's
    /// fixtures race another's assertions.
    fn tmp_root() -> PathBuf {
        let seq = ROOT_SEQ.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "airlynk-safety-{}-{seq}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn strips_path_separators_and_parent_segments() {
        assert_eq!(sanitize_filename("../../evil.txt").unwrap(), "evil.txt");
        assert_eq!(sanitize_filename("a/b/c.txt").unwrap(), "c.txt");
        assert_eq!(sanitize_filename(r"..\..\evil.txt").unwrap(), "evil.txt");
        assert_eq!(sanitize_filename("..\\sub\\photo.jpg").unwrap(), "photo.jpg");
    }

    #[test]
    fn strips_drive_letters() {
        assert_eq!(sanitize_filename(r"C:\evil").unwrap(), "evil");
        assert_eq!(sanitize_filename(r"C:evil.txt").unwrap(), "evil.txt");
        assert_eq!(sanitize_filename(r"D:\photos\IMG_1.jpg").unwrap(), "IMG_1.jpg");
    }

    #[test]
    fn neutralizes_ntfs_alternate_data_stream_colon() {
        assert_eq!(sanitize_filename("file.txt:ads").unwrap(), "file.txt_ads");
        assert_eq!(sanitize_filename(":hidden").unwrap(), "_hidden");
    }

    #[test]
    fn rejects_empty_and_all_dot_names() {
        assert!(matches!(sanitize_filename(""), Err(SafetyError::EmptyName)));
        assert!(matches!(sanitize_filename("..."), Err(SafetyError::EmptyName)));
        assert!(matches!(sanitize_filename(".."), Err(SafetyError::EmptyName)));
        assert!(matches!(sanitize_filename("/"), Err(SafetyError::EmptyName)));
        assert!(matches!(sanitize_filename("."), Err(SafetyError::EmptyName)));
    }

    #[test]
    fn strips_trailing_dots_and_spaces() {
        assert_eq!(sanitize_filename("photo.jpg   ").unwrap(), "photo.jpg");
        assert_eq!(sanitize_filename("name..").unwrap(), "name");
    }

    #[test]
    fn strips_control_characters() {
        assert_eq!(sanitize_filename("evil\u{0000}.txt").unwrap(), "evil_.txt");
        assert_eq!(sanitize_filename("a\u{0001}b.txt").unwrap(), "a_b.txt");
    }

    #[test]
    fn replaces_windows_forbidden_characters() {
        assert_eq!(sanitize_filename("a<b>c|d?e*f.txt").unwrap(), "a_b_c_d_e_f.txt");
        assert_eq!(sanitize_filename("quote\"name.txt").unwrap(), "quote_name.txt");
    }

    #[test]
    fn prefixes_reserved_device_names() {
        assert_eq!(sanitize_filename("CON").unwrap(), "_CON");
        assert_eq!(sanitize_filename("con.txt").unwrap(), "_con.txt");
        assert_eq!(sanitize_filename("NUL").unwrap(), "_NUL");
        assert_eq!(sanitize_filename("COM1.txt").unwrap(), "_COM1.txt");
        assert_eq!(sanitize_filename("LPT9").unwrap(), "_LPT9");
        assert_eq!(sanitize_filename("COM10.txt").unwrap(), "COM10.txt"); // not reserved
        assert_eq!(sanitize_filename("compose.txt").unwrap(), "compose.txt"); // not reserved
    }

    #[test]
    fn keeps_plain_names_untouched() {
        assert_eq!(sanitize_filename("holiday-photo (1).jpg").unwrap(), "holiday-photo (1).jpg");
        assert_eq!(sanitize_filename("IMG_4471.HEIC").unwrap(), "IMG_4471.HEIC");
        assert_eq!(sanitize_filename("日本語ファイル.txt").unwrap(), "日本語ファイル.txt");
    }

    #[test]
    fn caps_absurd_lengths() {
        let long = "x".repeat(10_000);
        let out = sanitize_filename(&long).unwrap();
        assert!(out.chars().count() <= MAX_NAME_CHARS);
    }

    #[test]
    fn keeps_last_component_of_mixed_hostile_input() {
        let hostile = "C:\\Users\\Public\\..\\..\\..\\Windows\\System32\\..\\evil.exe";
        assert_eq!(sanitize_filename(hostile).unwrap(), "evil.exe");
    }

    #[test]
    fn ensure_within_accepts_normal_name() {
        let root = tmp_root();
        let p = ensure_within(&root, "photo.jpg").unwrap();
        assert!(p.starts_with(root.canonicalize().unwrap()));
    }

    #[test]
    fn ensure_within_rejects_climbing_names() {
        let root = tmp_root();
        assert!(matches!(
            ensure_within(&root, "../../evil.txt"),
            Err(SafetyError::Escape)
        ));
        assert!(matches!(
            ensure_within(&root, r"..\..\evil.txt"),
            Err(SafetyError::Escape)
        ));
    }

    #[test]
    fn ensure_within_joins_without_touching_parent() {
        let root = tmp_root();
        let p = ensure_within(&root, "sub/../escape.txt").unwrap();
        // sanitized names never contain separators; a raw one must not resolve outside
        assert!(p.starts_with(root.canonicalize().unwrap()));
    }

    #[test]
    fn collision_safe_returns_original_when_free() {
        let root = tmp_root();
        let p = collision_safe_path(&root, "photo.jpg");
        assert_eq!(p, root.join("photo.jpg"));
    }

    #[test]
    fn collision_safe_suffixes_on_existing_name() {
        let root = tmp_root();
        std::fs::write(root.join("photo.jpg"), b"1").unwrap();
        let p = collision_safe_path(&root, "photo.jpg");
        assert_eq!(p, root.join("photo (1).jpg"));
    }

    #[test]
    fn collision_safe_increments_past_all_existing() {
        let root = tmp_root();
        for n in 0..3 {
            let name = if n == 0 {
                "clip.mp4".to_string()
            } else {
                format!("clip ({n}).mp4")
            };
            std::fs::write(root.join(name), b"x").unwrap();
        }
        let p = collision_safe_path(&root, "clip.mp4");
        assert_eq!(p, root.join("clip (3).mp4"));
    }

    #[test]
    fn collision_safe_handles_extensionless_names() {
        let root = tmp_root();
        std::fs::write(root.join("README"), b"x").unwrap();
        let p = collision_safe_path(&root, "README");
        assert_eq!(p, root.join("README (1)"));
    }

    #[test]
    fn collision_safe_never_returns_an_existing_path() {
        let root = tmp_root();
        for n in 0..5 {
            let name = if n == 0 {
                "data.bin".to_string()
            } else {
                format!("data ({n}).bin")
            };
            std::fs::write(root.join(&name), b"x").unwrap();
            let p = collision_safe_path(&root, "data.bin");
            assert!(
                !p.exists(),
                "candidate {p:?} must not exist — overwrite is forbidden"
            );
        }
    }

    #[test]
    fn collision_safe_treats_case_insensitively() {
        let root = tmp_root();
        std::fs::write(root.join("Photo.JPG"), b"x").unwrap();
        let p = collision_safe_path(&root, "photo.jpg");
        assert_eq!(p, root.join("photo (1).jpg"));
    }
}
