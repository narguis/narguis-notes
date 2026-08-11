use super::Database;
use crate::{
    dto::{
        CivilDateInput, CreatePlannerLineRequest, DeletePlannerLineRequest, MovePlannerLineRequest,
        PlannerLineDto, PlannerLineId, SetPlannerLineCollapsedRequest, SetPlannerLineTimeRequest,
        TimeOfDayMinutes, UpdatePlannerLineRequest,
    },
    error::AppError,
};
use rusqlite::{params, OptionalExtension, TransactionBehavior};
use std::collections::HashMap;

#[derive(Debug)]
struct StoredLine {
    dto: PlannerLineDto,
}

fn checked_time(value: Option<TimeOfDayMinutes>) -> Result<Option<i64>, AppError> {
    match value {
        Some(minutes) if minutes.0 >= 1440 => Err(AppError::invalid_time_of_day()),
        Some(minutes) => Ok(Some(i64::from(minutes.0))),
        None => Ok(None),
    }
}

fn checked_identifier(id: &PlannerLineId) -> Result<(), AppError> {
    uuid::Uuid::parse_str(&id.0)
        .map(|_| ())
        .map_err(|_| AppError::invalid_identifier())
}

fn checked_key(key: &str) -> Result<(), AppError> {
    if key.is_empty() || !key.bytes().all(|byte| byte.is_ascii_alphanumeric()) {
        Err(AppError::invalid_sibling_key())
    } else {
        Ok(())
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

fn load_line(
    connection: &rusqlite::Connection,
    id: &PlannerLineId,
) -> Result<Option<StoredLine>, AppError> {
    connection
        .query_row(
            "SELECT id, date, parent_id, sibling_key, title, description, time_of_day_minutes, is_collapsed FROM planner_lines WHERE id = ?1",
            params![id.0],
            |row| {
                let time: Option<i64> = row.get(6)?;
                Ok(StoredLine { dto: PlannerLineDto {
                    id: PlannerLineId(row.get(0)?),
                    date: CivilDateInput::parse(&row.get::<_, String>(1)?).map_err(|_| rusqlite::Error::InvalidQuery)?,
                    parent_id: row.get::<_, Option<String>>(2)?.map(PlannerLineId),
                    sibling_key: crate::dto::SiblingKey(row.get(3)?),
                    title: crate::dto::PlannerLineTitle(row.get(4)?),
                    description: row.get::<_, Option<String>>(5)?.map(crate::dto::PlannerLineDescription),
                    time_of_day_minutes: read_time(time)?,
                    is_collapsed: row.get::<_, i64>(7)? != 0,
                }})
            },
        )
        .optional()
        .map_err(|_| AppError::storage_read_failed())
}

impl Database {
    pub fn list_planner_lines(
        &mut self,
        date: &CivilDateInput,
    ) -> Result<Vec<PlannerLineDto>, AppError> {
        let date = date.as_str()?;
        let mut statement = self.connection.prepare(
            "SELECT id, date, parent_id, sibling_key, title, description, time_of_day_minutes, is_collapsed FROM planner_lines WHERE date = ?1 ORDER BY sibling_key COLLATE BINARY ASC, id ASC",
        ).map_err(|_| AppError::storage_read_failed())?;
        let rows = statement
            .query_map(params![date], |row| {
                let time: Option<i64> = row.get(6)?;
                Ok(StoredLine {
                    dto: PlannerLineDto {
                        id: PlannerLineId(row.get(0)?),
                        date: CivilDateInput::parse(&row.get::<_, String>(1)?)
                            .map_err(|_| rusqlite::Error::InvalidQuery)?,
                        parent_id: row.get::<_, Option<String>>(2)?.map(PlannerLineId),
                        sibling_key: crate::dto::SiblingKey(row.get(3)?),
                        title: crate::dto::PlannerLineTitle(row.get(4)?),
                        description: row
                            .get::<_, Option<String>>(5)?
                            .map(crate::dto::PlannerLineDescription),
                        time_of_day_minutes: read_time(time)?,
                        is_collapsed: row.get::<_, i64>(7)? != 0,
                    },
                })
            })
            .map_err(|_| AppError::storage_read_failed())?;
        let rows = rows
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| AppError::storage_read_failed())?;
        let mut by_parent: HashMap<Option<String>, Vec<PlannerLineDto>> = HashMap::new();
        for row in rows {
            by_parent
                .entry(row.dto.parent_id.as_ref().map(|id| id.0.clone()))
                .or_default()
                .push(row.dto);
        }
        for children in by_parent.values_mut() {
            children.sort_by(|a, b| {
                a.sibling_key
                    .0
                    .as_bytes()
                    .cmp(b.sibling_key.0.as_bytes())
                    .then_with(|| a.id.0.cmp(&b.id.0))
            });
        }
        fn visit(
            parent: Option<String>,
            by_parent: &mut HashMap<Option<String>, Vec<PlannerLineDto>>,
            output: &mut Vec<PlannerLineDto>,
        ) {
            if let Some(children) = by_parent.remove(&parent) {
                for child in children {
                    let id = child.id.0.clone();
                    output.push(child);
                    visit(Some(id), by_parent, output);
                }
            }
        }
        let mut output = Vec::new();
        visit(None, &mut by_parent, &mut output);
        Ok(output)
    }

    pub fn create_planner_line(
        &mut self,
        request: &CreatePlannerLineRequest,
    ) -> Result<PlannerLineDto, AppError> {
        request.date.as_str()?;
        checked_key(&request.sibling_key.0)?;
        let time = checked_time(request.time_of_day_minutes)?;
        if let Some(parent) = &request.parent_id {
            checked_identifier(parent)?;
            let parent_line = load_line(&self.connection, parent)?
                .ok_or_else(AppError::invalid_planner_line_parent)?;
            if parent_line.dto.date != request.date {
                return Err(AppError::invalid_planner_line_parent());
            }
        }
        let id = PlannerLineId(uuid::Uuid::new_v4().to_string());
        let timestamp = super::now_ms()?;
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| AppError::storage_write_failed())?;
        transaction.execute("INSERT INTO planner_lines (id, date, parent_id, sibling_key, title, description, time_of_day_minutes, is_collapsed, created_at_ms, updated_at_ms) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 0, ?8, ?8)", params![id.0, request.date.as_str()?, request.parent_id.as_ref().map(|value| &value.0), request.sibling_key.0, request.title.0, request.description.as_ref().map(|value| &value.0), time, timestamp]).map_err(|_| AppError::storage_write_failed())?;
        transaction
            .commit()
            .map_err(|_| AppError::storage_write_failed())?;
        Ok(PlannerLineDto {
            id,
            date: request.date.clone(),
            parent_id: request.parent_id.clone(),
            sibling_key: request.sibling_key.clone(),
            title: request.title.clone(),
            description: request.description.clone(),
            time_of_day_minutes: request.time_of_day_minutes,
            is_collapsed: false,
        })
    }

    pub fn update_planner_line(
        &mut self,
        request: &UpdatePlannerLineRequest,
    ) -> Result<(), AppError> {
        checked_identifier(&request.id)?;
        let time = checked_time(request.time_of_day_minutes)?;
        let timestamp = super::now_ms()?;
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| AppError::storage_write_failed())?;
        let changed = transaction.execute("UPDATE planner_lines SET title = ?1, description = ?2, time_of_day_minutes = ?3, updated_at_ms = ?4 WHERE id = ?5", params![request.title.0, request.description.as_ref().map(|value| &value.0), time, timestamp, request.id.0]).map_err(|_| AppError::storage_write_failed())?;
        if changed != 1 {
            return Err(AppError::storage_not_found());
        }
        transaction
            .commit()
            .map_err(|_| AppError::storage_write_failed())
    }

    pub fn delete_planner_line(
        &mut self,
        request: &DeletePlannerLineRequest,
    ) -> Result<(), AppError> {
        checked_identifier(&request.id)?;
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| AppError::storage_write_failed())?;
        let changed = transaction
            .execute(
                "DELETE FROM planner_lines WHERE id = ?1",
                params![request.id.0],
            )
            .map_err(|_| AppError::storage_write_failed())?;
        if changed != 1 {
            return Err(AppError::storage_not_found());
        }
        transaction
            .commit()
            .map_err(|_| AppError::storage_write_failed())
    }

    pub fn move_planner_line(&mut self, request: &MovePlannerLineRequest) -> Result<(), AppError> {
        checked_identifier(&request.id)?;
        checked_key(&request.sibling_key.0)?;
        let line =
            load_line(&self.connection, &request.id)?.ok_or_else(AppError::storage_not_found)?;
        if let Some(parent) = &request.parent_id {
            checked_identifier(parent)?;
            let parent_line = load_line(&self.connection, parent)?
                .ok_or_else(AppError::invalid_planner_line_parent)?;
            if parent_line.dto.date != line.dto.date || parent.0 == request.id.0 {
                return Err(AppError::invalid_planner_line_parent());
            }
            let mut cursor = Some(parent.clone());
            while let Some(candidate) = cursor {
                if candidate.0 == request.id.0 {
                    return Err(AppError::invalid_planner_line_parent());
                }
                cursor =
                    load_line(&self.connection, &candidate)?.and_then(|value| value.dto.parent_id);
            }
        }
        let timestamp = super::now_ms()?;
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| AppError::storage_write_failed())?;
        transaction.execute("UPDATE planner_lines SET parent_id = ?1, sibling_key = ?2, updated_at_ms = ?3 WHERE id = ?4", params![request.parent_id.as_ref().map(|value| &value.0), request.sibling_key.0, timestamp, request.id.0]).map_err(|_| AppError::storage_write_failed())?;
        transaction
            .commit()
            .map_err(|_| AppError::storage_write_failed())
    }

    pub fn set_planner_line_collapsed(
        &mut self,
        request: &SetPlannerLineCollapsedRequest,
    ) -> Result<(), AppError> {
        checked_identifier(&request.id)?;
        let timestamp = super::now_ms()?;
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| AppError::storage_write_failed())?;
        let changed = transaction
            .execute(
                "UPDATE planner_lines SET is_collapsed = ?1, updated_at_ms = ?2 WHERE id = ?3",
                params![request.is_collapsed as i64, timestamp, request.id.0],
            )
            .map_err(|_| AppError::storage_write_failed())?;
        if changed != 1 {
            return Err(AppError::storage_not_found());
        }
        transaction
            .commit()
            .map_err(|_| AppError::storage_write_failed())
    }

    pub fn set_planner_line_time(
        &mut self,
        request: &SetPlannerLineTimeRequest,
    ) -> Result<(), AppError> {
        checked_identifier(&request.id)?;
        let time = checked_time(request.time_of_day_minutes)?;
        let timestamp = super::now_ms()?;
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| AppError::storage_write_failed())?;
        let changed = transaction.execute("UPDATE planner_lines SET time_of_day_minutes = ?1, updated_at_ms = ?2 WHERE id = ?3", params![time, timestamp, request.id.0]).map_err(|_| AppError::storage_write_failed())?;
        if changed != 1 {
            return Err(AppError::storage_not_found());
        }
        transaction
            .commit()
            .map_err(|_| AppError::storage_write_failed())
    }
}
