use serde::Serialize;
use std::fmt::{Display, Formatter};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AppErrorCode {
    InvalidPayload,
    InvalidDate,
    TitleTooLong,
    BodyTooLong,
    FeatureUnavailable,
    StorageOpenFailed,
    StorageMigrationFailed,
    StorageReadFailed,
    StorageWriteFailed,
    StorageNotFound,
    InvalidIdentifier,
    InvalidSiblingKey,
    InvalidPlannerLineParent,
    InvalidTimeOfDay,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppError {
    pub code: AppErrorCode,
    pub message: &'static str,
}

impl AppError {
    pub const fn invalid_payload() -> Self {
        Self {
            code: AppErrorCode::InvalidPayload,
            message: "The command payload is invalid.",
        }
    }

    pub const fn invalid_date() -> Self {
        Self {
            code: AppErrorCode::InvalidDate,
            message: "The date must be a valid YYYY-MM-DD calendar date.",
        }
    }

    pub const fn title_too_long() -> Self {
        Self {
            code: AppErrorCode::TitleTooLong,
            message: "The note title exceeds the supported length.",
        }
    }

    pub const fn body_too_long() -> Self {
        Self {
            code: AppErrorCode::BodyTooLong,
            message: "The note body exceeds the supported length.",
        }
    }

    pub const fn feature_unavailable() -> Self {
        Self {
            code: AppErrorCode::FeatureUnavailable,
            message: "This command is not available until its storage workflow is installed.",
        }
    }

    pub const fn storage_open_failed() -> Self {
        Self {
            code: AppErrorCode::StorageOpenFailed,
            message: "Local storage could not be opened.",
        }
    }

    pub const fn storage_migration_failed() -> Self {
        Self {
            code: AppErrorCode::StorageMigrationFailed,
            message: "Local storage could not be migrated without preserving its data.",
        }
    }

    pub const fn storage_read_failed() -> Self {
        Self {
            code: AppErrorCode::StorageReadFailed,
            message: "Local storage could not be read.",
        }
    }

    pub const fn storage_write_failed() -> Self {
        Self {
            code: AppErrorCode::StorageWriteFailed,
            message: "Local storage could not be updated.",
        }
    }

    pub const fn storage_not_found() -> Self {
        Self {
            code: AppErrorCode::StorageNotFound,
            message: "The requested local storage record no longer exists.",
        }
    }

    pub const fn invalid_identifier() -> Self {
        Self {
            code: AppErrorCode::InvalidIdentifier,
            message: "The record identifier must be a UUID.",
        }
    }

    pub const fn invalid_sibling_key() -> Self {
        Self {
            code: AppErrorCode::InvalidSiblingKey,
            message: "The sibling ordering key is invalid.",
        }
    }

    pub const fn invalid_planner_line_parent() -> Self {
        Self {
            code: AppErrorCode::InvalidPlannerLineParent,
            message:
                "The planner line parent must be on the same day and outside the line subtree.",
        }
    }

    pub const fn invalid_time_of_day() -> Self {
        Self {
            code: AppErrorCode::InvalidTimeOfDay,
            message: "The local time must be between 0 and 1439 minutes.",
        }
    }
}

impl Display for AppError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}: {}", self.code, self.message)
    }
}

impl Display for AppErrorCode {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        let code = match self {
            Self::InvalidPayload => "invalid_payload",
            Self::InvalidDate => "invalid_date",
            Self::TitleTooLong => "title_too_long",
            Self::BodyTooLong => "body_too_long",
            Self::FeatureUnavailable => "feature_unavailable",
            Self::StorageOpenFailed => "storage_open_failed",
            Self::StorageMigrationFailed => "storage_migration_failed",
            Self::StorageReadFailed => "storage_read_failed",
            Self::StorageWriteFailed => "storage_write_failed",
            Self::StorageNotFound => "storage_not_found",
            Self::InvalidIdentifier => "invalid_identifier",
            Self::InvalidSiblingKey => "invalid_sibling_key",
            Self::InvalidPlannerLineParent => "invalid_planner_line_parent",
            Self::InvalidTimeOfDay => "invalid_time_of_day",
        };

        formatter.write_str(code)
    }
}

impl std::error::Error for AppError {}
