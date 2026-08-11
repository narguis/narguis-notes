use notes_planner_desktop::{
    dto::{
        CivilDateInput, CreateNoteRequest, DeleteNoteRequest, NoteBody, NoteTitle,
        SaveDailyPageRequest, UpdateNoteRequest,
    },
    error::AppErrorCode,
    storage::{Database, DatabaseOptions, TableName, DATABASE_VERSION},
};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicUsize, Ordering},
};

static NEXT_TEMP_DIRECTORY: AtomicUsize = AtomicUsize::new(0);

#[path = "storage/startup.rs"]
mod startup;

struct TemporaryDatabaseDirectory {
    path: PathBuf,
}

impl TemporaryDatabaseDirectory {
    fn new() -> Self {
        let sequence = NEXT_TEMP_DIRECTORY.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "notes-planner-storage-{}-{sequence}",
            std::process::id()
        ));
        fs::create_dir_all(&path).expect("temporary directory should be created");
        Self { path }
    }

    fn database_path(&self) -> PathBuf {
        self.path.join("planner.sqlite3")
    }
}

impl Drop for TemporaryDatabaseDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn database(directory: &TemporaryDatabaseDirectory) -> Database {
    Database::open(directory.database_path()).expect("database should open")
}

fn save_request(date: &str, content: &str) -> SaveDailyPageRequest {
    SaveDailyPageRequest {
        date: civil_date(date),
        content: content.to_owned(),
    }
}

fn civil_date(value: &str) -> CivilDateInput {
    CivilDateInput::parse(value).expect("test civil date should parse")
}

fn create_request(title: &str, body: &str) -> CreateNoteRequest {
    CreateNoteRequest {
        title: NoteTitle(title.to_owned()),
        body: NoteBody(body.to_owned()),
    }
}

#[test]
fn creates_the_expected_schema_and_version_when_opened_fresh() {
    // Given: a path below an empty disposable app-data directory
    let directory = TemporaryDatabaseDirectory::new();

    // When: the storage owner opens the database for the first time
    let database = database(&directory);

    // Then: the database has the current ordered migration version and exact tables
    assert_eq!(
        database.schema_version().expect("version should read"),
        DATABASE_VERSION
    );
    assert_eq!(
        database
            .table_columns(TableName::DailyPages)
            .expect("columns should read"),
        vec!["date", "content", "created_at_ms", "updated_at_ms"]
    );
    assert_eq!(
        database
            .table_columns(TableName::UndatedNotes)
            .expect("columns should read"),
        vec!["id", "title", "body", "created_at_ms", "updated_at_ms"]
    );
    assert_eq!(
        database.journal_mode().expect("journal mode should read"),
        "wal"
    );
}

#[test]
fn persists_daily_pages_and_notes_across_storage_restart() {
    // Given: a fresh database and valid boundary DTOs
    let directory = TemporaryDatabaseDirectory::new();
    let mut first_database = database(&directory);

    // When: a page and note are written, then the storage owner restarts
    let saved_page = first_database
        .save_daily_page(&save_request("2026-07-30", "plan"))
        .expect("page should save");
    let created_note = first_database
        .create_note(&create_request("loose thought", "body"))
        .expect("note should create");
    drop(first_database);
    let mut second_database = database(&directory);

    // Then: each entity survives with its original boundary values
    assert_eq!(
        second_database
            .get_daily_page(&saved_page.date)
            .expect("page should read"),
        saved_page
    );
    assert_eq!(
        second_database.list_notes().expect("notes should read"),
        vec![created_note]
    );
}

#[test]
fn lazily_creates_an_empty_daily_page_and_preserves_its_timestamps_after_restart() {
    // Given: a fresh local database and a valid civil-date page key
    let directory = TemporaryDatabaseDirectory::new();
    let date = civil_date("2026-03-29");
    let mut first_database = database(&directory);

    // When: the page is read before it has content and storage restarts
    let empty_page = first_database
        .get_daily_page(&date)
        .expect("valid page should lazily create");
    drop(first_database);
    let mut second_database = database(&directory);
    let reloaded_empty_page = second_database
        .get_daily_page(&date)
        .expect("empty page should reload");
    let saved_page = second_database
        .save_daily_page(&save_request("2026-03-29", "exact content"))
        .expect("page should save");

    // Then: the virtual-empty workflow persists its date and timestamps exactly
    assert_eq!(empty_page.date, date);
    assert_eq!(empty_page.content, "");
    assert_eq!(empty_page.created_at_ms, empty_page.updated_at_ms);
    assert_eq!(reloaded_empty_page, empty_page);
    assert_eq!(saved_page.date, date);
    assert_eq!(saved_page.content, "exact content");
    assert_eq!(saved_page.created_at_ms, empty_page.created_at_ms);
    assert!(saved_page.updated_at_ms >= empty_page.updated_at_ms);
}

