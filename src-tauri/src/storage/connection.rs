use crate::error::AppError;
use rusqlite::Connection;

pub enum TableName {
    DailyPages,
    UndatedNotes,
}

impl TableName {
    const fn sql_name(&self) -> &'static str {
        match self {
            Self::DailyPages => "daily_pages",
            Self::UndatedNotes => "undated_notes",
        }
    }
}

pub(super) fn configure(connection: &Connection) -> Result<(), AppError> {
    connection
        .pragma_update(None, "foreign_keys", "ON")
        .map_err(|_| AppError::storage_open_failed())?;
    if !foreign_keys_enabled(connection).map_err(|_| AppError::storage_open_failed())? {
        return Err(AppError::storage_open_failed());
    }
    connection
        .pragma_update(None, "journal_mode", "WAL")
        .map_err(|_| AppError::storage_open_failed())?;
    if journal_mode(connection)
        .map_err(|_| AppError::storage_open_failed())?
        .eq_ignore_ascii_case("wal")
    {
        Ok(())
    } else {
        Err(AppError::storage_open_failed())
    }
}

pub(super) fn schema_version(connection: &Connection) -> Result<u32, AppError> {
    connection
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .map_err(|_| AppError::storage_read_failed())
}

pub(super) fn table_columns(
    connection: &Connection,
    table: TableName,
) -> Result<Vec<String>, AppError> {
    let statement = format!("PRAGMA table_info({})", table.sql_name());
    let mut query = connection
        .prepare(&statement)
        .map_err(|_| AppError::storage_read_failed())?;
    let columns = query
        .query_map([], |row| row.get(1))
        .map_err(|_| AppError::storage_read_failed())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| AppError::storage_read_failed())?;
    Ok(columns)
}

pub(super) fn foreign_keys_enabled(connection: &Connection) -> Result<bool, AppError> {
    connection
        .pragma_query_value(None, "foreign_keys", |row| row.get::<_, i64>(0))
        .map(|value| value == 1)
        .map_err(|_| AppError::storage_read_failed())
}

pub(super) fn journal_mode(connection: &Connection) -> Result<String, AppError> {
    connection
        .pragma_query_value(None, "journal_mode", |row| row.get(0))
        .map_err(|_| AppError::storage_read_failed())
}
