CREATE TABLE planner_lines (
    id TEXT PRIMARY KEY,
    date TEXT NOT NULL,
    parent_id TEXT REFERENCES planner_lines(id) ON DELETE CASCADE,
    sibling_key TEXT NOT NULL COLLATE BINARY,
    text TEXT NOT NULL,
    time_of_day_minutes INTEGER,
    is_collapsed INTEGER NOT NULL DEFAULT 0,
    created_at_ms INTEGER NOT NULL,
    updated_at_ms INTEGER NOT NULL
);

CREATE INDEX planner_lines_date_order_index
    ON planner_lines (date, parent_id, sibling_key COLLATE BINARY);

CREATE TABLE task_templates (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    body TEXT NOT NULL,
    time_of_day_minutes INTEGER,
    created_at_ms INTEGER NOT NULL,
    updated_at_ms INTEGER NOT NULL
);

CREATE INDEX task_templates_updated_at_ms_index
    ON task_templates (updated_at_ms DESC, id ASC);
