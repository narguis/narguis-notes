use super::Database;
use crate::{
    dto::{
        CreateTaskTemplateRequest, InsertTaskTemplateCopyRequest, PlannerLineDescription,
        PlannerLineDto, PlannerLineId, PlannerLineTitle, TaskTemplateBody, TaskTemplateDto,
        TaskTemplateId, TaskTemplateTitle, TimeOfDayMinutes, UpdateTaskTemplateRequest,
    },
    error::AppError,
};
use rusqlite::{params, OptionalExtension, TransactionBehavior};

fn checked_id(id: &str) -> Result<(), AppError> {
    uuid::Uuid::parse_str(id)
        .map(|_| ())
        .map_err(|_| AppError::invalid_identifier())
}
fn checked_time(value: Option<TimeOfDayMinutes>) -> Result<Option<i64>, AppError> {
    match value {
        Some(value) if value.0 >= 1440 => Err(AppError::invalid_time_of_day()),
        Some(value) => Ok(Some(i64::from(value.0))),
        None => Ok(None),
    }
}

fn read_time(value: Option<i64>) -> Result<Option<TimeOfDayMinutes>, rusqlite::Error> {
    value
        .map(|minutes| {
            u16::try_from(minutes)
                .map(TimeOfDayMinutes)
                .map_err(|_| rusqlite::Error::InvalidQuery)
        })
        .transpose()
}

impl Database {
    pub fn list_task_templates(&self) -> Result<Vec<TaskTemplateDto>, AppError> {
        let mut statement = self.connection.prepare("SELECT id, title, body, time_of_day_minutes FROM task_templates ORDER BY updated_at_ms DESC, id ASC").map_err(|_| AppError::storage_read_failed())?;
        let templates = statement
            .query_map([], |row| {
                let time: Option<i64> = row.get(3)?;
                Ok(TaskTemplateDto {
                    id: TaskTemplateId(row.get(0)?),
                    title: TaskTemplateTitle(row.get(1)?),
                    body: TaskTemplateBody(row.get(2)?),
                    time_of_day_minutes: read_time(time)?,
                })
            })
            .map_err(|_| AppError::storage_read_failed())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| AppError::storage_read_failed());
        templates
    }

    pub fn create_task_template(
        &mut self,
        request: &CreateTaskTemplateRequest,
    ) -> Result<TaskTemplateDto, AppError> {
        let time = checked_time(request.time_of_day_minutes.map(TimeOfDayMinutes))?;
        let id = TaskTemplateId(uuid::Uuid::new_v4().to_string());
        let timestamp = super::now_ms()?;
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| AppError::storage_write_failed())?;
        transaction.execute("INSERT INTO task_templates (id, title, body, time_of_day_minutes, created_at_ms, updated_at_ms) VALUES (?1, ?2, ?3, ?4, ?5, ?5)", params![id.0, request.title.0, request.body.0, time, timestamp]).map_err(|_| AppError::storage_write_failed())?;
        transaction
            .commit()
            .map_err(|_| AppError::storage_write_failed())?;
        Ok(TaskTemplateDto {
            id,
            title: request.title.clone(),
            body: request.body.clone(),
            time_of_day_minutes: request.time_of_day_minutes.map(TimeOfDayMinutes),
        })
    }

    pub fn update_task_template(
        &mut self,
        request: &UpdateTaskTemplateRequest,
    ) -> Result<(), AppError> {
        checked_id(&request.id.0)?;
        let time = checked_time(request.time_of_day_minutes.map(TimeOfDayMinutes))?;
        let timestamp = super::now_ms()?;
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| AppError::storage_write_failed())?;
        let changed = transaction.execute("UPDATE task_templates SET title = ?1, body = ?2, time_of_day_minutes = ?3, updated_at_ms = ?4 WHERE id = ?5", params![request.title.0, request.body.0, time, timestamp, request.id.0]).map_err(|_| AppError::storage_write_failed())?;
        if changed != 1 {
            return Err(AppError::storage_not_found());
        }
        transaction
            .commit()
            .map_err(|_| AppError::storage_write_failed())
    }

    pub fn delete_task_template(&mut self, id: &TaskTemplateId) -> Result<(), AppError> {
        checked_id(&id.0)?;
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| AppError::storage_write_failed())?;
        let changed = transaction
            .execute("DELETE FROM task_templates WHERE id = ?1", params![id.0])
            .map_err(|_| AppError::storage_write_failed())?;
        if self.injected_template_delete_failure {
            return Err(AppError::storage_write_failed());
        }
        if changed != 1 {
            return Err(AppError::storage_not_found());
        }
        transaction
            .commit()
            .map_err(|_| AppError::storage_write_failed())
    }

    pub fn insert_task_template_copy(
        &mut self,
        request: &InsertTaskTemplateCopyRequest,
    ) -> Result<PlannerLineDto, AppError> {
        checked_id(&request.template_id.0)?;
        request.date.as_str()?;
        if request.sibling_key.0.is_empty()
            || !request
                .sibling_key
                .0
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric())
        {
            return Err(AppError::invalid_sibling_key());
        }
        let time_and_values: (String, String, Option<i64>) = self
            .connection
            .query_row(
                "SELECT title, body, time_of_day_minutes FROM task_templates WHERE id = ?1",
                params![request.template_id.0],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .optional()
            .map_err(|_| AppError::storage_read_failed())?
            .ok_or_else(AppError::storage_not_found)?;
        let id = PlannerLineId(uuid::Uuid::new_v4().to_string());
        let timestamp = super::now_ms()?;
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| AppError::storage_write_failed())?;
        transaction.execute("INSERT INTO planner_lines (id, date, parent_id, sibling_key, title, description, time_of_day_minutes, is_collapsed, created_at_ms, updated_at_ms) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 0, ?8, ?8)", params![id.0, request.date.as_str()?, request.parent_id.as_ref().map(|value| &value.0), request.sibling_key.0, time_and_values.0, time_and_values.1, time_and_values.2, timestamp]).map_err(|_| AppError::storage_write_failed())?;
        transaction
            .commit()
            .map_err(|_| AppError::storage_write_failed())?;
        Ok(PlannerLineDto {
            id,
            date: request.date.clone(),
            parent_id: request.parent_id.clone(),
            sibling_key: request.sibling_key.clone(),
            title: PlannerLineTitle(time_and_values.0),
            description: Some(PlannerLineDescription(time_and_values.1)),
            time_of_day_minutes: time_and_values
                .2
                .map(|value| TimeOfDayMinutes(u16::try_from(value).unwrap_or(0))),
            is_collapsed: false,
        })
    }
}
