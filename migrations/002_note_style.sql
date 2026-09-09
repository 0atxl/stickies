CREATE TABLE note_styles (
    note_id INTEGER PRIMARY KEY REFERENCES notes(id),
    settings TEXT NOT NULL
);
