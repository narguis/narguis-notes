use notes_planner_desktop::{
    dto::{CivilDateInput, CreateNoteRequest, NoteBody, NoteTitle, SaveDailyPageRequest},
    storage::Database,
};
use std::{fs, path::PathBuf};

struct TemporaryDatabaseDirectory {
    path: PathBuf,
}

impl TemporaryDatabaseDirectory {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "notes-planner-legacy-characterization-{}",
            std::process::id()
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

#[test]
fn preserves_legacy_daily_prose_and_undated_note_bytes_across_restart() {
    // Given: deliberately non-structured legacy prose and an independent freeform note
    let directory = TemporaryDatabaseDirectory::new();
    let daily_prose = "  Keep this prose exactly.\n- not a line\n\n09:31 maybe? 🗒️\n";
    let note_title = "Loose notes\t";
    let note_body = "verbatim\nbody\nwith spaces  \nand emoji 🧪";
    let date = CivilDateInput::parse("2026-07-30").expect("test date should parse");
    let mut first_database = Database::open(directory.database_path()).expect("database opens");

    // When: legacy records are persisted and the storage owner restarts
    first_database
        .save_daily_page(&SaveDailyPageRequest {
            date: date.clone(),
            content: daily_prose.to_owned(),
        })
        .expect("daily prose saves");
    let created_note = first_database
        .create_note(&CreateNoteRequest {
            title: NoteTitle(note_title.to_owned()),
            body: NoteBody(note_body.to_owned()),
        })
        .expect("note saves");
    drop(first_database);
    let mut restarted_database =
        Database::open(directory.database_path()).expect("database restarts");

    // Then: storage returns exactly the original UTF-8 bytes, without interpreting prose
    let reloaded_page = restarted_database
        .get_daily_page(&date)
        .expect("daily prose reloads");
    let reloaded_note = restarted_database
        .list_notes()
        .expect("notes reload")
        .into_iter()
        .find(|note| note.id == created_note.id)
        .expect("created note remains present");
    assert_eq!(reloaded_page.content.as_bytes(), daily_prose.as_bytes());
    assert_eq!(reloaded_note.title.0.as_bytes(), note_title.as_bytes());
    assert_eq!(reloaded_note.body.0.as_bytes(), note_body.as_bytes());
}
