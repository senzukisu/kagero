//! Autosave and crash recovery.
//!
//! While a document has unsaved changes, Kagero writes a copy of it into a recovery
//! folder (`%APPDATA%\OnibiLab\Kagero\recovery`) — never next to, or over, the
//! original project. Each running Kagero is a *session* holding an exclusive lock on
//! its own lock file, so copies of a still-running Kagero are never offered for
//! recovery. Copies left behind with no live lock come from a crash.
//!
//! A copy is two files written atomically one after the other: `<session>.kgr` (the
//! document) and then `<session>.json` (what it is). A copy without its description is
//! incomplete and ignored. The session's copy is deleted after a successful save or
//! when the document is closed; old copies are trimmed by count and size, oldest
//! crashed copies first — never the running session's own copy.

use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::core::Document;
use crate::file_io::{self, write_atomic};

/// Identifier stored in every recovery description.
const META_FORMAT: &str = "kagero-recovery";
const META_VERSION: u32 = 1;
/// At most this many copies are kept (the running session's copy always counts).
pub const MAX_COPIES: usize = 10;
/// At most this many bytes of copies are kept.
pub const MAX_TOTAL_BYTES: u64 = 512 * 1024 * 1024;
/// Largest description file read.
const MAX_META_BYTES: u64 = 64 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct Meta {
    format: String,
    version: u32,
    /// Display name of the document ("Ember Spirit" or "Untitled").
    name: String,
    /// The project file it belongs to, if it was ever saved.
    original: Option<PathBuf>,
    /// Unix seconds of the write.
    saved_at: u64,
    app_version: String,
}

/// A recovery copy found on disk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecoveryEntry {
    /// Session id (the file stem).
    pub id: String,
    pub name: String,
    pub original: Option<PathBuf>,
    pub saved_at: u64,
    pub app_version: String,
    pub project: PathBuf,
    pub bytes: u64,
}

impl RecoveryEntry {
    /// Loads the recovered document.
    pub fn load(&self) -> Result<Document, file_io::FileError> {
        file_io::load_kgr(&self.project)
    }
}

/// Result of a background write.
#[derive(Debug)]
pub enum WriteResult {
    Saved,
    Failed(String),
}

/// The running session's recovery copy.
pub struct Recovery {
    dir: PathBuf,
    id: String,
    /// Exclusive lock held for the whole session; released when dropped.
    lock: Option<File>,
    lock_path: PathBuf,
    writer: Option<Receiver<WriteResult>>,
    /// History state of the document when the last copy was started.
    last_state: Option<u64>,
    /// Copies to delete once this session has written its own (opened recoveries).
    pending_delete: Vec<RecoveryEntry>,
}

