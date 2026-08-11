use super::database::DatabaseOptions;
use crate::error::AppError;
use rusqlite::{Connection, TransactionBehavior};

const MIGRATIONS: &[(u32, &str)] = &[
    (1, include_str!("../../migrations/0001_initial_schema.sql")),
    (2, include_str!("../../migrations/0002_note_ordering.sql")),
    (
        3,
        include_str!("../../migrations/0003_planner_lines_and_task_templates.sql"),
    ),
    (
        4,
        include_str!("../../migrations/0004_planner_line_title_description.sql"),
    ),
    (5, include_str!("../../migrations/0005_task_scheduling.sql")),
    (6, include_str!("../../migrations/0006_task_lineage.sql")),
    (7, include_str!("../../migrations/0007_task_alarms.sql")),
];
pub const DATABASE_VERSION: u32 = 7;

pub(super) fn apply(
    connection: &mut Connection,
    options: &DatabaseOptions,
) -> Result<(), AppError> {
    if options.target_version > DATABASE_VERSION {
        return Err(AppError::storage_migration_failed());
    }
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|_| AppError::storage_migration_failed())?;
    let current_version = transaction
        .pragma_query_value(None, "user_version", |row| row.get::<_, u32>(0))
        .map_err(|_| AppError::storage_migration_failed())?;
    if current_version > options.target_version {
        return Err(AppError::storage_migration_failed());
    }
    for &(version, sql) in MIGRATIONS {
        if version > current_version && version <= options.target_version {
            if options.injected_migration_failure == Some(version) {
                return Err(AppError::storage_migration_failed());
            }
            if options.injected_migration_failure_after_first_statement == Some(version) {
                let (first_statement, _) = sql
                    .split_once(';')
                    .ok_or_else(AppError::storage_migration_failed)?;
                transaction
                    .execute_batch(first_statement)
                    .map_err(|_| AppError::storage_migration_failed())?;
                return Err(AppError::storage_migration_failed());
            }
            transaction
                .execute_batch(sql)
                .map_err(|_| AppError::storage_migration_failed())?;
            transaction
                .pragma_update(None, "user_version", version)
                .map_err(|_| AppError::storage_migration_failed())?;
        }
    }
    transaction
        .commit()
        .map_err(|_| AppError::storage_migration_failed())
}
