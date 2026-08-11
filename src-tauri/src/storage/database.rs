use super::{
    connection::{self, TableName},
    migrations::{self, DATABASE_VERSION},
};
use crate::{
    dto::{
        CivilDateInput, CreateNoteRequest, DailyPageDto, DeleteNoteRequest, NoteBody, NoteDto,
        NoteId, NoteTitle, SaveDailyPageRequest, UpdateNoteRequest,
    },
    error::AppError,
};
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};

mod outline;
mod templates;
use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

// allow: SIZE_OK — the connection-owner configuration belongs with its SQLite persistence API.
pub struct DatabaseOptions {
    pub(super) path: PathBuf,
    pub(super) target_version: u32,
    pub(super) injected_migration_failure: Option<u32>,
    pub(super) injected_migration_failure_after_first_statement: Option<u32>,
    pub(super) injected_write_failure: bool,
    pub(super) injected_template_delete_failure: bool,
}

impl DatabaseOptions {
    pub fn for_path(path: PathBuf) -> Self {
        Self {
            path,
            target_version: DATABASE_VERSION,
            injected_migration_failure: None,
            injected_migration_failure_after_first_statement: None,
            injected_write_failure: false,
            injected_template_delete_failure: false,
        }
    }

    pub fn with_target_version(mut self, target_version: u32) -> Self {
        self.target_version = target_version;
        self
    }

    pub fn with_injected_migration_failure(mut self, version: u32) -> Self {
        self.injected_migration_failure = Some(version);
        self
    }

    pub fn with_injected_migration_failure_after_first_statement(mut self, version: u32) -> Self {
        self.injected_migration_failure_after_first_statement = Some(version);
        self
    }

    pub fn with_injected_write_failure(mut self) -> Self {
        self.injected_write_failure = true;
        self
    }

    pub fn with_injected_template_delete_failure(mut self) -> Self {
        self.injected_template_delete_failure = true;
        self
    }
}

pub struct Database {
    connection: Connection,
    injected_write_failure: bool,
    injected_template_delete_failure: bool,
}

impl Database {
    pub fn open(path: PathBuf) -> Result<Self, AppError> {
        Self::open_with_options(DatabaseOptions::for_path(path))
    }

    pub fn open_with_options(options: DatabaseOptions) -> Result<Self, AppError> {
        let parent = options
            .path
            .parent()
            .ok_or_else(AppError::storage_open_failed)?;
        fs::create_dir_all(parent).map_err(|_| AppError::storage_open_failed())?;
        let mut connection =
            Connection::open(&options.path).map_err(|_| AppError::storage_open_failed())?;
        connection::configure(&connection)?;
        migrations::apply(&mut connection, &options)?;
        Ok(Self {
            connection,
            injected_write_failure: options.injected_write_failure,
            injected_template_delete_failure: options.injected_template_delete_failure,
        })
    }

