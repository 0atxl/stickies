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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NoteCollection {
    Active,
    Archived,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NoteSort {
    Recent,
    Title,
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

    pub fn load_or_seed(
        &mut self,
        initial_notes: &[Note],
        limit: usize,
    ) -> Result<Vec<Note>, StorageError> {
        let note_count = self
            .connection
            .query_row("SELECT COUNT(*) FROM notes", [], |row| row.get::<_, i64>(0))?;

        if note_count == 0 {
            self.insert_notes(initial_notes)?;
        }

        self.load_deck_notes(limit)
    }

    pub fn create_note(
        &self,
        title: &str,
        body: &str,
        color: NoteColor,
    ) -> Result<Note, StorageError> {
        let timestamp = unix_timestamp_millis();
        self.connection.execute(
            "INSERT INTO notes (
                title, body, color, pinned, sort_order,
                created_at, updated_at, archived_at, deleted_at
             ) VALUES (
                ?1, ?2, ?3, 0,
                (SELECT COALESCE(MIN(sort_order), 1) - 1 FROM notes),
                ?4, ?4, NULL, NULL
             )",
            params![title, body, color_name(color), timestamp],
        )?;
        let id = self.connection.last_insert_rowid();
        let id = u64::try_from(id).map_err(|_| StorageError::InvalidNoteId(id))?;
        Ok(Note::new(NoteId::new(id), title, body, color))
    }

    pub fn update_note(&self, note: &Note) -> Result<(), StorageError> {
        let updated = self.connection.execute(
            "UPDATE notes
             SET title = ?1, body = ?2, color = ?3, updated_at = ?4
             WHERE id = ?5 AND archived_at IS NULL AND deleted_at IS NULL",
            params![
                note.title,
                note.body,
                color_name(note.color),
                unix_timestamp_millis(),
                note_id_to_i64(note.id)?,
            ],
        )?;
        if updated == 0 {
            Err(StorageError::MissingNote(note.id))
        } else {
            Ok(())
        }
    }

    pub fn archive_note(&self, id: NoteId) -> Result<(), StorageError> {
        self.mark_note_removed(id, false)
    }

    pub fn delete_note(&self, id: NoteId) -> Result<(), StorageError> {
        self.mark_note_removed(id, true)
    }

    pub fn load_deck_notes(&self, limit: usize) -> Result<Vec<Note>, StorageError> {
        let limit = i64::try_from(limit).unwrap_or(i64::MAX);
        let mut statement = self.connection.prepare(
            "SELECT id, title, body, color, pinned
             FROM notes
             WHERE archived_at IS NULL AND deleted_at IS NULL
             ORDER BY pinned DESC, updated_at DESC, sort_order ASC, id DESC
             LIMIT ?1",
        )?;
        let rows = statement.query_map([limit], note_from_row)?;

        rows.map(|row| {
            let row = row?;
            stored_note_from_row(row)
        })
        .collect()
    }

    pub fn search_notes(
        &self,
        collection: NoteCollection,
        query: &str,
        sort: NoteSort,
    ) -> Result<Vec<Note>, StorageError> {
        let collection_clause = match collection {
            NoteCollection::Active => "archived_at IS NULL",
            NoteCollection::Archived => "archived_at IS NOT NULL",
        };
        let order_clause = match (collection, sort) {
            (NoteCollection::Active, NoteSort::Recent) => {
                "pinned DESC, updated_at DESC, sort_order ASC, id DESC"
            }
            (NoteCollection::Active, NoteSort::Title) => {
                "pinned DESC, title COLLATE NOCASE ASC, updated_at DESC, id DESC"
            }
            (NoteCollection::Archived, NoteSort::Recent) => {
                "updated_at DESC, sort_order ASC, id DESC"
            }
            (NoteCollection::Archived, NoteSort::Title) => {
                "title COLLATE NOCASE ASC, updated_at DESC, id DESC"
            }
        };
        let sql = format!(
            "SELECT id, title, body, color, pinned
             FROM notes
             WHERE {collection_clause}
               AND deleted_at IS NULL
               AND (instr(lower(title), lower(?1)) > 0
                    OR instr(lower(body), lower(?1)) > 0)
             ORDER BY {order_clause}"
        );
        let mut statement = self.connection.prepare(&sql)?;
        let rows = statement.query_map([query], note_from_row)?;

        rows.map(|row| {
            let row = row?;
            stored_note_from_row(row)
        })
        .collect()
    }

    pub fn restore_note(&self, id: NoteId) -> Result<(), StorageError> {
        let timestamp = unix_timestamp_millis();
        let updated = self.connection.execute(
            "UPDATE notes
             SET archived_at = NULL, updated_at = ?1
             WHERE id = ?2 AND archived_at IS NOT NULL AND deleted_at IS NULL",
            params![timestamp, note_id_to_i64(id)?],
        )?;
        if updated == 0 {
            Err(StorageError::MissingNote(id))
        } else {
            Ok(())
        }
    }

    pub fn set_note_pinned(&self, id: NoteId, pinned: bool) -> Result<(), StorageError> {
        let updated = self.connection.execute(
            "UPDATE notes
             SET pinned = ?1
             WHERE id = ?2 AND archived_at IS NULL AND deleted_at IS NULL",
            params![pinned, note_id_to_i64(id)?],
        )?;
        if updated == 0 {
            Err(StorageError::MissingNote(id))
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
        let timestamp = unix_timestamp_millis();
        let transaction = self.connection.transaction()?;

        for (sort_order, note) in notes.iter().enumerate() {
            transaction.execute(
                "INSERT INTO notes (
                    id, title, body, color, pinned, sort_order,
                    created_at, updated_at, archived_at, deleted_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7, NULL, NULL)",
                params![
                    note_id_to_i64(note.id)?,
                    note.title,
                    note.body,
                    color_name(note.color),
                    note.pinned,
                    i64::try_from(sort_order).unwrap_or(i64::MAX),
                    timestamp
                ],
            )?;
        }

        transaction.commit()?;
        Ok(())
    }

    fn mark_note_removed(&self, id: NoteId, delete: bool) -> Result<(), StorageError> {
        let statement = if delete {
            "UPDATE notes SET deleted_at = ?1, updated_at = ?1
             WHERE id = ?2 AND deleted_at IS NULL"
        } else {
            "UPDATE notes SET archived_at = ?1, updated_at = ?1
             WHERE id = ?2 AND archived_at IS NULL AND deleted_at IS NULL"
        };
        let updated = self.connection.execute(
            statement,
            params![unix_timestamp_millis(), note_id_to_i64(id)?],
        )?;
        if updated == 0 {
            Err(StorageError::MissingNote(id))
        } else {
            Ok(())
        }
    }
}

