ALTER TABLE planner_lines ADD COLUMN deadline_days INTEGER;
ALTER TABLE planner_lines ADD COLUMN deadline_date TEXT;
ALTER TABLE planner_lines ADD COLUMN repeat_days TEXT NOT NULL DEFAULT '';

ALTER TABLE task_templates ADD COLUMN deadline_days INTEGER;
ALTER TABLE task_templates ADD COLUMN repeat_days TEXT NOT NULL DEFAULT '';
