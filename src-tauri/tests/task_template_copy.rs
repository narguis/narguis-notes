use notes_planner_desktop::{
    dto::{
        CivilDateInput, CreateTaskTemplateRequest, InsertTaskTemplateCopyRequest,
        PlannerLineDescription, PlannerLineTitle, SiblingKey, TaskTemplateBody, TaskTemplateTitle,
        TimeOfDayMinutes, UpdateTaskTemplateRequest,
    },
    error::AppErrorCode,
    storage::Database,
    storage::DatabaseOptions,
};
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
};

static NEXT_TEMP_DIRECTORY: AtomicUsize = AtomicUsize::new(0);

struct TemporaryDatabaseDirectory(PathBuf);

impl TemporaryDatabaseDirectory {
    fn new() -> Self {
        let sequence = NEXT_TEMP_DIRECTORY.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "notes-planner-template-copy-{}-{sequence}",
            std::process::id(),
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("temporary directory should be created");
        Self(path)
    }

    fn database_path(&self) -> PathBuf {
        self.0.join("planner.sqlite3")
    }
}

impl Drop for TemporaryDatabaseDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn inserts_an_independent_template_copy_that_survives_template_edit_and_delete() {
    // Given: a reusable one-line template with an optional local time
    let directory = TemporaryDatabaseDirectory::new();
    let mut database = Database::open(directory.database_path()).expect("database opens");
    let template = database
        .create_task_template(&CreateTaskTemplateRequest {
            title: TaskTemplateTitle("Review launch checklist".to_owned()),
            body: TaskTemplateBody("Confirm desktop launch".to_owned()),
            time_of_day_minutes: Some(571),
        })
        .expect("template creates");

    // When: the template is copied into a planner day, then changed and deleted
    let inserted_line = database
        .insert_task_template_copy(&InsertTaskTemplateCopyRequest {
            template_id: template.id.clone(),
            date: CivilDateInput::parse("2026-07-30").expect("date parses"),
            parent_id: None,
            sibling_key: SiblingKey("a".to_owned()),
        })
        .expect("template copies");
    database
        .update_task_template(&UpdateTaskTemplateRequest {
            id: template.id.clone(),
            title: TaskTemplateTitle("Changed template".to_owned()),
            body: TaskTemplateBody("Changed body".to_owned()),
            time_of_day_minutes: None,
        })
        .expect("template updates");
    database
        .delete_task_template(&template.id)
        .expect("template deletes");

    // Then: the inserted line has its own identity and original copied values
    assert_ne!(inserted_line.id.0, template.id.0);
    assert_eq!(
        inserted_line.title,
        PlannerLineTitle("Review launch checklist".to_owned())
    );
    assert_eq!(
        inserted_line.description,
        Some(PlannerLineDescription("Confirm desktop launch".to_owned()))
    );
    assert_eq!(
        inserted_line.time_of_day_minutes,
        Some(TimeOfDayMinutes(571))
    );
    assert!(database
        .list_task_templates()
        .expect("templates list")
        .is_empty());
    assert_eq!(
        database
            .list_planner_lines(&CivilDateInput::parse("2026-07-30").expect("date parses"))
            .expect("lines list"),
        vec![inserted_line]
    );
}

#[test]
fn delete_failure_keeps_template_and_inserted_line_unchanged() {
    // Given: a template that has already been copied into a planner day
    let directory = TemporaryDatabaseDirectory::new();
    let mut database = Database::open(directory.database_path()).expect("database opens");
    let template = database
        .create_task_template(&CreateTaskTemplateRequest {
            title: TaskTemplateTitle("Review launch checklist".to_owned()),
            body: TaskTemplateBody("Confirm desktop launch".to_owned()),
            time_of_day_minutes: Some(571),
        })
        .expect("template creates");
    let inserted_line = database
        .insert_task_template_copy(&InsertTaskTemplateCopyRequest {
            template_id: template.id.clone(),
            date: CivilDateInput::parse("2026-07-30").expect("date parses"),
            parent_id: None,
            sibling_key: SiblingKey("a".to_owned()),
        })
        .expect("template copies");
    drop(database);

    // When: the next template deletion is injected to fail
    let mut failing_database = Database::open_with_options(
        DatabaseOptions::for_path(directory.database_path())
            .with_injected_template_delete_failure(),
    )
    .expect("failure database opens");
    let error = failing_database
        .delete_task_template(&template.id)
        .expect_err("template deletion should fail");

    // Then: the typed error is returned and neither observable record changes
    assert_eq!(error.code, AppErrorCode::StorageWriteFailed);
    assert_eq!(
        failing_database
            .list_task_templates()
            .expect("templates list"),
        vec![template]
    );
    assert_eq!(
        failing_database
            .list_planner_lines(&CivilDateInput::parse("2026-07-30").expect("date parses"))
            .expect("lines list"),
        vec![inserted_line]
    );
}
