mod connection;
mod database;
mod migrations;

use crate::{
    dto::{
        CivilDateInput, CreateNoteRequest, CreatePlannerLineRequest, CreateTaskTemplateRequest,
        DailyPageDto, DeleteNoteRequest, DeletePlannerLineRequest, InsertTaskTemplateCopyRequest,
        MovePlannerLineRequest, NoteDto, PlannerLineDto, SaveDailyPageRequest,
        SetPlannerLineCollapsedRequest, SetPlannerLineTimeRequest, TaskTemplateDto, TaskTemplateId,
        UpdateNoteRequest, UpdatePlannerLineRequest, UpdateTaskTemplateRequest,
    },
    error::AppError,
};
use std::{
    path::{Path, PathBuf},
    sync::Mutex,
};

pub use connection::TableName;
pub use database::{Database, DatabaseOptions};
pub use migrations::DATABASE_VERSION;

pub const DATABASE_FILE_NAME: &str = "notes-planner.sqlite3";

pub struct Storage {
    database: Mutex<Database>,
}

impl Storage {
    pub fn open_in_app_data_directory(app_data_directory: &Path) -> Result<Self, AppError> {
        std::fs::create_dir_all(app_data_directory).map_err(|_| AppError::storage_open_failed())?;
        Self::open(app_data_directory.join(DATABASE_FILE_NAME))
    }

    pub fn open(path: PathBuf) -> Result<Self, AppError> {
        Ok(Self {
            database: Mutex::new(Database::open(path)?),
        })
    }

    pub fn get_daily_page(&self, date: &CivilDateInput) -> Result<DailyPageDto, AppError> {
        self.database
            .lock()
            .map_err(|_| AppError::storage_read_failed())?
            .get_daily_page(date)
    }

    pub fn save_daily_page(
        &self,
        request: &SaveDailyPageRequest,
    ) -> Result<DailyPageDto, AppError> {
        self.database
            .lock()
            .map_err(|_| AppError::storage_write_failed())?
            .save_daily_page(request)
    }

    pub fn list_notes(&self) -> Result<Vec<NoteDto>, AppError> {
        self.database
            .lock()
            .map_err(|_| AppError::storage_read_failed())?
            .list_notes()
    }

    pub fn create_note(&self, request: &CreateNoteRequest) -> Result<NoteDto, AppError> {
        self.database
            .lock()
            .map_err(|_| AppError::storage_write_failed())?
            .create_note(request)
    }

    pub fn update_note(&self, request: &UpdateNoteRequest) -> Result<NoteDto, AppError> {
        self.database
            .lock()
            .map_err(|_| AppError::storage_write_failed())?
            .update_note(request)
    }

    pub fn delete_note(&self, request: &DeleteNoteRequest) -> Result<(), AppError> {
        self.database
            .lock()
            .map_err(|_| AppError::storage_write_failed())?
            .delete_note(request)
    }

    pub fn list_planner_lines(
        &self,
        date: &CivilDateInput,
    ) -> Result<Vec<PlannerLineDto>, AppError> {
        self.database
            .lock()
            .map_err(|_| AppError::storage_read_failed())?
            .list_planner_lines(date)
    }

    pub fn create_planner_line(
        &self,
        request: &CreatePlannerLineRequest,
    ) -> Result<PlannerLineDto, AppError> {
        self.database
            .lock()
            .map_err(|_| AppError::storage_write_failed())?
            .create_planner_line(request)
    }

    pub fn update_planner_line(&self, request: &UpdatePlannerLineRequest) -> Result<(), AppError> {
        self.database
            .lock()
            .map_err(|_| AppError::storage_write_failed())?
            .update_planner_line(request)
    }

    pub fn delete_planner_line(&self, request: &DeletePlannerLineRequest) -> Result<(), AppError> {
        self.database
            .lock()
            .map_err(|_| AppError::storage_write_failed())?
            .delete_planner_line(request)
    }

    pub fn move_planner_line(&self, request: &MovePlannerLineRequest) -> Result<(), AppError> {
        self.database
            .lock()
            .map_err(|_| AppError::storage_write_failed())?
            .move_planner_line(request)
    }

    pub fn set_planner_line_collapsed(
        &self,
        request: &SetPlannerLineCollapsedRequest,
    ) -> Result<(), AppError> {
        self.database
            .lock()
            .map_err(|_| AppError::storage_write_failed())?
            .set_planner_line_collapsed(request)
    }

    pub fn set_planner_line_time(
        &self,
        request: &SetPlannerLineTimeRequest,
    ) -> Result<(), AppError> {
        self.database
            .lock()
            .map_err(|_| AppError::storage_write_failed())?
            .set_planner_line_time(request)
    }

    pub fn list_task_templates(&self) -> Result<Vec<TaskTemplateDto>, AppError> {
        self.database
            .lock()
            .map_err(|_| AppError::storage_read_failed())?
            .list_task_templates()
    }

    pub fn create_task_template(
        &self,
        request: &CreateTaskTemplateRequest,
    ) -> Result<TaskTemplateDto, AppError> {
        self.database
            .lock()
            .map_err(|_| AppError::storage_write_failed())?
            .create_task_template(request)
    }

    pub fn update_task_template(
        &self,
        request: &UpdateTaskTemplateRequest,
    ) -> Result<(), AppError> {
        self.database
            .lock()
            .map_err(|_| AppError::storage_write_failed())?
            .update_task_template(request)
    }

    pub fn delete_task_template(&self, id: &TaskTemplateId) -> Result<(), AppError> {
        self.database
            .lock()
            .map_err(|_| AppError::storage_write_failed())?
            .delete_task_template(id)
    }

    pub fn insert_task_template_copy(
        &self,
        request: &InsertTaskTemplateCopyRequest,
    ) -> Result<PlannerLineDto, AppError> {
        self.database
            .lock()
            .map_err(|_| AppError::storage_write_failed())?
            .insert_task_template_copy(request)
    }
}
