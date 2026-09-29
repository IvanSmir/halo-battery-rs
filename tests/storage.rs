use std::fs;
use std::time::Duration;

use halo_battery::storage::FileWatch;
use tempfile::TempDir;

fn watch(dir: &TempDir, every: Duration) -> FileWatch {
    FileWatch::new(dir.path().join("config.json"), every)
}

/// Rewrites the file so its modification time moves on, even on file systems
/// that keep coarse timestamps.
fn touch(dir: &TempDir, content: &str) {
    std::thread::sleep(Duration::from_millis(20));
    fs::write(dir.path().join("config.json"), content).unwrap();
}

#[test]
fn nothing_changed_at_first() {
    let dir = TempDir::new().unwrap();
    touch(&dir, "a");
    assert!(!watch(&dir, Duration::ZERO).changed());
}

#[test]
fn a_rewrite_is_reported_once() {
    let dir = TempDir::new().unwrap();
    touch(&dir, "a");
    let mut w = watch(&dir, Duration::ZERO);
    touch(&dir, "b");
    assert!(w.changed());
    assert!(!w.changed());
}

#[test]
fn creating_and_removing_the_file_count_as_changes() {
    let dir = TempDir::new().unwrap();
    let mut w = watch(&dir, Duration::ZERO);
    touch(&dir, "a");
    assert!(w.changed());
    fs::remove_file(w.path()).unwrap();
    assert!(w.changed());
}

#[test]
fn our_own_writes_can_be_ignored() {
    let dir = TempDir::new().unwrap();
    let mut w = watch(&dir, Duration::ZERO);
    touch(&dir, "a");
    w.mark_seen();
    assert!(!w.changed());
}

#[test]
fn checks_no_more_often_than_asked() {
    let dir = TempDir::new().unwrap();
    let mut w = watch(&dir, Duration::from_secs(3600));
    touch(&dir, "a");
    assert!(!w.changed(), "the next check is an hour away");
}
