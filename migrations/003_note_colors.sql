CREATE TABLE notes_new (
    id INTEGER PRIMARY KEY,
    title TEXT NOT NULL,
    body TEXT NOT NULL,
    color TEXT NOT NULL CHECK (color IN (
        'yellow', 'blue', 'purple', 'green', 'orange', 'red', 'pink', 'lime',
        'teal', 'cyan', 'indigo', 'magenta', 'brown', 'gray', 'black', 'white'
    )),
    pinned INTEGER NOT NULL DEFAULT 0 CHECK (pinned IN (0, 1)),
    sort_order INTEGER NOT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    archived_at INTEGER,
    deleted_at INTEGER
);
INSERT INTO notes_new SELECT * FROM notes;
CREATE TABLE note_styles_new (
    note_id INTEGER PRIMARY KEY REFERENCES notes_new(id),
    settings TEXT NOT NULL
);
INSERT INTO note_styles_new SELECT * FROM note_styles;
DROP TABLE note_styles;
DROP TABLE notes;
ALTER TABLE notes_new RENAME TO notes;
ALTER TABLE note_styles_new RENAME TO note_styles;
