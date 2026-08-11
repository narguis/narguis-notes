use notes_planner_desktop::{
    dto::{CivilDateInput, CreateNoteRequest, NoteBody, NoteTitle, SaveDailyPageRequest},
    storage::{Database, DatabaseOptions},
};
use rusqlite::{params, Connection};
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
};

static NEXT_TEMP_DIRECTORY: AtomicUsize = AtomicUsize::new(0);

struct TemporaryDatabaseDirectory {
    path: PathBuf,
}

impl TemporaryDatabaseDirectory {
    fn new() -> Self {
        let sequence = NEXT_TEMP_DIRECTORY.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "notes-planner-outline-migration-{}-{sequence}",
            std::process::id(),
        ));
        let _ = fs::remove_dir_all(&path);
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

fn seed_v3_line(path: &PathBuf, date: &CivilDateInput, text: &str) {
    let connection = Connection::open(path).expect("v3 database should open directly");
    connection
        .execute(
            "INSERT INTO planner_lines (id, date, parent_id, sibling_key, text, time_of_day_minutes, is_collapsed, created_at_ms, updated_at_ms) VALUES (?1, ?2, NULL, 'a', ?3, NULL, 0, 1, 1)",
            params!["00000000-0000-4000-8000-000000000001", date.as_str().expect("date should validate"), text],
        )
        .expect("v3 line should save");
}

#[test]
fn migrates_v2_additively_without_changing_legacy_prose_or_notes() {
    // Given: a version-two database containing deliberately unstructured legacy text
    let directory = TemporaryDatabaseDirectory::new();
    let path = directory.database_path();
    let prose = "not parsed\n  09:31?\n🗒️\n";
    let note_body = "independent notes stay exact\n ";
    let date = CivilDateInput::parse("2026-07-30").expect("test date should parse");
    let mut v2_database =
        Database::open_with_options(DatabaseOptions::for_path(path.clone()).with_target_version(2))
            .expect("version two database should open");
    v2_database
        .save_daily_page(&SaveDailyPageRequest {
            date: date.clone(),
            content: prose.to_owned(),
        })
        .expect("prose should save");
    let note = v2_database
        .create_note(&CreateNoteRequest {
            title: NoteTitle("note".to_owned()),
            body: NoteBody(note_body.to_owned()),
        })
        .expect("note should save");
    drop(v2_database);

    // When: normal storage applies migrations through v4
    let mut migrated_database = Database::open(path).expect("v4 migration should succeed");

    // Then: the new schema exists while the prior records retain their original bytes
    assert_eq!(
        migrated_database.schema_version().expect("version reads"),
        4
    );
    assert_eq!(
        migrated_database
            .get_daily_page(&date)
            .expect("page reads")
            .content
            .as_bytes(),
        prose.as_bytes()
    );
    assert_eq!(
        migrated_database
            .list_notes()
            .expect("notes read")
            .into_iter()
            .find(|candidate| candidate.id == note.id)
            .expect("note remains")
            .body
            .0
            .as_bytes(),
        note_body.as_bytes()
    );
}

#[test]
fn preserves_v2_database_when_v3_migration_is_interrupted() {
    // Given: a version-two database with one exact legacy page
    let directory = TemporaryDatabaseDirectory::new();
    let path = directory.database_path();
    let date = CivilDateInput::parse("2026-07-30").expect("test date should parse");
    let mut v2_database =
        Database::open_with_options(DatabaseOptions::for_path(path.clone()).with_target_version(2))
            .expect("version two database should open");
    v2_database
        .save_daily_page(&SaveDailyPageRequest {
            date: date.clone(),
            content: "do not parse or lose me".to_owned(),
        })
        .expect("prose should save");
    drop(v2_database);

    // When: the v3 migration failure hook interrupts its immediate transaction
    let error = match Database::open_with_options(
        DatabaseOptions::for_path(path.clone()).with_injected_migration_failure(3),
    ) {
        Err(error) => error,
        Ok(_) => panic!("v3 migration should fail"),
    };

    // Then: the existing file and version-two contents are still readable intact
    assert_eq!(error.code.to_string(), "storage_migration_failed");
    let mut preserved_database =
        Database::open_with_options(DatabaseOptions::for_path(path).with_target_version(2))
            .expect("version-two database remains usable");
    assert_eq!(
        preserved_database.schema_version().expect("version reads"),
        2
    );
    assert_eq!(
        preserved_database
            .get_daily_page(&date)
            .expect("page reads")
            .content,
        "do not parse or lose me"
    );
}

