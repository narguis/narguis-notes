use crate::{
    dto::{
        CivilDateInput, CreateNoteRequest, CreatePlannerLineRequest, CreateTaskTemplateRequest,
        InsertTaskTemplateCopyRequest, NoteBody, NoteTitle, PlannerLineDescription,
        PlannerLineTitle, SetPlannerLineCollapsedRequest, SiblingKey, TaskTemplateBody,
        TaskTemplateTitle, TimeOfDayMinutes, UpdateTaskTemplateRequest,
    },
    error::AppError,
    storage::{Database, DATABASE_FILE_NAME},
};
use std::path::Path;

const DATE: &str = "2026-07-30";
const LEGACY_PROSE: &str = "  Keep this prose exactly.\n- not a line\n\n09:31 maybe? 🗒️\n";
const NOTE_TITLE: &str = "Loose notes\t";
const NOTE_BODY: &str = "verbatim\nbody\nwith spaces  \nand emoji 🧪";
const TEMPLATE_TITLE: &str = "Review launch checklist";
const TEMPLATE_BODY: &str = "Confirm desktop launch";

pub fn run(app_data_directory: &Path) -> Result<(), AppError> {
    std::fs::create_dir_all(app_data_directory).map_err(|_| AppError::storage_open_failed())?;
    let database_path = app_data_directory.join(DATABASE_FILE_NAME);
    let date = CivilDateInput::parse(DATE)?;
    let mut database = Database::open(database_path.clone())?;
    database.save_daily_page(&crate::dto::SaveDailyPageRequest {
        date: date.clone(),
        content: LEGACY_PROSE.to_owned(),
    })?;
    let note = database.create_note(&CreateNoteRequest {
        title: NoteTitle(NOTE_TITLE.to_owned()),
        body: NoteBody(NOTE_BODY.to_owned()),
    })?;
    let root = database.create_planner_line(&CreatePlannerLineRequest {
        date: date.clone(),
        parent_id: None,
        sibling_key: SiblingKey("a".to_owned()),
        title: PlannerLineTitle("root".to_owned()),
        description: None,
        time_of_day_minutes: None,
        deadline_days: None,
        deadline_date: None,
        source_task_id: None,
        repeat_days: String::new(),
    })?;
    let child = database.create_planner_line(&CreatePlannerLineRequest {
        date: date.clone(),
        parent_id: Some(root.id.clone()),
        sibling_key: SiblingKey("aV".to_owned()),
        title: PlannerLineTitle("child".to_owned()),
        description: Some(PlannerLineDescription("child description".to_owned())),
        time_of_day_minutes: Some(TimeOfDayMinutes(571)),
        deadline_days: None,
        deadline_date: None,
        source_task_id: None,
        repeat_days: String::new(),
    })?;
    let later_root = database.create_planner_line(&CreatePlannerLineRequest {
        date: date.clone(),
        parent_id: None,
        sibling_key: SiblingKey("z".to_owned()),
        title: PlannerLineTitle("later root".to_owned()),
        description: None,
        time_of_day_minutes: None,
        deadline_days: None,
        deadline_date: None,
        source_task_id: None,
        repeat_days: String::new(),
    })?;
    database.set_planner_line_collapsed(&SetPlannerLineCollapsedRequest {
        id: root.id.clone(),
        is_collapsed: true,
    })?;
    let template = database.create_task_template(&CreateTaskTemplateRequest {
        title: TaskTemplateTitle(TEMPLATE_TITLE.to_owned()),
        body: TaskTemplateBody(TEMPLATE_BODY.to_owned()),
        time_of_day_minutes: Some(720),
        deadline_days: None,
        repeat_days: String::new(),
    })?;
    let copied_line = database.insert_task_template_copy(&InsertTaskTemplateCopyRequest {
        template_id: template.id.clone(),
        date: date.clone(),
        parent_id: Some(root.id.clone()),
        sibling_key: SiblingKey("z".to_owned()),
    })?;
    drop(database);

    let mut restarted_database = Database::open(database_path.clone())?;
    let persisted_root = crate::dto::PlannerLineDto {
        is_collapsed: true,
        ..root.clone()
    };
    if copied_line.title.0.as_bytes() != TEMPLATE_TITLE.as_bytes()
        || copied_line
            .description
            .as_ref()
            .is_none_or(|description| description.0.as_bytes() != TEMPLATE_BODY.as_bytes())
        || restarted_database.get_daily_page(&date)?.content.as_bytes() != LEGACY_PROSE.as_bytes()
        || restarted_database
            .list_notes()?
            .into_iter()
            .find(|candidate| candidate.id == note.id)
            .is_none_or(|candidate| {
                candidate.title.0.as_bytes() != NOTE_TITLE.as_bytes()
                    || candidate.body.0.as_bytes() != NOTE_BODY.as_bytes()
            })
        || restarted_database.list_planner_lines(&date)?
            != vec![
                persisted_root.clone(),
                child.clone(),
                copied_line.clone(),
                later_root.clone(),
            ]
        || restarted_database.list_task_templates()? != vec![template.clone()]
    {
        return Err(AppError::storage_read_failed());
    }

    restarted_database.update_task_template(&UpdateTaskTemplateRequest {
        id: template.id.clone(),
        title: TaskTemplateTitle("Changed template".to_owned()),
        body: TaskTemplateBody("Changed body".to_owned()),
        time_of_day_minutes: None,
        deadline_days: None,
        repeat_days: String::new(),
    })?;
    restarted_database.delete_task_template(&template.id)?;
    drop(restarted_database);

    let mut final_database = Database::open(database_path)?;
    if final_database.list_task_templates()?.is_empty()
        && final_database.list_planner_lines(&date)?
            == vec![persisted_root, child, copied_line, later_root]
    {
        Ok(())
    } else {
        Err(AppError::storage_read_failed())
    }
}
