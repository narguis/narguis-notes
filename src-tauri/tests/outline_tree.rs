use notes_planner_desktop::{
    dto::{
        CivilDateInput, CreatePlannerLineRequest, MovePlannerLineRequest, PlannerLineId,
        PlannerLineTitle, SetPlannerLineCollapsedRequest, SetPlannerLineTimeRequest, SiblingKey,
        TimeOfDayMinutes,
    },
    error::AppErrorCode,
    storage::Database,
};
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
};

static NEXT_TEMP_DIRECTORY: AtomicUsize = AtomicUsize::new(0);

struct TemporaryDatabaseDirectory(PathBuf);

impl TemporaryDatabaseDirectory {
    fn new() -> Self {
        let sequence = NEXT_TEMP_DIRECTORY.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "notes-planner-outline-tree-{}-{sequence}",
            std::process::id(),
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("temporary directory should be created");
        Self(path)
    }

    fn database_path(&self) -> PathBuf {
        self.0.join("planner.sqlite3")
    }
}

impl Drop for TemporaryDatabaseDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn date(value: &str) -> CivilDateInput {
    CivilDateInput::parse(value).expect("test date should parse")
}

fn line_request(
    date: CivilDateInput,
    parent_id: Option<PlannerLineId>,
    sibling_key: &str,
    title: &str,
    time_of_day_minutes: Option<u16>,
) -> CreatePlannerLineRequest {
    CreatePlannerLineRequest {
        date,
        parent_id,
        sibling_key: SiblingKey(sibling_key.to_owned()),
        title: PlannerLineTitle(title.to_owned()),
        description: None,
        time_of_day_minutes: time_of_day_minutes.map(TimeOfDayMinutes),
    }
}

#[test]
fn lists_tree_depth_first_in_binary_fractional_sibling_order_after_restart() {
    // Given: roots and descendants using intentionally non-lexical insertion order
    let directory = TemporaryDatabaseDirectory::new();
    let mut database = Database::open(directory.database_path()).expect("database opens");
    let root = database
        .create_planner_line(&line_request(date("2026-07-30"), None, "a", "root", None))
        .expect("root creates");
    let later_root = database
        .create_planner_line(&line_request(
            date("2026-07-30"),
            None,
            "z",
            "later root",
            None,
        ))
        .expect("later root creates");
    let child = database
        .create_planner_line(&line_request(
            date("2026-07-30"),
            Some(root.id.clone()),
            "aV",
            "child",
            Some(571),
        ))
        .expect("child creates");
    let grandchild = database
        .create_planner_line(&line_request(
            date("2026-07-30"),
            Some(child.id.clone()),
            "aV0",
            "grandchild",
            None,
        ))
        .expect("grandchild creates");
    drop(database);
    let mut restarted_database =
        Database::open(directory.database_path()).expect("database restarts");

    // When: the date-scoped tree is listed through storage
    let lines = restarted_database
        .list_planner_lines(&date("2026-07-30"))
        .expect("lines list");

    // Then: depth-first traversal honors BINARY keys, parent links, and floating local time
    assert_eq!(
        lines.iter().map(|line| &line.id).collect::<Vec<_>>(),
        vec![&root.id, &child.id, &grandchild.id, &later_root.id]
    );
    assert_eq!(lines[1].time_of_day_minutes, Some(TimeOfDayMinutes(571)));
}

#[test]
fn rejects_cycles_cross_day_moves_and_invalid_time_without_partial_mutation() {
    // Given: a parent-child pair and a root on another day
    let directory = TemporaryDatabaseDirectory::new();
    let mut database = Database::open(directory.database_path()).expect("database opens");
    let root = database
        .create_planner_line(&line_request(date("2026-07-30"), None, "a", "root", None))
        .expect("root creates");
    let child = database
        .create_planner_line(&line_request(
            date("2026-07-30"),
            Some(root.id.clone()),
            "b",
            "child",
            None,
        ))
        .expect("child creates");
    let other_day = database
        .create_planner_line(&line_request(date("2026-07-31"), None, "a", "other", None))
        .expect("other day root creates");

    // When: invalid reparent and time requests reach the transactional boundary
    let cycle = database
        .move_planner_line(&MovePlannerLineRequest {
            id: root.id.clone(),
            parent_id: Some(child.id.clone()),
            sibling_key: SiblingKey("c".to_owned()),
        })
        .expect_err("cycle must fail");
    let cross_day = database
        .move_planner_line(&MovePlannerLineRequest {
            id: child.id.clone(),
            parent_id: Some(other_day.id),
            sibling_key: SiblingKey("c".to_owned()),
        })
        .expect_err("cross-day parent must fail");
    let invalid_time = database.create_planner_line(&line_request(
        date("2026-07-30"),
        None,
        "d",
        "invalid time",
        Some(1440),
    ));

    // Then: typed validation rejects each request and the original tree remains unchanged
    assert_eq!(cycle.code, AppErrorCode::InvalidPlannerLineParent);
    assert_eq!(cross_day.code, AppErrorCode::InvalidPlannerLineParent);
    assert_eq!(
        invalid_time.expect_err("time must fail").code,
        AppErrorCode::InvalidTimeOfDay
    );
    assert_eq!(
        database
            .list_planner_lines(&date("2026-07-30"))
            .expect("lines list")
            .iter()
            .map(|line| &line.id)
            .collect::<Vec<_>>(),
        vec![&root.id, &child.id]
    );
}

#[test]
fn moves_reorders_collapses_and_clears_time_transactionally() {
    // Given: two roots and a timed child on the same planner date
    let directory = TemporaryDatabaseDirectory::new();
    let mut database = Database::open(directory.database_path()).expect("database opens");
    let first = database
        .create_planner_line(&line_request(date("2026-07-30"), None, "a", "first", None))
        .expect("first creates");
    let second = database
        .create_planner_line(&line_request(date("2026-07-30"), None, "z", "second", None))
        .expect("second creates");
    let child = database
        .create_planner_line(&line_request(
            date("2026-07-30"),
            Some(first.id.clone()),
            "a",
            "child",
            Some(571),
        ))
        .expect("child creates");

    // When: the child is moved and reordered, and persisted presentation fields change
    database
        .move_planner_line(&MovePlannerLineRequest {
            id: child.id.clone(),
            parent_id: Some(second.id.clone()),
            sibling_key: SiblingKey("A".to_owned()),
        })
        .expect("move succeeds");
    database
        .set_planner_line_collapsed(&SetPlannerLineCollapsedRequest {
            id: second.id.clone(),
            is_collapsed: true,
        })
        .expect("collapse persists");
    database
        .set_planner_line_time(&SetPlannerLineTimeRequest {
            id: child.id.clone(),
            time_of_day_minutes: None,
        })
        .expect("time clears");

    // Then: deterministic BINARY ordering, adjacency, collapse, and null time survive reload
    drop(database);
    let mut restarted_database =
        Database::open(directory.database_path()).expect("database restarts");
    let lines = restarted_database
        .list_planner_lines(&date("2026-07-30"))
        .expect("lines list");
    assert_eq!(
        lines.iter().map(|line| &line.id).collect::<Vec<_>>(),
        vec![&first.id, &second.id, &child.id]
    );
    assert!(lines[1].is_collapsed);
    assert_eq!(lines[2].parent_id, Some(second.id));
    assert_eq!(lines[2].time_of_day_minutes, None);
}