#[test]
fn migrates_v3_line_text_to_exact_title_with_null_description() {
    // Given: a v3 line containing long, multiline Unicode text with leading and trailing spaces
    let directory = TemporaryDatabaseDirectory::new();
    let path = directory.database_path();
    let date = CivilDateInput::parse("2026-07-30").expect("test date should parse");
    let legacy_text = "  heading 🧪\nsecond line\n\ntrailing space ";
    let mut v3_database =
        Database::open_with_options(DatabaseOptions::for_path(path.clone()).with_target_version(3))
            .expect("version three database should open");
    v3_database
        .save_daily_page(&SaveDailyPageRequest {
            date: date.clone(),
            content: String::new(),
        })
        .expect("v3 daily page should save");
    drop(v3_database);
    seed_v3_line(&path, &date, legacy_text);

    // When: normal startup applies migration v4
    let mut migrated_database = Database::open(path).expect("v4 migration should succeed");

    // Then: the legacy bytes are the exact title and the new description remains SQL NULL
    let lines = migrated_database
        .list_planner_lines(&date)
        .expect("lines should list after migration");
    assert_eq!(
        migrated_database.schema_version().expect("version reads"),
        4
    );
    assert_eq!(lines[0].title.0.as_bytes(), legacy_text.as_bytes());
    assert_eq!(lines[0].description, None);
}

#[test]
fn preserves_v3_database_when_v4_migration_is_interrupted() {
    // Given: a v3 database containing a planner line
    let directory = TemporaryDatabaseDirectory::new();
    let path = directory.database_path();
    let mut v3_database =
        Database::open_with_options(DatabaseOptions::for_path(path.clone()).with_target_version(3))
            .expect("version three database should open");
    let date = CivilDateInput::parse("2026-07-30").expect("test date should parse");
    v3_database
        .save_daily_page(&SaveDailyPageRequest {
            date: date.clone(),
            content: String::new(),
        })
        .expect("v3 daily page should save");
    drop(v3_database);
    seed_v3_line(&path, &date, "do not alter this v3 text");

    // When: the injected v4 failure aborts the immediate migration transaction
    let error = match Database::open_with_options(
        DatabaseOptions::for_path(path.clone()).with_injected_migration_failure(4),
    ) {
        Err(error) => error,
        Ok(_) => panic!("v4 migration should fail"),
    };

    // Then: SQLite retains the v3 version, column, and exact original bytes
    let connection = Connection::open(path).expect("preserved database should open directly");
    let version = connection
        .pragma_query_value(None, "user_version", |row| row.get::<_, u32>(0))
        .expect("version should read");
    let text: String = connection
        .query_row("SELECT text FROM planner_lines", [], |row| row.get(0))
        .expect("v3 text should remain");
    assert_eq!(error.code.to_string(), "storage_migration_failed");
    assert_eq!(version, 3);
    assert_eq!(text.as_bytes(), b"do not alter this v3 text");
}

#[test]
fn rolls_back_v4_when_failure_follows_its_first_statement() {
    // Given: a version-three line with legacy text that migration v4 must not partially transform
    let directory = TemporaryDatabaseDirectory::new();
    let path = directory.database_path();
    let date = CivilDateInput::parse("2026-07-30").expect("test date should parse");
    let mut v3_database =
        Database::open_with_options(DatabaseOptions::for_path(path.clone()).with_target_version(3))
            .expect("version three database should open");
    v3_database
        .save_daily_page(&SaveDailyPageRequest {
            date: date.clone(),
            content: String::new(),
        })
        .expect("v3 daily page should save");
    drop(v3_database);
    seed_v3_line(&path, &date, "unchanged after the first v4 statement");

    // When: the test hook fails after v4 renames text but before it adds description
    let error = match Database::open_with_options(
        DatabaseOptions::for_path(path.clone())
            .with_injected_migration_failure_after_first_statement(4),
    ) {
        Err(error) => error,
        Ok(_) => panic!("v4 migration should fail after its first statement"),
    };

    // Then: the whole immediate transaction rolls back to the v3 schema, version, and row bytes
    let connection = Connection::open(path).expect("preserved database should open directly");
    let version = connection
        .pragma_query_value(None, "user_version", |row| row.get::<_, u32>(0))
        .expect("version should read");
    let columns = connection
        .prepare("PRAGMA table_info(planner_lines)")
        .expect("v3 table should remain queryable")
        .query_map([], |row| row.get::<_, String>(1))
        .expect("column query should run")
        .collect::<Result<Vec<_>, _>>()
        .expect("column names should read");
    let text: String = connection
        .query_row("SELECT text FROM planner_lines", [], |row| row.get(0))
        .expect("v3 text should remain");
    assert_eq!(error.code.to_string(), "storage_migration_failed");
    assert_eq!(version, 3);
    assert!(columns.iter().any(|column| column == "text"));
    assert!(!columns.iter().any(|column| column == "title"));
    assert!(!columns.iter().any(|column| column == "description"));
    assert_eq!(text, "unchanged after the first v4 statement");
}