impl Recovery {
    /// Starts a session in `dir` (created if needed). Without a usable folder the
    /// session still works; writes then report an error.
    pub fn start(dir: PathBuf) -> Self {
        static STARTED: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
        let n = STARTED.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let id = format!("session-{}-{}-{n}", unix_now(), std::process::id());
        let lock_path = dir.join(format!("{id}.lock"));
        let lock = fs::create_dir_all(&dir)
            .ok()
            .and_then(|()| open_exclusive(&lock_path, true).ok());
        if lock.is_none() {
            tracing::warn!("recovery folder {} is not usable", dir.display());
        }
        Self {
            dir,
            id,
            lock,
            lock_path,
            writer: None,
            last_state: None,
            pending_delete: Vec::new(),
        }
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    fn project_path(&self) -> PathBuf {
        self.dir.join(format!("{}.kgr", self.id))
    }

    fn meta_path(&self) -> PathBuf {
        self.dir.join(format!("{}.json", self.id))
    }

    /// Whether a copy is being written right now.
    pub fn is_writing(&self) -> bool {
        self.writer.is_some()
    }

    /// Whether the document at history state `state` still needs a copy.
    pub fn needs_copy(&self, state: u64) -> bool {
        self.last_state != Some(state)
    }

    /// Writes a copy of `doc` in the background. Returns `false` (and does nothing) if
    /// a previous copy is still being written.
    pub fn write(
        &mut self,
        doc: Document,
        state: u64,
        name: &str,
        original: Option<&Path>,
    ) -> bool {
        if self.writer.is_some() {
            return false;
        }
        let (tx, rx) = mpsc::channel();
        let project = self.project_path();
        let meta_path = self.meta_path();
        let meta = Meta {
            format: META_FORMAT.to_owned(),
            version: META_VERSION,
            name: name.to_owned(),
            original: original.map(Path::to_path_buf),
            saved_at: unix_now(),
            app_version: crate::APP_VERSION.to_owned(),
        };
        let spawned = std::thread::Builder::new()
            .name("kagero-autosave".into())
            .spawn(move || {
                let result = (|| -> Result<(), String> {
                    // The document first; the description only once it is complete.
                    file_io::save_kgr(&doc, &project).map_err(|e| e.to_string())?;
                    let json = serde_json::to_vec_pretty(&meta).map_err(|e| e.to_string())?;
                    write_atomic(&meta_path, |w| {
                        std::io::Write::write_all(w, &json)?;
                        Ok(())
                    })
                    .map_err(|e| e.to_string())
                })();
                let _ = tx.send(match result {
                    Ok(()) => WriteResult::Saved,
                    Err(e) => WriteResult::Failed(e),
                });
            });
        match spawned {
            Ok(_) => {
                self.writer = Some(rx);
                self.last_state = Some(state);
                true
            }
            Err(e) => {
                tracing::warn!("autosave thread not started: {e}");
                false
            }
        }
    }

    /// Checks the background write; returns its result once, when it finishes.
    pub fn poll(&mut self) -> Option<WriteResult> {
        let rx = self.writer.as_ref()?;
        let result = match rx.try_recv() {
            Ok(r) => r,
            Err(TryRecvError::Empty) => return None,
            Err(TryRecvError::Disconnected) => WriteResult::Failed("the autosave stopped".into()),
        };
        self.writer = None;
        match &result {
            WriteResult::Saved => {
                for entry in std::mem::take(&mut self.pending_delete) {
                    delete(&entry);
                }
                self.trim();
            }
            WriteResult::Failed(e) => {
                tracing::warn!("autosave failed: {e}");
                // Try again at the next opportunity.
                self.last_state = None;
            }
        }
        Some(result)
    }

    /// Waits for a running write (used before deleting or on exit).
    pub fn finish(&mut self) {
        if let Some(rx) = self.writer.take() {
            let _ = rx.recv();
        }
    }

    /// Deletes this session's copy (after a successful save, or when the document is
    /// closed). Opened recoveries that were waiting for a new copy are deleted too:
    /// the user saved or deliberately closed that work.
    pub fn discard(&mut self) {
        self.finish();
        remove_quietly(&self.meta_path());
        remove_quietly(&self.project_path());
        self.last_state = None;
        for entry in std::mem::take(&mut self.pending_delete) {
            delete(&entry);
        }
    }

    /// Remembers an opened recovery copy, to delete it once this session has its own
    /// copy (or the work is saved), so there is never a moment without one.
    pub fn adopt(&mut self, entry: RecoveryEntry) {
        self.last_state = None;
        self.pending_delete.push(entry);
    }

    /// Copies left by sessions that are no longer running (newest first), excluding
    /// copies that are about to be deleted.
    pub fn orphans(&self) -> Vec<RecoveryEntry> {
        scan(&self.dir)
            .into_iter()
            .filter(|e| e.id != self.id && !self.pending_delete.iter().any(|p| p.id == e.id))
            .collect()
    }

    /// Keeps at most [`MAX_COPIES`] copies and [`MAX_TOTAL_BYTES`], deleting the oldest
    /// crashed copies first. This session's own copy is never deleted here.
    fn trim(&self) {
        // `scan` leaves out running sessions, this one included; count its copy here.
        let mut entries = scan(&self.dir);
        let own_bytes = fs::metadata(self.project_path()).ok().map(|m| m.len());
        let mut count = entries.len() + usize::from(own_bytes.is_some());
        let mut bytes: u64 = entries.iter().map(|e| e.bytes).sum::<u64>() + own_bytes.unwrap_or(0);
        // `scan` sorts newest first: delete from the end.
        while (count > MAX_COPIES || bytes > MAX_TOTAL_BYTES) && !entries.is_empty() {
            if let Some(old) = entries.pop() {
                delete(&old);
                count -= 1;
                bytes = bytes.saturating_sub(old.bytes);
            }
        }
    }
}

impl Drop for Recovery {
    fn drop(&mut self) {
        self.finish();
        // Release the lock by closing its handle, then remove the file.
        self.lock = None;
        let _ = fs::remove_file(&self.lock_path);
    }
}

/// Deletes a recovery copy (both files).
pub fn delete(entry: &RecoveryEntry) {
    let dir = entry
        .project
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_default();
    remove_quietly(&dir.join(format!("{}.json", entry.id)));
    remove_quietly(&entry.project);
    // A crashed session's lock file is left behind; it is unlocked by now.
    remove_quietly(&dir.join(format!("{}.lock", entry.id)));
}

/// Deletes a recovery file. A file that is already gone is fine; any other failure
/// is logged, because the copy would be offered again after a restart.
fn remove_quietly(path: &Path) {
    if let Err(e) = fs::remove_file(path) {
        if e.kind() != std::io::ErrorKind::NotFound {
            tracing::warn!("could not delete recovery file {}: {e}", path.display());
        }
    }
}

/// Complete copies in `dir` whose session is not running, newest first. Damaged or
/// foreign descriptions are skipped (and logged), never deleted automatically.
pub fn scan(dir: &Path) -> Vec<RecoveryEntry> {
    let Ok(read) = fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for item in read.flatten() {
        let path = item.path();
        if path.extension().is_none_or(|e| e != "json") {
            continue;
        }
        let Some(id) = path.file_stem().map(|s| s.to_string_lossy().into_owned()) else {
            continue;
        };
        if is_running(dir, &id) {
            continue;
        }
        match read_meta(&path) {
            Ok(meta) => {
                let project = dir.join(format!("{id}.kgr"));
                let Ok(info) = fs::metadata(&project) else {
                    continue; // description without its project: incomplete write
                };
                out.push(RecoveryEntry {
                    id,
                    name: meta.name,
                    original: meta.original,
                    saved_at: meta.saved_at,
                    app_version: meta.app_version,
                    project,
                    bytes: info.len(),
                });
            }
            Err(e) => tracing::warn!("skipping recovery description {}: {e}", path.display()),
        }
    }
    out.sort_by(|a, b| b.saved_at.cmp(&a.saved_at).then_with(|| a.id.cmp(&b.id)));
    out
}

fn read_meta(path: &Path) -> Result<Meta, String> {
    let len = fs::metadata(path).map_err(|e| e.to_string())?.len();
    if len > MAX_META_BYTES {
        return Err("description too large".into());
    }
    let bytes = fs::read(path).map_err(|e| e.to_string())?;
    let meta: Meta = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    if meta.format != META_FORMAT || meta.version != META_VERSION {
        return Err("not a Kagero recovery description".into());
    }
    Ok(meta)
}

/// Opens a lock file so that nobody else can open it while the handle lives (on
/// Windows, a share mode of 0). The OS closes the handle when the process dies, so
/// a crashed session's lock file can be opened again.
fn open_exclusive(path: &Path, create: bool) -> std::io::Result<File> {
    let mut options = File::options();
    options.read(true).write(true).create(create);
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        options.share_mode(0);
    }
    options.open(path)
}

