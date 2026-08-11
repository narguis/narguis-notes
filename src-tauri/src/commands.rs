use crate::{
    civil_date::CivilDate,
    dto::{
        CreateNoteRequest, CreatePlannerLineRequest, CreateTaskTemplateRequest, DailyPageDto,
        DeleteNoteRequest, DeletePlannerLineRequest, DeleteTaskTemplateRequest,
        GetDailyPageRequest, InsertTaskTemplateCopyRequest, MovePlannerLineRequest, NoteDto,
        PlannerLineDto, SaveDailyPageRequest, SetPlannerLineCollapsedRequest,
        SetPlannerLineTimeRequest, TaskTemplateDto, UpdateNoteRequest, UpdatePlannerLineRequest,
        UpdateTaskTemplateRequest,
    },
    error::AppError,
    ipc,
    storage::Storage,
};
use tauri::{ipc::Request, State};

#[tauri::command]
pub fn get_local_today() -> Result<String, AppError> {
    Ok(CivilDate::today()?.as_str().to_owned())
}

#[tauri::command]
pub fn get_daily_page(
    storage: State<'_, Storage>,
    request: Request<'_>,
) -> Result<DailyPageDto, AppError> {
    execute_get_daily_page(storage.inner(), ipc::parse_request(request)?)
}

pub fn execute_get_daily_page(
    storage: &Storage,
    request: GetDailyPageRequest,
) -> Result<DailyPageDto, AppError> {
    storage.get_daily_page(&request.date)
}

#[tauri::command]
pub fn save_daily_page(
    storage: State<'_, Storage>,
    request: Request<'_>,
) -> Result<DailyPageDto, AppError> {
    execute_save_daily_page(storage.inner(), ipc::parse_request(request)?)
}

pub fn execute_save_daily_page(
    storage: &Storage,
    request: SaveDailyPageRequest,
) -> Result<DailyPageDto, AppError> {
    storage.save_daily_page(&request)
}

#[tauri::command]
pub fn list_notes(storage: State<'_, Storage>) -> Result<Vec<NoteDto>, AppError> {
    execute_list_notes(storage.inner())
}

pub fn execute_list_notes(storage: &Storage) -> Result<Vec<NoteDto>, AppError> {
    storage.list_notes()
}

#[tauri::command]
pub fn create_note(storage: State<'_, Storage>, request: Request<'_>) -> Result<NoteDto, AppError> {
    execute_create_note(storage.inner(), ipc::parse_request(request)?)
}

pub fn execute_create_note(
    storage: &Storage,
    request: CreateNoteRequest,
) -> Result<NoteDto, AppError> {
    storage.create_note(&request)
}

#[tauri::command]
pub fn update_note(storage: State<'_, Storage>, request: Request<'_>) -> Result<NoteDto, AppError> {
    execute_update_note(storage.inner(), ipc::parse_request(request)?)
}

pub fn execute_update_note(
    storage: &Storage,
    request: UpdateNoteRequest,
) -> Result<NoteDto, AppError> {
    storage.update_note(&request)
}

#[tauri::command]
pub fn delete_note(storage: State<'_, Storage>, request: Request<'_>) -> Result<(), AppError> {
    execute_delete_note(storage.inner(), ipc::parse_request(request)?)
}

pub fn execute_delete_note(storage: &Storage, request: DeleteNoteRequest) -> Result<(), AppError> {
    storage.delete_note(&request)
}

#[tauri::command]
pub fn list_planner_lines(
    storage: State<'_, Storage>,
    request: Request<'_>,
) -> Result<Vec<PlannerLineDto>, AppError> {
    storage.list_planner_lines(&ipc::parse_request::<GetDailyPageRequest>(request)?.date)
}

#[tauri::command]
pub fn create_planner_line(
    storage: State<'_, Storage>,
    request: Request<'_>,
) -> Result<PlannerLineDto, AppError> {
    storage.create_planner_line(&ipc::parse_request::<CreatePlannerLineRequest>(request)?)
}

#[tauri::command]
pub fn update_planner_line(
    storage: State<'_, Storage>,
    request: Request<'_>,
) -> Result<(), AppError> {
    storage.update_planner_line(&ipc::parse_request::<UpdatePlannerLineRequest>(request)?)
}

#[tauri::command]
pub fn delete_planner_line(
    storage: State<'_, Storage>,
    request: Request<'_>,
) -> Result<(), AppError> {
    storage.delete_planner_line(&ipc::parse_request::<DeletePlannerLineRequest>(request)?)
}

#[tauri::command]
pub fn move_planner_line(
    storage: State<'_, Storage>,
    request: Request<'_>,
) -> Result<(), AppError> {
    storage.move_planner_line(&ipc::parse_request::<MovePlannerLineRequest>(request)?)
}

#[tauri::command]
pub fn set_planner_line_collapsed(
    storage: State<'_, Storage>,
    request: Request<'_>,
) -> Result<(), AppError> {
    storage.set_planner_line_collapsed(&ipc::parse_request::<SetPlannerLineCollapsedRequest>(
        request,
    )?)
}

#[tauri::command]
pub fn set_planner_line_time(
    storage: State<'_, Storage>,
    request: Request<'_>,
) -> Result<(), AppError> {
    storage.set_planner_line_time(&ipc::parse_request::<SetPlannerLineTimeRequest>(request)?)
}

#[tauri::command]
pub fn list_task_templates(storage: State<'_, Storage>) -> Result<Vec<TaskTemplateDto>, AppError> {
    storage.list_task_templates()
}

#[tauri::command]
pub fn create_task_template(
    storage: State<'_, Storage>,
    request: Request<'_>,
) -> Result<TaskTemplateDto, AppError> {
    storage.create_task_template(&ipc::parse_request::<CreateTaskTemplateRequest>(request)?)
}

#[tauri::command]
pub fn update_task_template(
    storage: State<'_, Storage>,
    request: Request<'_>,
) -> Result<(), AppError> {
    storage.update_task_template(&ipc::parse_request::<UpdateTaskTemplateRequest>(request)?)
}

#[tauri::command]
pub fn delete_task_template(
    storage: State<'_, Storage>,
    request: Request<'_>,
) -> Result<(), AppError> {
    let request = ipc::parse_request::<DeleteTaskTemplateRequest>(request)?;
    storage.delete_task_template(&request.id)
}

#[tauri::command]
pub fn insert_task_template_copy(
    storage: State<'_, Storage>,
    request: Request<'_>,
) -> Result<PlannerLineDto, AppError> {
    storage.insert_task_template_copy(&ipc::parse_request::<InsertTaskTemplateCopyRequest>(
        request,
    )?)
}