    pub fn get_daily_page(&mut self, date: &CivilDateInput) -> Result<DailyPageDto, AppError> {
        let date = date.as_str()?;
        if let Some(page) = self
            .connection
            .query_row(
                "SELECT date, content, created_at_ms, updated_at_ms FROM daily_pages WHERE date = ?1",
                params![date],
                |row| {
                    Ok(DailyPageDto {
                        date: CivilDateInput::parse(&row.get::<_, String>(0)?)
                            .map_err(|_| rusqlite::Error::InvalidQuery)?,
                        content: row.get(1)?,
                        created_at_ms: row.get(2)?,
                        updated_at_ms: row.get(3)?,
                    })
                },
            )
            .optional()
            .map_err(|_| AppError::storage_read_failed())?
        {
            return Ok(page);
        }

        let timestamp = now_ms()?;
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| AppError::storage_write_failed())?;
        transaction
            .execute(
                "INSERT INTO daily_pages (date, content, created_at_ms, updated_at_ms) VALUES (?1, '', ?2, ?2) \
                 ON CONFLICT(date) DO NOTHING",
                params![date, timestamp],
            )
            .map_err(|_| AppError::storage_write_failed())?;
        transaction
            .commit()
            .map_err(|_| AppError::storage_write_failed())?;
        Ok(DailyPageDto {
            date: CivilDateInput::parse(date)?,
            content: String::new(),
            created_at_ms: timestamp,
            updated_at_ms: timestamp,
        })
    }

    pub fn save_daily_page(
        &mut self,
        request: &SaveDailyPageRequest,
    ) -> Result<DailyPageDto, AppError> {
        let date = request.date.as_str()?;
        let timestamp = now_ms()?;
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| AppError::storage_write_failed())?;
        transaction.execute(
            "INSERT INTO daily_pages (date, content, created_at_ms, updated_at_ms) VALUES (?1, ?2, ?3, ?3) \
             ON CONFLICT(date) DO UPDATE SET content = excluded.content, updated_at_ms = excluded.updated_at_ms",
            params![date, request.content, timestamp],
        ).map_err(|_| AppError::storage_write_failed())?;
        if self.injected_write_failure {
            return Err(AppError::storage_write_failed());
        }
        transaction
            .commit()
            .map_err(|_| AppError::storage_write_failed())?;
        self.get_daily_page(&request.date)
    }

    pub fn list_notes(&self) -> Result<Vec<NoteDto>, AppError> {
        let mut statement = self
            .connection
            .prepare(
                "SELECT id, title, body FROM undated_notes ORDER BY updated_at_ms DESC, id ASC",
            )
            .map_err(|_| AppError::storage_read_failed())?;
        let notes = statement
            .query_map([], |row| {
                Ok(NoteDto {
                    id: NoteId(row.get(0)?),
                    title: NoteTitle(row.get(1)?),
                    body: NoteBody(row.get(2)?),
                })
            })
            .map_err(|_| AppError::storage_read_failed())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| AppError::storage_read_failed())?;
        Ok(notes)
    }

    pub fn create_note(&mut self, request: &CreateNoteRequest) -> Result<NoteDto, AppError> {
        let note = NoteDto {
            id: NoteId(uuid::Uuid::new_v4().to_string()),
            title: request.title.clone(),
            body: request.body.clone(),
        };
        let timestamp = now_ms()?;
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| AppError::storage_write_failed())?;
        transaction.execute(
            "INSERT INTO undated_notes (id, title, body, created_at_ms, updated_at_ms) VALUES (?1, ?2, ?3, ?4, ?4)",
            params![note.id.0, note.title.0, note.body.0, timestamp],
        ).map_err(|_| AppError::storage_write_failed())?;
        transaction
            .commit()
            .map_err(|_| AppError::storage_write_failed())?;
        Ok(note)
    }

    pub fn update_note(&mut self, request: &UpdateNoteRequest) -> Result<NoteDto, AppError> {
        let timestamp = now_ms()?;
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| AppError::storage_write_failed())?;
        let changed_rows = transaction
            .execute(
                "UPDATE undated_notes SET title = ?1, body = ?2, updated_at_ms = ?3 WHERE id = ?4",
                params![request.title.0, request.body.0, timestamp, request.id.0],
            )
            .map_err(|_| AppError::storage_write_failed())?;
        if changed_rows != 1 {
            return Err(AppError::storage_not_found());
        }
        transaction
            .commit()
            .map_err(|_| AppError::storage_write_failed())?;
        Ok(NoteDto {
            id: request.id.clone(),
            title: request.title.clone(),
            body: request.body.clone(),
        })
    }

    pub fn delete_note(&mut self, request: &DeleteNoteRequest) -> Result<(), AppError> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| AppError::storage_write_failed())?;
        let changed_rows = transaction
            .execute(
                "DELETE FROM undated_notes WHERE id = ?1",
                params![request.id.0],
            )
            .map_err(|_| AppError::storage_write_failed())?;
        if changed_rows != 1 {
            return Err(AppError::storage_not_found());
        }
        transaction
            .commit()
            .map_err(|_| AppError::storage_write_failed())
    }

    pub fn schema_version(&self) -> Result<u32, AppError> {
        connection::schema_version(&self.connection)
    }

    pub fn table_columns(&self, table: TableName) -> Result<Vec<String>, AppError> {
        connection::table_columns(&self.connection, table)
    }

    pub fn foreign_keys_enabled(&self) -> Result<bool, AppError> {
        connection::foreign_keys_enabled(&self.connection)
    }

    pub fn journal_mode(&self) -> Result<String, AppError> {
        connection::journal_mode(&self.connection)
    }
}

fn now_ms() -> Result<i64, AppError> {
    let milliseconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| AppError::storage_write_failed())?
        .as_millis();
    i64::try_from(milliseconds).map_err(|_| AppError::storage_write_failed())
}
