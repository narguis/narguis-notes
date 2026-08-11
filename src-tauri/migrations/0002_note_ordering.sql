CREATE INDEX undated_notes_updated_at_ms_index
    ON undated_notes (updated_at_ms DESC, id ASC);