#[test]
fn enables_foreign_keys_on_every_storage_connection() {
    // Given: two separately opened storage connections for one local database
    let directory = TemporaryDatabaseDirectory::new();
    let first_database = database(&directory);
    let second_database = database(&directory);

    // When: each owner reports its connection configuration
    let first_enabled = first_database
        .foreign_keys_enabled()
        .expect("pragma should read");
    let second_enabled = second_database
        .foreign_keys_enabled()
        .expect("pragma should read");

    // Then: neither relies on SQLite's default foreign-key setting
    assert!(first_enabled);
    assert!(second_enabled);
}

#[test]
fn migrates_a_seeded_version_one_database_without_losing_rows() {
    // Given: a database intentionally stopped after migration one with seeded content
    let directory = TemporaryDatabaseDirectory::new();
    let mut legacy_database = Database::open_with_options(
        DatabaseOptions::for_path(directory.database_path()).with_target_version(1),
    )
    .expect("version one database should open");
    legacy_database
        .save_daily_page(&save_request("2026-07-30", "legacy page"))
        .expect("legacy page should save");
    drop(legacy_database);

    // When: the normal storage owner upgrades it
    let mut database = database(&directory);

    // Then: the version advances atomically and seeded data survives
    assert_eq!(
        database.schema_version().expect("version should read"),
        DATABASE_VERSION
    );
    assert_eq!(
        database
            .get_daily_page(&civil_date("2026-07-30"))
            .expect("page should read")
            .content,
        "legacy page"
    );
}

#[test]
fn rolls_back_a_failed_write_without_changing_the_existing_page() {
    // Given: an existing page and a deterministic write failure from the database
    let directory = TemporaryDatabaseDirectory::new();
    let mut database = database(&directory);
    database
        .save_daily_page(&save_request("2026-07-30", "before"))
        .expect("initial page should save");
    drop(database);
    let mut database = Database::open_with_options(
        DatabaseOptions::for_path(directory.database_path()).with_injected_write_failure(),
    )
    .expect("database with write failure should open");

    // When: the page save executes its short transaction
    let error = database
        .save_daily_page(&save_request("2026-07-30", "after"))
        .expect_err("trigger should fail the write");

    // Then: the typed storage error is returned and the prior row remains unchanged
    assert_eq!(error.code, AppErrorCode::StorageWriteFailed);
    assert_eq!(
        database
            .get_daily_page(&civil_date("2026-07-30"))
            .expect("page should read")
            .content,
        "before"
    );
}

#[test]
fn preserves_a_seeded_database_when_an_ordered_migration_is_injected_to_fail() {
    // Given: a seeded version-one database and a one-shot migration failure at version two
    let directory = TemporaryDatabaseDirectory::new();
    let database_path = directory.database_path();
    let mut legacy_database = Database::open_with_options(
        DatabaseOptions::for_path(database_path.clone()).with_target_version(1),
    )
    .expect("version one database should open");
    legacy_database
        .save_daily_page(&save_request("2026-07-30", "do not lose this"))
        .expect("seed page should save");
    drop(legacy_database);

    // When: migration two is explicitly injected to fail
    let error = match Database::open_with_options(
        DatabaseOptions::for_path(database_path.clone()).with_injected_migration_failure(2),
    ) {
        Err(error) => error,
        Ok(_) => panic!("migration should fail"),
    };

    // Then: no recreate/delete fallback occurs; the original version and row remain readable
    assert_eq!(error.code, AppErrorCode::StorageMigrationFailed);
    assert!(Path::new(&database_path).is_file());
    let mut preserved_database = Database::open_with_options(
        DatabaseOptions::for_path(database_path).with_target_version(1),
    )
    .expect("seeded database should remain usable");
    assert_eq!(
        preserved_database
            .schema_version()
            .expect("version should read"),
        1
    );
    assert_eq!(
        preserved_database
            .get_daily_page(&civil_date("2026-07-30"))
            .expect("page should read")
            .content,
        "do not lose this"
    );
}

#[test]
fn updates_and_deletes_a_note_through_short_transactions() {
    // Given: a persisted note
    let directory = TemporaryDatabaseDirectory::new();
    let mut database = database(&directory);
    let created_note = database
        .create_note(&create_request("before", "before body"))
        .expect("note should create");

    // When: it is updated and then deleted through the storage owner
    let updated_note = database
        .update_note(&UpdateNoteRequest {
            id: created_note.id.clone(),
            title: NoteTitle("after".to_owned()),
            body: NoteBody("after body".to_owned()),
        })
        .expect("note should update");
    database
        .delete_note(&DeleteNoteRequest {
            id: updated_note.id.clone(),
        })
        .expect("note should delete");

    // Then: the update result is typed and delete leaves no note rows
    assert_eq!(updated_note.title.0, "after");
    assert!(database.list_notes().expect("notes should list").is_empty());
}
