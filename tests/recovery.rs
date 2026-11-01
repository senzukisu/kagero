//! Autosave copies and crash recovery (Kagero 0.4).

use std::fs;
use std::path::{Path, PathBuf};

use kagero::core::{Document, Rgba};
use kagero::services::recovery::{scan, Recovery, RecoveryEntry, WriteResult, MAX_COPIES};

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("kagero-recovery-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn doc() -> Document {
    Document::new(8, 6, Rgba::opaque(200, 40, 10)).unwrap()
}

/// Writes a copy and waits for it.
fn write(r: &mut Recovery, state: u64, name: &str, original: Option<&Path>) {
    assert!(r.write(doc(), state, name, original));
    r.finish_for_test();
}

/// Copies a session's files under another id with no lock: what a crash leaves.
fn fake_crash(dir: &Path, from: &str, to: &str, saved_at: Option<u64>) {
    fs::copy(
        dir.join(format!("{from}.kgr")),
        dir.join(format!("{to}.kgr")),
    )
    .unwrap();
    let mut meta: serde_json::Value =
        serde_json::from_slice(&fs::read(dir.join(format!("{from}.json"))).unwrap()).unwrap();
    if let Some(t) = saved_at {
        meta["saved_at"] = t.into();
    }
    fs::write(
        dir.join(format!("{to}.json")),
        serde_json::to_vec(&meta).unwrap(),
    )
    .unwrap();
}

fn own_id(dir: &Path) -> String {
    fs::read_dir(dir)
        .unwrap()
        .flatten()
        .find(|e| e.path().extension().is_some_and(|x| x == "json"))
        .map(|e| e.path().file_stem().unwrap().to_string_lossy().into_owned())
        .unwrap()
}

trait FinishForTest {
    fn finish_for_test(&mut self);
}

impl FinishForTest for Recovery {
    fn finish_for_test(&mut self) {
        loop {
            match self.poll() {
                Some(WriteResult::Saved) => return,
                Some(WriteResult::Failed(e)) => panic!("write failed: {e}"),
                None => std::thread::sleep(std::time::Duration::from_millis(5)),
            }
        }
    }
}