/// Whether the session `id` is still running (its lock file is held open).
fn is_running(dir: &Path, id: &str) -> bool {
    let lock = dir.join(format!("{id}.lock"));
    match open_exclusive(&lock, false) {
        // Opened (and closed again right away): nobody holds it.
        Ok(_) => false,
        // No lock file: the session ended, or crashed and the file was removed.
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => false,
        // Held by a running Kagero (or unreadable): don't offer its copy.
        Err(_) => true,
    }
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

/// "3 minutes ago", "yesterday", … for the recovery dialog.
pub fn age(saved_at: u64) -> String {
    let secs = unix_now().saturating_sub(saved_at);
    match secs {
        0..=59 => "just now".to_owned(),
        60..=3599 => plural(secs / 60, "minute"),
        3600..=86_399 => plural(secs / 3600, "hour"),
        _ => plural(secs / 86_400, "day"),
    }
}

fn plural(n: u64, unit: &str) -> String {
    if n == 1 {
        format!("1 {unit} ago")
    } else {
        format!("{n} {unit}s ago")
    }
}

/// `YYYY-MM-DD HH:MM` in UTC (no time-zone database needed).
pub fn utc_time(secs: u64) -> String {
    let days = (secs / 86_400) as i64;
    let rem = secs % 86_400;
    // Civil-from-days (Howard Hinnant's algorithm).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!(
        "{y:04}-{m:02}-{d:02} {:02}:{:02} UTC",
        rem / 3600,
        rem % 3600 / 60
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn utc_dates() {
        assert_eq!(utc_time(0), "1970-01-01 00:00 UTC");
        assert_eq!(utc_time(1_793_016_000), "2026-10-26 12:00 UTC");
    }
}
