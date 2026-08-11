use chrono::{DateTime, Days, Local, NaiveDate, TimeZone};

use crate::error::AppError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CivilDate {
    canonical: String,
    date: NaiveDate,
}

impl CivilDate {
    pub fn parse(value: &str) -> Result<Self, AppError> {
        let date =
            NaiveDate::parse_from_str(value, "%Y-%m-%d").map_err(|_| AppError::invalid_date())?;
        Self::from_naive_date(date, value)
    }

    pub fn today() -> Result<Self, AppError> {
        Self::from_local_datetime(Local::now())
    }

    pub fn from_local_datetime<Timezone>(value: DateTime<Timezone>) -> Result<Self, AppError>
    where
        Timezone: TimeZone,
    {
        let date = value.date_naive();
        let canonical = date.format("%F").to_string();
        Self::from_naive_date(date, &canonical)
    }

    pub fn previous_day(&self) -> Result<Self, AppError> {
        self.shift_days(Days::new(1), Direction::Previous)
    }

    pub fn next_day(&self) -> Result<Self, AppError> {
        self.shift_days(Days::new(1), Direction::Next)
    }

    pub fn as_str(&self) -> &str {
        &self.canonical
    }

    fn from_naive_date(date: NaiveDate, value: &str) -> Result<Self, AppError> {
        let canonical = date.format("%F").to_string();
        if canonical != value || !is_canonical_length(&canonical) {
            return Err(AppError::invalid_date());
        }

        Ok(Self { canonical, date })
    }

    fn shift_days(&self, days: Days, direction: Direction) -> Result<Self, AppError> {
        let shifted = match direction {
            Direction::Previous => self.date.checked_sub_days(days),
            Direction::Next => self.date.checked_add_days(days),
        }
        .ok_or_else(AppError::invalid_date)?;
        let canonical = shifted.format("%F").to_string();
        Self::from_naive_date(shifted, &canonical)
    }
}

enum Direction {
    Previous,
    Next,
}

const fn is_canonical_length(value: &str) -> bool {
    value.len() == 10
}