#[test]
fn a_running_session_is_never_offered() {
    let dir = temp_dir("running");
    let mut a = Recovery::start(dir.clone());
    write(&mut a, 1, "Spirit", None);
    // Another Kagero sees the copy of a live session as not recoverable.
    let b = Recovery::start(dir.clone());
    assert!(b.orphans().is_empty());
    assert!(scan(&dir).is_empty());
    drop(b);
    drop(a);
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn a_crashed_copy_is_found_and_loads() {
    let dir = temp_dir("crash");
    let original = dir.join("project.kgr");
    let mut a = Recovery::start(dir.clone());
    write(&mut a, 1, "Spirit", Some(&original));
    let id = own_id(&dir);
    fake_crash(&dir, &id, "session-1-1-0", None);

    let b = Recovery::start(dir.clone());
    let found = b.orphans();
    assert_eq!(found.len(), 1);
    let e = &found[0];
    assert_eq!(e.id, "session-1-1-0");
    assert_eq!(e.name, "Spirit");
    assert_eq!(e.original.as_deref(), Some(original.as_path()));
    let loaded = e.load().unwrap();
    assert_eq!(loaded.width(), 8);
    assert_eq!(
        loaded.composite_frame_pixel(0, 2, 3),
        Some(Rgba::opaque(200, 40, 10))
    );
    // Recovery never touched the original path.
    assert!(!original.exists());
    drop((a, b));
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn discard_removes_only_the_own_copy() {
    let dir = temp_dir("discard");
    let mut a = Recovery::start(dir.clone());
    write(&mut a, 1, "A", None);
    let id = own_id(&dir);
    fake_crash(&dir, &id, "session-2-2-0", None);
    a.discard();
    assert!(!dir.join(format!("{id}.kgr")).exists());
    assert!(!dir.join(format!("{id}.json")).exists());
    assert!(dir.join("session-2-2-0.kgr").exists());
    // After a discard the next change needs a copy again.
    assert!(a.needs_copy(1));
    drop(a);
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn an_adopted_copy_lives_until_the_new_one_is_written() {
    let dir = temp_dir("adopt");
    let mut a = Recovery::start(dir.clone());
    write(&mut a, 1, "A", None);
    let id = own_id(&dir);
    fake_crash(&dir, &id, "session-3-3-0", None);
    a.discard();

    let mut b = Recovery::start(dir.clone());
    let entry: RecoveryEntry = b.orphans().remove(0);
    b.adopt(entry);
    // Hidden from the list, but still on disk.
    assert!(b.orphans().is_empty());
    assert!(dir.join("session-3-3-0.kgr").exists());
    write(&mut b, 7, "A (recovered)", None);
    assert!(!dir.join("session-3-3-0.kgr").exists());
    assert!(!dir.join("session-3-3-0.json").exists());
    drop((a, b));
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn damaged_and_incomplete_copies_are_skipped_not_deleted() {
    let dir = temp_dir("damaged");
    let mut a = Recovery::start(dir.clone());
    write(&mut a, 1, "A", None);
    let id = own_id(&dir);
    // Garbage description, a foreign one, one without its project, and a broken project.
    fs::write(dir.join("session-x.json"), b"{not json").unwrap();
    fs::write(dir.join("session-x.kgr"), b"zzz").unwrap();
    fs::write(
        dir.join("session-y.json"),
        br#"{"format":"other","version":1}"#,
    )
    .unwrap();
    fake_crash(&dir, &id, "session-z", None);
    fs::remove_file(dir.join("session-z.kgr")).unwrap();
    fake_crash(&dir, &id, "session-w", None);
    fs::write(dir.join("session-w.kgr"), b"not a zip").unwrap();

    let found = scan(&dir);
    assert_eq!(found.len(), 1, "only the broken-project copy is listed");
    assert_eq!(found[0].id, "session-w");
    // Opening it fails with an error instead of looping or panicking.
    assert!(found[0].load().is_err());
    assert!(dir.join("session-x.json").exists());
    drop(a);
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn old_copies_are_trimmed_but_never_the_own_one() {
    let dir = temp_dir("trim");
    let mut a = Recovery::start(dir.clone());
    write(&mut a, 1, "A", None);
    let id = own_id(&dir);
    for i in 0..(MAX_COPIES + 4) {
        fake_crash(
            &dir,
            &id,
            &format!("session-old-{i:02}"),
            Some(1_000 + i as u64),
        );
    }
    // The next write trims.
    write(&mut a, 2, "A", None);
    let left = scan(&dir);
    assert_eq!(left.len(), MAX_COPIES - 1);
    // The newest crashed copies survive.
    assert_eq!(left[0].id, format!("session-old-{:02}", MAX_COPIES + 3));
    assert!(dir.join(format!("{id}.kgr")).exists());
    drop(a);
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn needs_copy_follows_the_history_state() {
    let dir = temp_dir("state");
    let mut a = Recovery::start(dir.clone());
    assert!(a.needs_copy(4));
    write(&mut a, 4, "A", None);
    assert!(!a.needs_copy(4));
    assert!(a.needs_copy(5));
    drop(a);
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn a_dropped_session_releases_its_lock() {
    let dir = temp_dir("drop");
    let mut a = Recovery::start(dir.clone());
    write(&mut a, 1, "A", None);
    let id = own_id(&dir);
    drop(a);
    assert!(!dir.join(format!("{id}.lock")).exists());
    // Closing without discarding (a crash right after a write) leaves a recoverable copy.
    let found = scan(&dir);
    assert_eq!(found.len(), 1);
    kagero::services::recovery::delete(&found[0]);
    assert!(scan(&dir).is_empty());
    let _ = fs::remove_dir_all(&dir);
}
