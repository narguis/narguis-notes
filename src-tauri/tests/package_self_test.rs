use notes_planner_desktop::{run_package_self_test, PACKAGE_SELF_TEST_SUCCESS_LOG};
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
};

static NEXT_TEMP_DIRECTORY: AtomicUsize = AtomicUsize::new(0);

struct TemporaryAppDataDirectory(PathBuf);

impl TemporaryAppDataDirectory {
    fn new() -> Self {
        let sequence = NEXT_TEMP_DIRECTORY.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "notes-planner-package-self-test-{}-{sequence}",
            std::process::id(),
        ));
        let _ = fs::remove_dir_all(&path);
        Self(path)
    }
}

impl Drop for TemporaryAppDataDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn package_self_test_writes_restarts_and_verifies_the_complete_persistence_fixture() {
    // Given: an isolated framework-owned app-data directory
    let directory = TemporaryAppDataDirectory::new();

    // When: the packaged self-test exercises the real SQLite storage path
    let result = run_package_self_test(&directory.0);

    // Then: exact prose, tree, Notes, templates, and copied-line independence all pass
    assert!(result.is_ok());
    assert!(directory.0.join("notes-planner.sqlite3").is_file());
}

#[test]
fn package_self_test_reports_title_description_template_mapping_after_restart() {
    // Given: the packaged self-test's documented release evidence marker
    let expected = "PACKAGE_SELF_TEST_OK title_description_template_mapping_after_restart";

    // When: the packaged binary selects its success log line
    let actual = PACKAGE_SELF_TEST_SUCCESS_LOG;

    // Then: the clean Ubuntu verifier can require mapping coverage explicitly
    assert_eq!(actual, expected);
}
