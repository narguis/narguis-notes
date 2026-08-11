use notes_planner_desktop::{
    commands::{
        execute_create_note, execute_delete_note, execute_get_daily_page, execute_list_notes,
        execute_save_daily_page, execute_update_note,
    },
    error::{AppError, AppErrorCode},
    ipc::{
        parse_create_note_request, parse_create_task_template_request, parse_delete_note_request,
        parse_delete_task_template_request, parse_get_daily_page_request,
        parse_insert_task_template_copy_request, parse_save_daily_page_request,
        parse_update_note_request, parse_update_task_template_request,
    },
    storage::Storage,
};
use std::{fs, path::PathBuf};

fn error_code<T: std::fmt::Debug>(result: Result<T, AppError>) -> AppErrorCode {
    result.unwrap_err().code
}

#[test]
fn rejects_malformed_daily_page_payloads_before_command_execution() {
    // Given: malformed JSON, an unknown field, impossible calendar date, and instant-like keys
    let malformed_json = "{\"date\":";
    let unknown_field = r#"{"date":"2026-02-28","sql":"DROP TABLE notes"}"#;
    let invalid_date = r#"{"date":"2026-02-30"}"#;
    let utc_date = r#"{"date":"2026-07-30T00:00:00Z"}"#;
    let offset_date = r#"{"date":"2026-07-30T00:00:00+01:00"}"#;

    // When: each daily-page command parses its native payload
    let get_malformed = error_code(parse_get_daily_page_request(malformed_json));
    let get_unknown = error_code(parse_get_daily_page_request(unknown_field));
    let get_invalid_date = error_code(parse_get_daily_page_request(invalid_date));
    let get_utc_date = error_code(parse_get_daily_page_request(utc_date));
    let get_offset_date = error_code(parse_get_daily_page_request(offset_date));
    let save_malformed = error_code(parse_save_daily_page_request(malformed_json));
    let save_unknown = error_code(parse_save_daily_page_request(unknown_field));
    let save_invalid_date = error_code(parse_save_daily_page_request(
        r#"{"date":"2026-02-30","content":"plan"}"#,
    ));

    // Then: the boundary rejects every bad payload with a stable code
    assert_eq!(get_malformed, AppErrorCode::InvalidPayload);
    assert_eq!(get_unknown, AppErrorCode::InvalidPayload);
    assert_eq!(get_invalid_date, AppErrorCode::InvalidDate);
    assert_eq!(get_utc_date, AppErrorCode::InvalidDate);
    assert_eq!(get_offset_date, AppErrorCode::InvalidDate);
    assert_eq!(save_malformed, AppErrorCode::InvalidPayload);
    assert_eq!(save_unknown, AppErrorCode::InvalidPayload);
    assert_eq!(save_invalid_date, AppErrorCode::InvalidDate);
}

#[test]
fn rejects_malformed_note_payloads_before_command_execution() {
    // Given: malformed and unknown-field payloads for each note command
    let malformed_json = "{\"id\":";
    let create_unknown = r#"{"title":"ok","body":"ok","shell":"id"}"#;
    let update_unknown = r#"{"id":"note-1","title":"ok","body":"ok","sql":"SELECT 1"}"#;
    let delete_unknown = r#"{"id":"note-1","path":"/etc/passwd"}"#;

    // When: the note DTOs parse the untrusted payloads
    let create_malformed = error_code(parse_create_note_request(malformed_json));
    let create_unknown_error = error_code(parse_create_note_request(create_unknown));
    let update_malformed = error_code(parse_update_note_request(malformed_json));
    let update_unknown_error = error_code(parse_update_note_request(update_unknown));
    let delete_malformed = error_code(parse_delete_note_request(malformed_json));
    let delete_unknown_error = error_code(parse_delete_note_request(delete_unknown));

    // Then: invalid input has no route to future native or persistence work
    assert_eq!(create_malformed, AppErrorCode::InvalidPayload);
    assert_eq!(create_unknown_error, AppErrorCode::InvalidPayload);
    assert_eq!(update_malformed, AppErrorCode::InvalidPayload);
    assert_eq!(update_unknown_error, AppErrorCode::InvalidPayload);
    assert_eq!(delete_malformed, AppErrorCode::InvalidPayload);
    assert_eq!(delete_unknown_error, AppErrorCode::InvalidPayload);
}

