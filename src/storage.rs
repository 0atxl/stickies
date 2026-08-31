use std::{
    error::Error,
    fmt, fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use rusqlite::{Connection, params};

use crate::app::{Note, NoteColor, NoteId};

const SCHEMA_VERSION: i64 = 1;

#[derive(Debug)]
pub enum StorageError {
    Io(std::io::Error),
    Database(rusqlite::Error),
    InvalidColor(String),
    InvalidNoteId(i64),
    NoteIdOutOfRange(u64),
    MissingNote(NoteId),
    UnsupportedSchema(i64),
}

impl fmt::Display for StorageError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "could not prepare the data directory: {error}"),
            Self::Database(error) => write!(formatter, "database error: {error}"),
            Self::InvalidColor(color) => {
                write!(formatter, "database contains unknown color: {color}")
            }
            Self::InvalidNoteId(id) => write!(formatter, "database contains invalid note id: {id}"),
            Self::NoteIdOutOfRange(id) => write!(formatter, "note id {id} is too large for SQLite"),
            Self::MissingNote(id) => {
                write!(formatter, "database does not contain note {}", id.value())
            }
            Self::UnsupportedSchema(version) => {
                write!(
                    formatter,
                    "database schema {version} is newer than this app supports"
                )
            }
        }
    }
}

impl Error for StorageError {}

impl From<std::io::Error> for StorageError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<rusqlite::Error> for StorageError {
    fn from(error: rusqlite::Error) -> Self {
        Self::Database(error)
    }
}

pub struct Storage {
    connection: Connection,
}

impl Storage {
    pub fn open(path: &Path) -> Result<Self, StorageError> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let connection = Connection::open(path)?;
        let mut storage = Self { connection };
        storage.migrate()?;
        Ok(storage)
    }

    pub fn load_or_seed(&mut self, initial_notes: &[Note]) -> Result<Vec<Note>, StorageError> {
        let note_count = self
            .connection
            .query_row("SELECT COUNT(*) FROM notes", [], |row| row.get::<_, i64>(0))?;

        if note_count == 0 {
            self.insert_notes(initial_notes)?;
        }

        self.load_active_notes()
    }

    pub fn update_note(&self, note: &Note) -> Result<(), StorageError> {
        let updated = self.connection.execute(
            "UPDATE notes
             SET title = ?1, body = ?2, color = ?3, updated_at = ?4
             WHERE id = ?5 AND deleted_at IS NULL",
            params![
                note.title,
                note.body,
                color_name(note.color),
                unix_timestamp(),
                note_id_to_i64(note.id)?,
            ],
        )?;
        if updated == 0 {
            Err(StorageError::MissingNote(note.id))
        } else {
            Ok(())
        }
    }

    fn migrate(&mut self) -> Result<(), StorageError> {
        let version = self
            .connection
            .query_row("PRAGMA user_version", [], |row| row.get::<_, i64>(0))?;

        if version > SCHEMA_VERSION {
            return Err(StorageError::UnsupportedSchema(version));
        }

        if version == 0 {
            let transaction = self.connection.transaction()?;
            transaction.execute_batch(include_str!("../migrations/001_initial.sql"))?;
            transaction.pragma_update(None, "user_version", SCHEMA_VERSION)?;
            transaction.commit()?;
        }

        Ok(())
    }

    fn insert_notes(&mut self, notes: &[Note]) -> Result<(), StorageError> {
        let timestamp = unix_timestamp();
        let transaction = self.connection.transaction()?;

        for (sort_order, note) in notes.iter().enumerate() {
            transaction.execute(
                "INSERT INTO notes (
                    id, title, body, color, pinned, sort_order,
                    created_at, updated_at, archived_at, deleted_at
                 ) VALUES (?1, ?2, ?3, ?4, 0, ?5, ?6, ?6, NULL, NULL)",
                params![
                    note_id_to_i64(note.id)?,
                    note.title,
                    note.body,
                    color_name(note.color),
                    i64::try_from(sort_order).unwrap_or(i64::MAX),
                    timestamp,
                ],
            )?;
        }

        transaction.commit()?;
        Ok(())
    }

    fn load_active_notes(&self) -> Result<Vec<Note>, StorageError> {
        let mut statement = self.connection.prepare(
            "SELECT id, title, body, color
             FROM notes
             WHERE archived_at IS NULL AND deleted_at IS NULL
             ORDER BY pinned DESC, sort_order ASC, id ASC",
        )?;
        let rows = statement.query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
            ))
        })?;

        let mut notes = Vec::new();
        for row in rows {
            let (id, title, body, color) = row?;
            let id = u64::try_from(id).map_err(|_| StorageError::InvalidNoteId(id))?;
            notes.push(Note {
                id: NoteId::new(id),
                title,
                body,
                color: parse_color(&color)?,
            });
        }
        Ok(notes)
    }
}

pub fn database_path() -> PathBuf {
    gtk::glib::user_data_dir().join("stickies").join("notes.db")
}

fn note_id_to_i64(id: NoteId) -> Result<i64, StorageError> {
    i64::try_from(id.value()).map_err(|_| StorageError::NoteIdOutOfRange(id.value()))
}

const fn color_name(color: NoteColor) -> &'static str {
    match color {
        NoteColor::Yellow => "yellow",
        NoteColor::Blue => "blue",
        NoteColor::Purple => "purple",
        NoteColor::Green => "green",
    }
}

fn parse_color(color: &str) -> Result<NoteColor, StorageError> {
    match color {
        "yellow" => Ok(NoteColor::Yellow),
        "blue" => Ok(NoteColor::Blue),
        "purple" => Ok(NoteColor::Purple),
        "green" => Ok(NoteColor::Green),
        _ => Err(StorageError::InvalidColor(color.to_owned())),
    }
}

fn unix_timestamp() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs() as i64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notes_survive_reopening_the_database() {
        let path = std::env::temp_dir().join(format!(
            "stickies-storage-{}-{}.db",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time must follow the Unix epoch")
                .as_nanos()
        ));
        let initial = vec![Note::new(
            NoteId::new(1),
            "Work",
            "Initial body",
            NoteColor::Yellow,
        )];

        {
            let mut storage = Storage::open(&path).expect("database should open");
            let mut notes = storage
                .load_or_seed(&initial)
                .expect("initial notes should be stored");
            notes[0].body = "Changed body".to_owned();
            storage
                .update_note(&notes[0])
                .expect("edited note should be stored");
        }

        {
            let mut storage = Storage::open(&path).expect("database should reopen");
            let notes = storage
                .load_or_seed(&initial)
                .expect("stored notes should load");
            assert_eq!(notes[0].body, "Changed body");
        }

        fs::remove_file(path).expect("test database should be removable");
    }

    #[test]
    fn a_future_schema_is_not_modified() {
        let connection = Connection::open_in_memory().expect("database should open");
        connection
            .pragma_update(None, "user_version", SCHEMA_VERSION + 1)
            .expect("schema version should be writable");
        let mut storage = Storage { connection };

        let error = storage.migrate().expect_err("future schema should fail");
        assert!(matches!(error, StorageError::UnsupportedSchema(2)));
        assert_eq!(
            storage
                .connection
                .query_row("PRAGMA user_version", [], |row| row.get::<_, i64>(0))
                .expect("schema version should remain readable"),
            2
        );
    }
}
