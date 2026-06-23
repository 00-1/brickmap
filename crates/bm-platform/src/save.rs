//! bm-platform::save — a tiny **persistence abstraction** (engine save service). A game stores
//! opaque byte blobs (typically JSON) under short string keys; the backend differs per platform —
//! a file per key on native/Android, `localStorage` on the web — behind one [`Store`] trait so the
//! game never sees the difference (architecture §6: platform edges behind `cfg`).
//!
//! The blob format is the game's business; this is just durable key→bytes. Keys are sanitised to
//! a safe charset, so they can't escape the store (no path traversal).

use std::io;

/// A durable key→bytes store. Keys are short identifiers; values are opaque blobs (e.g. JSON).
pub trait Store {
    /// Load the blob stored under `key`, or `None` if there isn't one.
    fn load(&self, key: &str) -> Option<Vec<u8>>;
    /// Persist `bytes` under `key`, replacing any previous value.
    fn save(&self, key: &str, bytes: &[u8]) -> io::Result<()>;
    /// Remove `key` (a no-op if it's absent).
    fn remove(&self, key: &str) -> io::Result<()>;
}

/// Sanitise a key to a safe filename/slot: keep `[A-Za-z0-9_.-]`, map the rest to `_`. Prevents
/// path traversal (native) and keeps web slots tidy.
fn safe_key(key: &str) -> String {
    let s: String = key
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-') {
                c
            } else {
                '_'
            }
        })
        .collect();
    if s.is_empty() {
        "_".to_string()
    } else {
        s
    }
}

// ── native / Android: one file per key ───────────────────────────────────────────────────────

/// A filesystem-backed store: one `<key>.bin` file under `base`. Used on native and Android
/// (the caller passes the platform's writable app directory).
pub struct FileStore {
    base: std::path::PathBuf,
}

impl FileStore {
    /// Open a store at `base`, creating the directory if needed.
    pub fn open(base: impl Into<std::path::PathBuf>) -> io::Result<FileStore> {
        let base = base.into();
        std::fs::create_dir_all(&base)?;
        Ok(FileStore { base })
    }

    fn path(&self, key: &str) -> std::path::PathBuf {
        self.base.join(format!("{}.bin", safe_key(key)))
    }
}

impl Store for FileStore {
    fn load(&self, key: &str) -> Option<Vec<u8>> {
        std::fs::read(self.path(key)).ok()
    }

    fn save(&self, key: &str, bytes: &[u8]) -> io::Result<()> {
        // Write to a temp file then rename, so a crash mid-write can't truncate the saved blob.
        let path = self.path(key);
        let tmp = path.with_extension("tmp");
        std::fs::write(&tmp, bytes)?;
        std::fs::rename(&tmp, &path)
    }

    fn remove(&self, key: &str) -> io::Result<()> {
        match std::fs::remove_file(self.path(key)) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e),
        }
    }
}

// ── web: localStorage (bytes hex-encoded, since it only holds strings) ────────────────────────

/// Lowercase-hex encode (no dependency) — `localStorage` holds strings, so byte blobs round-trip
/// through hex.
#[cfg(target_arch = "wasm32")]
fn to_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut s = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        s.push(HEX[(b >> 4) as usize] as char);
        s.push(HEX[(b & 0xf) as usize] as char);
    }
    s
}

#[cfg(target_arch = "wasm32")]
fn from_hex(s: &str) -> Option<Vec<u8>> {
    let bytes = s.as_bytes();
    if !bytes.len().is_multiple_of(2) {
        return None;
    }
    let val = |c: u8| -> Option<u8> {
        match c {
            b'0'..=b'9' => Some(c - b'0'),
            b'a'..=b'f' => Some(c - b'a' + 10),
            b'A'..=b'F' => Some(c - b'A' + 10),
            _ => None,
        }
    };
    let mut out = Vec::with_capacity(bytes.len() / 2);
    for pair in bytes.chunks_exact(2) {
        out.push((val(pair[0])? << 4) | val(pair[1])?);
    }
    Some(out)
}

/// A `localStorage`-backed store. Keys are namespaced with `prefix` so several stores (or apps on
/// one origin) don't collide.
#[cfg(target_arch = "wasm32")]
pub struct WebStore {
    prefix: String,
}

#[cfg(target_arch = "wasm32")]
impl WebStore {
    /// Open a store namespaced under `prefix` (e.g. the app id).
    pub fn open(prefix: impl Into<String>) -> WebStore {
        WebStore {
            prefix: prefix.into(),
        }
    }

    fn storage(&self) -> Option<web_sys::Storage> {
        web_sys::window()?.local_storage().ok().flatten()
    }

    fn slot(&self, key: &str) -> String {
        format!("{}:{}", self.prefix, safe_key(key))
    }
}

#[cfg(target_arch = "wasm32")]
impl Store for WebStore {
    fn load(&self, key: &str) -> Option<Vec<u8>> {
        let s = self.storage()?;
        let hex = s.get_item(&self.slot(key)).ok().flatten()?;
        from_hex(&hex)
    }

    fn save(&self, key: &str, bytes: &[u8]) -> io::Result<()> {
        let s = self
            .storage()
            .ok_or_else(|| io::Error::other("no localStorage"))?;
        s.set_item(&self.slot(key), &to_hex(bytes))
            .map_err(|_| io::Error::other("localStorage set_item failed"))
    }

    fn remove(&self, key: &str) -> io::Result<()> {
        if let Some(s) = self.storage() {
            let _ = s.remove_item(&self.slot(key));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(tag: &str) -> std::path::PathBuf {
        let mut p = std::env::temp_dir();
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        p.push(format!("bm-save-{tag}-{nanos}"));
        p
    }

    #[test]
    fn file_store_round_trips_and_removes() {
        let dir = temp_dir("rt");
        let store = FileStore::open(&dir).expect("open");
        assert!(store.load("progress").is_none(), "absent key → None");

        store.save("progress", b"{\"solved\":12}").expect("save");
        assert_eq!(
            store.load("progress").as_deref(),
            Some(&b"{\"solved\":12}"[..])
        );

        // Overwrite replaces.
        store.save("progress", b"new").expect("save2");
        assert_eq!(store.load("progress").as_deref(), Some(&b"new"[..]));

        store.remove("progress").expect("remove");
        assert!(store.load("progress").is_none(), "removed key → None");
        store.remove("progress").expect("remove absent is a no-op");

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn keys_are_sanitised_so_they_cannot_escape_the_store() {
        let dir = temp_dir("safe");
        let store = FileStore::open(&dir).expect("open");
        // A traversal-looking key must stay inside `base` (mapped to underscores).
        store.save("../../etc/passwd", b"x").expect("save");
        // It round-trips under the same sanitised key…
        assert_eq!(store.load("../../etc/passwd").as_deref(), Some(&b"x"[..]));
        // …and only one file was created inside the store dir (no escape).
        let count = std::fs::read_dir(&dir).unwrap().count();
        assert_eq!(count, 1, "exactly one file, inside the store");
        std::fs::remove_dir_all(&dir).ok();
    }
}