#[test]
fn accepts_exact_note_limits_and_rejects_the_next_character() {
    // Given: note fields at the supported limits and one character past each limit
    let accepted_create = format!(
        r#"{{"title":"{}","body":"{}"}}"#,
        "t".repeat(200),
        "b".repeat(10_000)
    );
    let rejected_title = format!(r#"{{"title":"{}","body":"ok"}}"#, "t".repeat(201));
    let accepted_update = format!(
        r#"{{"id":"note-1","title":"{}","body":"{}"}}"#,
        "t".repeat(200),
        "b".repeat(10_000)
    );
    let rejected_body = format!(
        r#"{{"id":"note-1","title":"ok","body":"{}"}}"#,
        "b".repeat(10_001)
    );
    let accepted_daily_page = format!(
        r#"{{"date":"2026-02-28","content":"{}"}}"#,
        "p".repeat(10_000)
    );
    let rejected_daily_page = format!(
        r#"{{"date":"2026-02-28","content":"{}"}}"#,
        "p".repeat(10_001)
    );

    // When: create and update payloads cross the typed boundary
    let accepted_create_request = parse_create_note_request(&accepted_create);
    let rejected_title_error = error_code(parse_create_note_request(&rejected_title));
    let accepted_update_request = parse_update_note_request(&accepted_update);
    let rejected_body_error = error_code(parse_update_note_request(&rejected_body));
    let accepted_daily_page_request = parse_save_daily_page_request(&accepted_daily_page);
    let rejected_daily_page_error = error_code(parse_save_daily_page_request(&rejected_daily_page));

    // Then: exact limits parse and the next character returns the stable rejection code
    assert!(accepted_create_request.is_ok());
    assert_eq!(rejected_title_error, AppErrorCode::TitleTooLong);
    assert!(accepted_update_request.is_ok());
    assert_eq!(rejected_body_error, AppErrorCode::BodyTooLong);
    assert!(accepted_daily_page_request.is_ok());
    assert_eq!(rejected_daily_page_error, AppErrorCode::BodyTooLong);
}

#[test]
fn rejects_malformed_task_template_requests_before_storage_access() {
    // Given: malformed JSON and unknown fields for every payload-bearing template command
    let malformed_json = "{\"title\":";
    let create_unknown = r#"{"title":"ok","body":"ok","shell":"id"}"#;
    let update_unknown = r#"{"id":"550e8400-e29b-41d4-a716-446655440000","title":"ok","body":"ok","sql":"SELECT 1"}"#;
    let delete_unknown = r#"{"id":"550e8400-e29b-41d4-a716-446655440000","path":"/etc/passwd"}"#;
    let insert_unknown = r#"{"templateId":"550e8400-e29b-41d4-a716-446655440000","date":"2026-07-30","parentId":null,"siblingKey":"a","shell":"id"}"#;

    // When: each TaskTemplate DTO crosses its native parser
    let create_malformed = error_code(parse_create_task_template_request(malformed_json));
    let create_unknown_error = error_code(parse_create_task_template_request(create_unknown));
    let update_malformed = error_code(parse_update_task_template_request(malformed_json));
    let update_unknown_error = error_code(parse_update_task_template_request(update_unknown));
    let delete_malformed = error_code(parse_delete_task_template_request(malformed_json));
    let delete_unknown_error = error_code(parse_delete_task_template_request(delete_unknown));
    let insert_malformed = error_code(parse_insert_task_template_copy_request(malformed_json));
    let insert_unknown_error = error_code(parse_insert_task_template_copy_request(insert_unknown));

    // Then: no malformed request reaches a storage command
    assert_eq!(create_malformed, AppErrorCode::InvalidPayload);
    assert_eq!(create_unknown_error, AppErrorCode::InvalidPayload);
    assert_eq!(update_malformed, AppErrorCode::InvalidPayload);
    assert_eq!(update_unknown_error, AppErrorCode::InvalidPayload);
    assert_eq!(delete_malformed, AppErrorCode::InvalidPayload);
    assert_eq!(delete_unknown_error, AppErrorCode::InvalidPayload);
    assert_eq!(insert_malformed, AppErrorCode::InvalidPayload);
    assert_eq!(insert_unknown_error, AppErrorCode::InvalidPayload);
}

#[test]
fn rejects_invalid_task_template_identifiers_and_minutes() {
    // Given: invalid UUIDs and out-of-range local minutes in TaskTemplate requests
    let invalid_update_id = r#"{"id":"not-a-uuid","title":"ok","body":"ok","timeOfDayMinutes":0}"#;
    let invalid_delete_id = r#"{"id":"not-a-uuid"}"#;
    let invalid_insert_id =
        r#"{"templateId":"not-a-uuid","date":"2026-07-30","parentId":null,"siblingKey":"a"}"#;
    let invalid_create_time = r#"{"title":"ok","body":"ok","timeOfDayMinutes":1440}"#;
    let invalid_update_time = r#"{"id":"550e8400-e29b-41d4-a716-446655440000","title":"ok","body":"ok","timeOfDayMinutes":1440}"#;

    // When: the invalid values cross their corresponding parsers
    let update_id_error = error_code(parse_update_task_template_request(invalid_update_id));
    let delete_id_error = error_code(parse_delete_task_template_request(invalid_delete_id));
    let insert_id_error = error_code(parse_insert_task_template_copy_request(invalid_insert_id));
    let create_time_error = error_code(parse_create_task_template_request(invalid_create_time));
    let update_time_error = error_code(parse_update_task_template_request(invalid_update_time));

    // Then: each semantic validation returns its stable typed code
    assert_eq!(update_id_error, AppErrorCode::InvalidIdentifier);
    assert_eq!(delete_id_error, AppErrorCode::InvalidIdentifier);
    assert_eq!(insert_id_error, AppErrorCode::InvalidIdentifier);
    assert_eq!(create_time_error, AppErrorCode::InvalidTimeOfDay);
    assert_eq!(update_time_error, AppErrorCode::InvalidTimeOfDay);
}

#[test]
fn rejects_oversized_task_template_title_and_body() {
    // Given: title and body values one character beyond the supported limits
    let oversized_create_title = format!(
        r#"{{"title":"{}","body":"ok","timeOfDayMinutes":null}}"#,
        "t".repeat(201)
    );
    let oversized_create_body = format!(
        r#"{{"title":"ok","body":"{}","timeOfDayMinutes":null}}"#,
        "b".repeat(10_001)
    );
    let oversized_update_title = format!(
        r#"{{"id":"550e8400-e29b-41d4-a716-446655440000","title":"{}","body":"ok","timeOfDayMinutes":null}}"#,
        "t".repeat(201)
    );
    let oversized_update_body = format!(
        r#"{{"id":"550e8400-e29b-41d4-a716-446655440000","title":"ok","body":"{}","timeOfDayMinutes":null}}"#,
        "b".repeat(10_001)
    );

    // When: each oversized field crosses the create or update parser
    let create_title_error =
        error_code(parse_create_task_template_request(&oversized_create_title));
    let create_body_error = error_code(parse_create_task_template_request(&oversized_create_body));
    let update_title_error =
        error_code(parse_update_task_template_request(&oversized_update_title));
    let update_body_error = error_code(parse_update_task_template_request(&oversized_update_body));

    // Then: the boundary returns the field-specific stable size error
    assert_eq!(create_title_error, AppErrorCode::TitleTooLong);
    assert_eq!(create_body_error, AppErrorCode::BodyTooLong);
    assert_eq!(update_title_error, AppErrorCode::TitleTooLong);
    assert_eq!(update_body_error, AppErrorCode::BodyTooLong);
}

#[test]
fn every_exposed_command_uses_only_the_typed_storage_boundary() {
    // Given: valid DTOs for every payload command, the no-payload list command, and disposable storage
    let storage_directory = temporary_storage_directory();
    let storage = Storage::open(storage_directory.join("planner.sqlite3")).unwrap();
    let get_request = parse_get_daily_page_request(r#"{"date":"2026-02-28"}"#).unwrap();
    let save_request =
        parse_save_daily_page_request(r#"{"date":"2026-02-28","content":"plan"}"#).unwrap();
    let create_request = parse_create_note_request(r#"{"title":"title","body":"body"}"#).unwrap();
    let mut update_request =
        parse_update_note_request(r#"{"id":"note-1","title":"title","body":"body"}"#).unwrap();
    let mut delete_request = parse_delete_note_request(r#"{"id":"note-1"}"#).unwrap();

    // When: every registered command reaches its post-parse command executor
    let saved_page = execute_save_daily_page(&storage, save_request).unwrap();
    let loaded_page = execute_get_daily_page(&storage, get_request).unwrap();
    let created_note = execute_create_note(&storage, create_request).unwrap();
    update_request.id = created_note.id.clone();
    let updated_note = execute_update_note(&storage, update_request).unwrap();
    delete_request.id = updated_note.id.clone();
    let listed_notes = execute_list_notes(&storage).unwrap();
    execute_delete_note(&storage, delete_request).unwrap();

    // Then: all six use only typed records and no generic native capability
    assert_eq!(saved_page, loaded_page);
    assert_eq!(created_note.title.0, "title");
    assert_eq!(updated_note.id, created_note.id);
    assert_eq!(listed_notes, vec![updated_note]);
    assert!(execute_list_notes(&storage).unwrap().is_empty());
    fs::remove_dir_all(storage_directory).unwrap();
}

fn temporary_storage_directory() -> PathBuf {
    let directory = std::env::temp_dir().join(format!("notes-planner-ipc-{}", std::process::id()));
    let _ = fs::remove_dir_all(&directory);
    fs::create_dir_all(&directory).unwrap();
    directory
}