pub fn database_path() -> PathBuf {
    gtk::glib::user_data_dir().join("stickies").join("notes.db")
}

fn note_id_to_i64(id: NoteId) -> Result<i64, StorageError> {
    i64::try_from(id.value()).map_err(|_| StorageError::NoteIdOutOfRange(id.value()))
}

fn note_from_row(
    row: &rusqlite::Row<'_>,
) -> Result<(i64, String, String, String, bool), rusqlite::Error> {
    Ok((
        row.get(0)?,
        row.get(1)?,
        row.get(2)?,
        row.get(3)?,
        row.get(4)?,
    ))
}

fn stored_note_from_row(
    (id, title, body, color, pinned): (i64, String, String, String, bool),
) -> Result<Note, StorageError> {
    let id = u64::try_from(id).map_err(|_| StorageError::InvalidNoteId(id))?;
    Ok(Note {
        id: NoteId::new(id),
        title,
        body,
        color: parse_color(&color)?,
        pinned,
    })
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

fn unix_timestamp_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_millis() as i64)
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
                .load_or_seed(&initial, 5)
                .expect("initial notes should be stored");
            notes[0].body = "Changed body".to_owned();
            storage
                .update_note(&notes[0])
                .expect("edited note should be stored");
        }

        {
            let mut storage = Storage::open(&path).expect("database should reopen");
            let notes = storage
                .load_or_seed(&initial, 5)
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

    #[test]
    fn create_archive_delete_and_deck_limit_are_persisted() {
        let connection = Connection::open_in_memory().expect("database should open");
        let mut storage = Storage { connection };
        storage.migrate().expect("migration should run");
        storage
            .load_or_seed(
                &[Note::new(NoteId::new(1), "Existing", "", NoteColor::Blue)],
                5,
            )
            .expect("initial note should be stored");

        let first = storage
            .create_note("First", "Body", NoteColor::Yellow)
            .expect("note should be created");
        let second = storage
            .create_note("Second", "", NoteColor::Green)
            .expect("note should be created");
        assert_eq!(
            storage
                .load_deck_notes(1)
                .expect("limited deck should load")[0]
                .id,
            second.id
        );

        storage
            .archive_note(second.id)
            .expect("note should be archived");
        let mut archived_edit = second.clone();
        archived_edit.body = "This must not be saved".to_owned();
        assert!(matches!(
            storage.update_note(&archived_edit),
            Err(StorageError::MissingNote(id)) if id == second.id
        ));
        assert!(
            storage
                .load_deck_notes(5)
                .expect("deck should load")
                .iter()
                .all(|note| note.id != second.id)
        );

        storage
            .delete_note(first.id)
            .expect("note should be soft deleted");
        let deleted_at = storage
            .connection
            .query_row(
                "SELECT deleted_at FROM notes WHERE id = ?1",
                [note_id_to_i64(first.id).expect("note id should fit")],
                |row| row.get::<_, Option<i64>>(0),
            )
            .expect("deleted note should remain stored");
        assert!(deleted_at.is_some());
    }

    #[test]
    fn search_separates_active_archived_and_deleted_notes_and_restore_moves_one_back() {
        let connection = Connection::open_in_memory().expect("database should open");
        let mut storage = Storage { connection };
        storage.migrate().expect("migration should run");

        let active = storage
            .create_note("Groceries", "Buy ginger", NoteColor::Green)
            .expect("active note should be created");
        let archived = storage
            .create_note("Work", "Review Ginger API", NoteColor::Blue)
            .expect("archived note should be created");
        let deleted = storage
            .create_note("Old ginger", "Remove", NoteColor::Yellow)
            .expect("deleted note should be created");
        storage
            .archive_note(archived.id)
            .expect("note should be archived");
        storage
            .archive_note(deleted.id)
            .expect("deleted note should first be archived");
        storage
            .delete_note(deleted.id)
            .expect("note should be deleted");

        assert_eq!(
            storage
                .search_notes(NoteCollection::Active, "GINGER", NoteSort::Recent)
                .expect("active search should run"),
            vec![active.clone()]
        );
        assert_eq!(
            storage
                .search_notes(NoteCollection::Archived, "ginger", NoteSort::Recent)
                .expect("archive search should run"),
            vec![archived.clone()]
        );

        storage
            .restore_note(archived.id)
            .expect("archived note should restore");
        let active_results = storage
            .search_notes(NoteCollection::Active, "ginger", NoteSort::Recent)
            .expect("active search should run after restore");
        assert_eq!(active_results.len(), 2);
        assert!(active_results.contains(&archived));
        assert!(
            storage
                .search_notes(NoteCollection::Archived, "", NoteSort::Recent)
                .expect("archive should be empty")
                .is_empty()
        );

        let title_sorted = storage
            .search_notes(NoteCollection::Active, "", NoteSort::Title)
            .expect("title sort should run");
        assert_eq!(title_sorted[0].id, active.id);
        assert_eq!(title_sorted[1].id, archived.id);

        storage
            .connection
            .execute(
                "UPDATE notes SET updated_at = ?1 WHERE id = ?2",
                params![100, note_id_to_i64(active.id).expect("note ID should fit")],
            )
            .expect("active timestamp should be set");
        storage
            .connection
            .execute(
                "UPDATE notes SET updated_at = ?1 WHERE id = ?2",
                params![
                    200,
                    note_id_to_i64(archived.id).expect("note ID should fit")
                ],
            )
            .expect("restored timestamp should be set");
        storage
            .set_note_pinned(active.id, true)
            .expect("older note should pin");
        assert_eq!(
            storage
                .search_notes(NoteCollection::Active, "", NoteSort::Recent)
                .expect("recent sort should put pinned note first")[0]
                .id,
            active.id
        );
        storage
            .set_note_pinned(active.id, false)
            .expect("older note should unpin");
        assert_eq!(
            storage
                .search_notes(NoteCollection::Active, "", NoteSort::Recent)
                .expect("recent sort should restore edit order")[0]
                .id,
            archived.id
        );

        storage
            .set_note_pinned(archived.id, true)
            .expect("active note should pin");
        assert_eq!(
            storage
                .search_notes(NoteCollection::Active, "", NoteSort::Title)
                .expect("title sort should run")[0]
                .id,
            archived.id
        );
        assert!(
            storage
                .load_deck_notes(5)
                .expect("deck should load pinned note")[0]
                .pinned
        );
    }
}
