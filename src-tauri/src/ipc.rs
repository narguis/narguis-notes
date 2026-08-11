use crate::{
    dto::{
        CreateNoteRequest, CreatePlannerLineRequest, CreateTaskTemplateRequest, DeleteNoteRequest,
        DeletePlannerLineRequest, DeleteTaskTemplateRequest, GetDailyPageRequest,
        InsertTaskTemplateCopyRequest, MovePlannerLineRequest, SaveDailyPageRequest,
        SetPlannerLineCollapsedRequest, SetPlannerLineTimeRequest, UpdateNoteRequest,
        UpdatePlannerLineRequest, UpdateTaskTemplateRequest, Validate,
    },
    error::AppError,
};
use serde::de::DeserializeOwned;
use tauri::ipc::{InvokeBody, Request};

pub use crate::error::AppErrorCode;

pub fn parse_get_daily_page_request(payload: &str) -> Result<GetDailyPageRequest, AppError> {
    parse_payload(payload)
}

pub fn parse_create_note_request(payload: &str) -> Result<CreateNoteRequest, AppError> {
    parse_payload(payload)
}

pub fn parse_save_daily_page_request(payload: &str) -> Result<SaveDailyPageRequest, AppError> {
    parse_payload(payload)
}

pub fn parse_update_note_request(payload: &str) -> Result<UpdateNoteRequest, AppError> {
    parse_payload(payload)
}

pub fn parse_delete_note_request(payload: &str) -> Result<DeleteNoteRequest, AppError> {
    parse_payload(payload)
}

pub fn parse_create_planner_line_request(
    payload: &str,
) -> Result<CreatePlannerLineRequest, AppError> {
    parse_payload(payload)
}
pub fn parse_update_planner_line_request(
    payload: &str,
) -> Result<UpdatePlannerLineRequest, AppError> {
    parse_payload(payload)
}
pub fn parse_delete_planner_line_request(
    payload: &str,
) -> Result<DeletePlannerLineRequest, AppError> {
    parse_payload(payload)
}
pub fn parse_move_planner_line_request(payload: &str) -> Result<MovePlannerLineRequest, AppError> {
    parse_payload(payload)
}
pub fn parse_set_planner_line_collapsed_request(
    payload: &str,
) -> Result<SetPlannerLineCollapsedRequest, AppError> {
    parse_payload(payload)
}
pub fn parse_set_planner_line_time_request(
    payload: &str,
) -> Result<SetPlannerLineTimeRequest, AppError> {
    parse_payload(payload)
}
pub fn parse_create_task_template_request(
    payload: &str,
) -> Result<CreateTaskTemplateRequest, AppError> {
    parse_payload(payload)
}
pub fn parse_update_task_template_request(
    payload: &str,
) -> Result<UpdateTaskTemplateRequest, AppError> {
    parse_payload(payload)
}
pub fn parse_delete_task_template_request(
    payload: &str,
) -> Result<DeleteTaskTemplateRequest, AppError> {
    parse_payload(payload)
}
pub fn parse_insert_task_template_copy_request(
    payload: &str,
) -> Result<InsertTaskTemplateCopyRequest, AppError> {
    parse_payload(payload)
}

pub(crate) fn parse_request<T>(request: Request<'_>) -> Result<T, AppError>
where
    T: DeserializeOwned + Validate,
{
    match request.body() {
        InvokeBody::Json(payload) => parse_json_value(payload.clone()),
        InvokeBody::Raw(_) => Err(AppError::invalid_payload()),
    }
}

fn parse_payload<T>(payload: &str) -> Result<T, AppError>
where
    T: DeserializeOwned + Validate,
{
    let request = serde_json::from_str::<T>(payload).map_err(|_| AppError::invalid_payload())?;
    request.validate()?;
    Ok(request)
}

fn parse_json_value<T>(payload: serde_json::Value) -> Result<T, AppError>
where
    T: DeserializeOwned + Validate,
{
    let request = serde_json::from_value::<T>(payload).map_err(|_| AppError::invalid_payload())?;
    request.validate()?;
    Ok(request)
}
