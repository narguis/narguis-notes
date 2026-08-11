use crate::{civil_date::CivilDate, error::AppError};
use serde::{ser::Error as _, Deserialize, Deserializer, Serialize, Serializer};

mod outline;
pub use outline::*;

pub const MAX_NOTE_TITLE_LENGTH: usize = 200;
pub const MAX_NOTE_BODY_LENGTH: usize = 10_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CivilDateInput {
    Valid(CivilDate),
    Invalid,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(transparent)]
pub struct NoteId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(transparent)]
pub struct NoteTitle(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(transparent)]
pub struct NoteBody(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct GetDailyPageRequest {
    pub date: CivilDateInput,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct SaveDailyPageRequest {
    pub date: CivilDateInput,
    pub content: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct CreateNoteRequest {
    pub title: NoteTitle,
    pub body: NoteBody,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct UpdateNoteRequest {
    pub id: NoteId,
    pub title: NoteTitle,
    pub body: NoteBody,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct DeleteNoteRequest {
    pub id: NoteId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DailyPageDto {
    pub date: CivilDateInput,
    pub content: String,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NoteDto {
    pub id: NoteId,
    pub title: NoteTitle,
    pub body: NoteBody,
}

pub(crate) trait Validate {
    fn validate(&self) -> Result<(), AppError>;
}

impl Validate for GetDailyPageRequest {
    fn validate(&self) -> Result<(), AppError> {
        self.date.validate()
    }
}

impl Validate for SaveDailyPageRequest {
    fn validate(&self) -> Result<(), AppError> {
        self.date.validate()?;
        if self.content.chars().count() > MAX_NOTE_BODY_LENGTH {
            return Err(AppError::body_too_long());
        }
        Ok(())
    }
}

impl Validate for CreateNoteRequest {
    fn validate(&self) -> Result<(), AppError> {
        self.title.validate()?;
        self.body.validate()
    }
}

impl Validate for UpdateNoteRequest {
    fn validate(&self) -> Result<(), AppError> {
        self.title.validate()?;
        self.body.validate()
    }
}

impl Validate for DeleteNoteRequest {
    fn validate(&self) -> Result<(), AppError> {
        Ok(())
    }
}

impl CivilDateInput {
    pub fn parse(value: &str) -> Result<Self, AppError> {
        CivilDate::parse(value).map(Self::Valid)
    }

    pub fn as_str(&self) -> Result<&str, AppError> {
        match self {
            Self::Valid(date) => Ok(date.as_str()),
            Self::Invalid => Err(AppError::invalid_date()),
        }
    }

    fn validate(&self) -> Result<(), AppError> {
        self.as_str().map(|_| ())
    }

    pub fn add_days(&self, days: u16) -> Result<Self, AppError> {
        let mut date = self.clone();
        for _ in 0..days {
            date = match date {
                Self::Valid(value) => Self::Valid(value.next_day()?),
                Self::Invalid => return Err(AppError::invalid_date()),
            }
        }
        Ok(date)
    }
}

impl NoteTitle {
    fn validate(&self) -> Result<(), AppError> {
        if self.0.chars().count() > MAX_NOTE_TITLE_LENGTH {
            return Err(AppError::title_too_long());
        }

        Ok(())
    }
}

impl NoteBody {
    fn validate(&self) -> Result<(), AppError> {
        if self.0.chars().count() > MAX_NOTE_BODY_LENGTH {
            return Err(AppError::body_too_long());
        }

        Ok(())
    }
}

impl<'de> Deserialize<'de> for CivilDateInput {
    fn deserialize<DeserializerType>(
        deserializer: DeserializerType,
    ) -> Result<Self, DeserializerType::Error>
    where
        DeserializerType: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Ok(CivilDate::parse(&value).map_or(Self::Invalid, Self::Valid))
    }
}

impl Serialize for CivilDateInput {
    fn serialize<SerializerType>(
        &self,
        serializer: SerializerType,
    ) -> Result<SerializerType::Ok, SerializerType::Error>
    where
        SerializerType: Serializer,
    {
        match self {
            Self::Valid(date) => serializer.serialize_str(date.as_str()),
            Self::Invalid => Err(SerializerType::Error::custom("invalid civil date")),
        }
    }
}
