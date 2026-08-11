use super::{CivilDateInput, Validate, MAX_NOTE_BODY_LENGTH, MAX_NOTE_TITLE_LENGTH};
use crate::error::AppError;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(transparent)]
pub struct PlannerLineId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(transparent)]
pub struct PlannerLineTitle(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(transparent)]
pub struct PlannerLineDescription(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(transparent)]
pub struct SiblingKey(pub String);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(transparent)]
pub struct TimeOfDayMinutes(pub u16);

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(transparent)]
pub struct TaskTemplateId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(transparent)]
pub struct TaskTemplateTitle(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(transparent)]
pub struct TaskTemplateBody(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct CreatePlannerLineRequest {
    pub date: CivilDateInput,
    pub parent_id: Option<PlannerLineId>,
    pub sibling_key: SiblingKey,
    #[serde(alias = "text")]
    pub title: PlannerLineTitle,
    pub description: Option<PlannerLineDescription>,
    pub time_of_day_minutes: Option<TimeOfDayMinutes>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct UpdatePlannerLineRequest {
    pub id: PlannerLineId,
    pub title: PlannerLineTitle,
    pub description: Option<PlannerLineDescription>,
    pub time_of_day_minutes: Option<TimeOfDayMinutes>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct DeletePlannerLineRequest {
    pub id: PlannerLineId,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct MovePlannerLineRequest {
    pub id: PlannerLineId,
    pub parent_id: Option<PlannerLineId>,
    pub sibling_key: SiblingKey,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct SetPlannerLineCollapsedRequest {
    pub id: PlannerLineId,
    pub is_collapsed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct SetPlannerLineTimeRequest {
    pub id: PlannerLineId,
    pub time_of_day_minutes: Option<TimeOfDayMinutes>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlannerLineDto {
    pub id: PlannerLineId,
    pub date: CivilDateInput,
    pub parent_id: Option<PlannerLineId>,
    pub sibling_key: SiblingKey,
    pub title: PlannerLineTitle,
    pub description: Option<PlannerLineDescription>,
    pub time_of_day_minutes: Option<TimeOfDayMinutes>,
    pub is_collapsed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct CreateTaskTemplateRequest {
    pub title: TaskTemplateTitle,
    pub body: TaskTemplateBody,
    pub time_of_day_minutes: Option<u16>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct UpdateTaskTemplateRequest {
    pub id: TaskTemplateId,
    pub title: TaskTemplateTitle,
    pub body: TaskTemplateBody,
    pub time_of_day_minutes: Option<u16>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct DeleteTaskTemplateRequest {
    pub id: TaskTemplateId,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct InsertTaskTemplateCopyRequest {
    pub template_id: TaskTemplateId,
    pub date: CivilDateInput,
    pub parent_id: Option<PlannerLineId>,
    pub sibling_key: SiblingKey,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskTemplateDto {
    pub id: TaskTemplateId,
    pub title: TaskTemplateTitle,
    pub body: TaskTemplateBody,
    pub time_of_day_minutes: Option<TimeOfDayMinutes>,
}

fn validate_id(value: &str, template: bool) -> Result<(), AppError> {
    if Uuid::parse_str(value).is_err() || (template && value.is_empty()) {
        Err(AppError::invalid_identifier())
    } else {
        Ok(())
    }
}

fn validate_sibling_key(value: &str) -> Result<(), AppError> {
    if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_alphanumeric()) {
        Err(AppError::invalid_sibling_key())
    } else {
        Ok(())
    }
}

fn validate_time(value: Option<TimeOfDayMinutes>) -> Result<(), AppError> {
    if value.is_some_and(|minutes| minutes.0 >= 1440) {
        Err(AppError::invalid_time_of_day())
    } else {
        Ok(())
    }
}

fn validate_text(title: &str, body: &str) -> Result<(), AppError> {
    if title.chars().count() > MAX_NOTE_TITLE_LENGTH {
        return Err(AppError::title_too_long());
    }
    if body.chars().count() > MAX_NOTE_BODY_LENGTH {
        return Err(AppError::body_too_long());
    }
    Ok(())
}

impl Validate for CreatePlannerLineRequest {
    fn validate(&self) -> Result<(), AppError> {
        self.date.validate()?;
        if let Some(parent) = &self.parent_id {
            validate_id(&parent.0, false)?;
        }
        validate_sibling_key(&self.sibling_key.0)?;
        validate_text(
            &self.title.0,
            self.description.as_ref().map_or("", |value| &value.0),
        )?;
        validate_time(self.time_of_day_minutes)
    }
}

impl Validate for UpdatePlannerLineRequest {
    fn validate(&self) -> Result<(), AppError> {
        validate_id(&self.id.0, false)?;
        validate_text(
            &self.title.0,
            self.description.as_ref().map_or("", |value| &value.0),
        )?;
        validate_time(self.time_of_day_minutes)
    }
}

impl Validate for DeletePlannerLineRequest {
    fn validate(&self) -> Result<(), AppError> {
        validate_id(&self.id.0, false)
    }
}

impl Validate for MovePlannerLineRequest {
    fn validate(&self) -> Result<(), AppError> {
        validate_id(&self.id.0, false)?;
        if let Some(parent) = &self.parent_id {
            validate_id(&parent.0, false)?;
        }
        validate_sibling_key(&self.sibling_key.0)
    }
}

impl Validate for SetPlannerLineCollapsedRequest {
    fn validate(&self) -> Result<(), AppError> {
        validate_id(&self.id.0, false)
    }
}

impl Validate for SetPlannerLineTimeRequest {
    fn validate(&self) -> Result<(), AppError> {
        validate_id(&self.id.0, false)?;
        validate_time(self.time_of_day_minutes)
    }
}

impl Validate for CreateTaskTemplateRequest {
    fn validate(&self) -> Result<(), AppError> {
        validate_text(&self.title.0, &self.body.0)?;
        validate_time(self.time_of_day_minutes.map(TimeOfDayMinutes))
    }
}

impl Validate for UpdateTaskTemplateRequest {
    fn validate(&self) -> Result<(), AppError> {
        validate_id(&self.id.0, true)?;
        validate_text(&self.title.0, &self.body.0)?;
        validate_time(self.time_of_day_minutes.map(TimeOfDayMinutes))
    }
}

impl Validate for DeleteTaskTemplateRequest {
    fn validate(&self) -> Result<(), AppError> {
        validate_id(&self.id.0, true)
    }
}

impl Validate for InsertTaskTemplateCopyRequest {
    fn validate(&self) -> Result<(), AppError> {
        validate_id(&self.template_id.0, true)?;
        self.date.validate()?;
        if let Some(parent) = &self.parent_id {
            validate_id(&parent.0, false)?;
        }
        validate_sibling_key(&self.sibling_key.0)
    }
}
