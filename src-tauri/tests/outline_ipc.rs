use notes_planner_desktop::{
    error::AppErrorCode,
    ipc::{
        parse_create_planner_line_request, parse_create_task_template_request,
        parse_move_planner_line_request, parse_set_planner_line_time_request,
    },
};

fn error_code<T: std::fmt::Debug>(
    result: Result<T, notes_planner_desktop::error::AppError>,
) -> AppErrorCode {
    result.expect_err("payload should be rejected").code
}

#[test]
fn rejects_malformed_outline_payloads_at_the_typed_ipc_boundary() {
    // Given: invalid UUIDs, unknown fields, invalid fractional keys, and out-of-range times
    let invalid_parent = r#"{"date":"2026-07-30","parentId":"not-a-uuid","siblingKey":"a","text":"line","timeOfDayMinutes":null}"#;
    let unknown_field = r#"{"date":"2026-07-30","parentId":null,"siblingKey":"a!","text":"line","timeOfDayMinutes":0,"sql":"DROP TABLE"}"#;
    let invalid_time = r#"{"id":"018e8c7b-7f10-7cc4-8a76-50d7f8df3b11","timeOfDayMinutes":1440}"#;
    let malformed_move =
        r#"{"id":"018e8c7b-7f10-7cc4-8a76-50d7f8df3b11","parentId":null,"siblingKey":""}"#;
    let invalid_template_time = r#"{"title":"template","body":"line","timeOfDayMinutes":1440}"#;

    // When: untrusted JSON is parsed before it can reach a command or SQLite
    let parent_error = error_code(parse_create_planner_line_request(invalid_parent));
    let unknown_error = error_code(parse_create_planner_line_request(unknown_field));
    let time_error = error_code(parse_set_planner_line_time_request(invalid_time));
    let move_error = error_code(parse_move_planner_line_request(malformed_move));
    let template_error = error_code(parse_create_task_template_request(invalid_template_time));

    // Then: every malformed payload has a stable typed rejection code
    assert_eq!(parent_error, AppErrorCode::InvalidIdentifier);
    assert_eq!(unknown_error, AppErrorCode::InvalidPayload);
    assert_eq!(time_error, AppErrorCode::InvalidTimeOfDay);
    assert_eq!(move_error, AppErrorCode::InvalidSiblingKey);
    assert_eq!(template_error, AppErrorCode::InvalidTimeOfDay);
}
