use std::{
    error::Error,
    fmt, fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use rusqlite::{Connection, OptionalExtension, functions::FunctionFlags, params};

use crate::app::{Note, NoteColor, NoteId};

const SCHEMA_VERSION: i64 = 4;
// An unpublished note has no creation date yet. Normal notes, including old
// blanks, retain their positive timestamp. Promotion writes a real date atomically.
const DRAFT_CREATED_AT: i64 = -1;

#[derive(Debug)]
pub enum StorageError {
    Io(std::io::Error),
    Database(rusqlite::Error),
    InvalidColor(String),
    InvalidNoteId(i64),
    NoteIdOutOfRange(u64),
    MissingNote(NoteId),
    UnsupportedSchema(i64),
    InvalidStyle(String),
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
            Self::InvalidStyle(message) => write!(formatter, "invalid note appearance: {message}"),
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

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum NoteCollection {
    All,
    #[default]
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
        connection.create_scalar_function(
            "stickies_casefold",
            1,
            FunctionFlags::SQLITE_UTF8 | FunctionFlags::SQLITE_DETERMINISTIC,
            |context| {
                let text = context.get::<String>(0)?;
                Ok(gtk::glib::casefold(text).to_string())
            },
        )?;
        let mut storage = Self { connection };
        storage.migrate()?;
        let transaction = storage.connection.unchecked_transaction()?;
        transaction.execute(
            "DELETE FROM note_styles WHERE note_id IN (SELECT id FROM notes WHERE created_at = ?1)",
            [DRAFT_CREATED_AT],
        )?;
        transaction.execute(
            "DELETE FROM notes WHERE created_at = ?1",
            [DRAFT_CREATED_AT],
        )?;
        transaction.commit()?;
        Ok(storage)
    }

    pub fn create_note(
        &self,
        title: &str,
        body: &str,
        color: NoteColor,
    ) -> Result<Note, StorageError> {
        self.insert_note(title, body, color, false)
    }

    pub fn create_draft(&self, color: NoteColor) -> Result<Note, StorageError> {
        self.insert_note("", "", color, true)
    }

    pub fn create_new_note(&self, title: &str) -> Result<Note, StorageError> {
        let previous: Option<String> = self
            .connection
            .query_row(
                "SELECT color FROM notes ORDER BY id DESC LIMIT 1",
                [],
                |row| row.get(0),
            )
            .optional()?;
        let previous = previous.as_deref().map(parse_color).transpose()?;
        let choices = NoteColor::ALL.len() - usize::from(previous.is_some());
        let index = gtk::glib::random_int_range(0, choices as i32) as usize;
        let color = NoteColor::ALL
            .into_iter()
            .filter(|color| Some(*color) != previous)
            .nth(index)
            .expect("the note palette contains another color");
        if title.trim().is_empty() {
            self.create_draft(color)
        } else {
            self.create_note(title, "", color)
        }
    }

    pub fn has_notes(&self) -> Result<bool, StorageError> {
        Ok(self.connection.query_row(
            "SELECT EXISTS(SELECT 1 FROM notes WHERE deleted_at IS NULL AND created_at != ?1)",
            [DRAFT_CREATED_AT],
            |row| row.get(0),
        )?)
    }

    pub fn is_draft(&self, id: NoteId) -> Result<bool, StorageError> {
        Ok(self.connection.query_row(
            "SELECT created_at = ?2 FROM notes WHERE id = ?1",
            params![note_id_to_i64(id)?, DRAFT_CREATED_AT],
            |row| row.get(0),
        )?)
    }

    fn insert_note(
        &self,
        title: &str,
        body: &str,
        color: NoteColor,
        draft: bool,
    ) -> Result<Note, StorageError> {
        let timestamp = unix_timestamp_millis();
        self.connection.execute(
            "INSERT INTO notes (
                title, body, color, pinned, sort_order,
                created_at, updated_at, archived_at, deleted_at
             ) VALUES (
                ?1, ?2, ?3, 0,
                CASE WHEN ?5 = ?6 THEN 0 ELSE (SELECT COALESCE(MIN(sort_order), 1) - 1 FROM notes WHERE created_at != ?6) END,
                ?5, ?4, NULL, NULL
             )",
            params![title, body, color_name(color), timestamp, if draft { DRAFT_CREATED_AT } else { timestamp }, DRAFT_CREATED_AT],
        )?;
        let id = self.connection.last_insert_rowid();
        let id = u64::try_from(id).map_err(|_| StorageError::InvalidNoteId(id))?;
        Ok(Note::new(NoteId::new(id), title, body, color))
    }

    #[cfg(test)]
    pub fn update_note(&self, note: &Note) -> Result<(), StorageError> {
        self.update_formatted_note(note, &self.load_formatting(note.id)?)
    }

    pub fn load_formatting(
        &self,
        id: NoteId,
    ) -> Result<Vec<crate::text_editing::Span>, StorageError> {
        let text: String = self.connection.query_row(
            "SELECT formatting FROM notes WHERE id = ?1",
            [note_id_to_i64(id)?],
            |row| row.get(0),
        )?;
        serde_json::from_str(&text).map_err(|e| StorageError::InvalidStyle(e.to_string()))
    }

    pub fn update_formatted_note(
        &self,
        note: &Note,
        spans: &[crate::text_editing::Span],
    ) -> Result<(), StorageError> {
        let formatting =
            serde_json::to_string(spans).map_err(|e| StorageError::InvalidStyle(e.to_string()))?;
        let written = !note.title.trim().is_empty() || !note.body.trim().is_empty();
        let updated = self.connection.execute("UPDATE notes SET title = ?1, body = ?2, color = ?3, formatting = ?4, updated_at = ?5,
            sort_order = CASE WHEN created_at = ?7 AND ?8 THEN (SELECT COALESCE(MIN(sort_order), 1) - 1 FROM notes WHERE created_at != ?7) ELSE sort_order END,
            created_at = CASE WHEN created_at = ?7 AND ?8 THEN ?5 ELSE created_at END
            WHERE id = ?6 AND archived_at IS NULL AND deleted_at IS NULL",
            params![note.title, note.body, color_name(note.color), formatting, unix_timestamp_millis(), note_id_to_i64(note.id)?, DRAFT_CREATED_AT, written])?;
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

    pub fn undo_delete(&self, id: NoteId) -> Result<(), StorageError> {
        let updated = self.connection.execute(
            "UPDATE notes SET deleted_at = NULL WHERE id = ?1 AND deleted_at IS NOT NULL",
            params![note_id_to_i64(id)?],
        )?;
        if updated == 0 {
            Err(StorageError::MissingNote(id))
        } else {
            Ok(())
        }
    }

    pub fn load_deck_notes(&self, limit: usize, sort: NoteSort) -> Result<Vec<Note>, StorageError> {
        let limit = i64::try_from(limit).unwrap_or(i64::MAX);
        let order = note_order(NoteCollection::Active, sort);
        let mut statement = self.connection.prepare(&format!(
            "SELECT id, title, body, color, pinned
             FROM notes
             WHERE archived_at IS NULL AND deleted_at IS NULL AND created_at != {DRAFT_CREATED_AT}
             ORDER BY {order}
             LIMIT ?1"
        ))?;
        let rows = statement.query_map([limit], note_from_row)?;

        rows.map(|row| {
            let row = row?;
            stored_note_from_row(row)
        })
        .collect()
    }

    pub fn load_note(&self, id: NoteId) -> Result<Note, StorageError> {
        let row = self.connection.query_row(
            "SELECT id, title, body, color, pinned FROM notes WHERE id = ?1 AND archived_at IS NULL AND deleted_at IS NULL",
            [note_id_to_i64(id)?], note_from_row,
        )?;
        stored_note_from_row(row)
    }

    pub fn load_note_style(&self, id: NoteId) -> Result<crate::config::NoteStyle, StorageError> {
        let text: Option<String> = self
            .connection
            .query_row(
                "SELECT settings FROM note_styles WHERE note_id = ?1",
                [note_id_to_i64(id)?],
                |row| row.get(0),
            )
            .optional()?;
        text.map(|text| {
            toml::from_str(&text)
                .map_err(|error: toml::de::Error| StorageError::InvalidStyle(error.to_string()))
        })
        .unwrap_or_else(|| Ok(crate::config::NoteStyle::default()))
    }

    /// Preserve existing appearance before retiring app-wide font/paper keys.
    pub fn migrate_note_defaults(
        &self,
        font: Option<crate::config::Font>,
        paper: Option<crate::config::Paper>,
    ) -> Result<(), StorageError> {
        let transaction = self.connection.unchecked_transaction()?;
        let rows = {
            let mut statement = transaction.prepare("SELECT notes.id, note_styles.settings FROM notes LEFT JOIN note_styles ON notes.id = note_styles.note_id")?;
            statement
                .query_map([], |row| {
                    Ok((row.get::<_, i64>(0)?, row.get::<_, Option<String>>(1)?))
                })?
                .collect::<Result<Vec<_>, _>>()?
        };
        for (id, text) in rows {
            let mut style: crate::config::NoteStyle = text
                .map(|text| toml::from_str(&text))
                .transpose()
                .map_err(|error| StorageError::InvalidStyle(error.to_string()))?
                .unwrap_or_default();
            if let Some(font) = font {
                // The old per-note font key was ignored; preserve the formerly active global font.
                style.font = Some(font);
            }
            if style.paper.is_none() {
                style.paper = paper;
            }
            let text = toml::to_string(&style)
                .map_err(|error| StorageError::InvalidStyle(error.to_string()))?;
            transaction.execute("INSERT INTO note_styles (note_id, settings) VALUES (?1, ?2) ON CONFLICT(note_id) DO UPDATE SET settings = excluded.settings", params![id, text])?;
        }
        transaction.commit()?;
        Ok(())
    }

    pub fn save_note_style(
        &self,
        id: NoteId,
        style: &crate::config::NoteStyle,
    ) -> Result<(), StorageError> {
        let text = toml::to_string(style)
            .map_err(|error| StorageError::InvalidStyle(error.to_string()))?;
        self.connection.execute("INSERT INTO note_styles (note_id, settings) VALUES (?1, ?2) ON CONFLICT(note_id) DO UPDATE SET settings = excluded.settings",params![note_id_to_i64(id)?,text])?;
        Ok(())
    }

    pub fn note_dates(&self, id: NoteId) -> Result<(i64, i64), StorageError> {
        Ok(self.connection.query_row(
            "SELECT created_at, updated_at FROM notes WHERE id = ?1",
            [note_id_to_i64(id)?],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?)
    }

    pub fn is_archived(&self, id: NoteId) -> Result<bool, StorageError> {
        Ok(self.connection.query_row(
            "SELECT archived_at IS NOT NULL FROM notes WHERE id = ?1 AND deleted_at IS NULL",
            [note_id_to_i64(id)?],
            |row| row.get(0),
        )?)
    }

    pub fn search_notes(
        &self,
        collection: NoteCollection,
        query: &str,
        sort: NoteSort,
    ) -> Result<Vec<Note>, StorageError> {
        let collection_clause = match collection {
            NoteCollection::All => "1 = 1",
            NoteCollection::Active => "archived_at IS NULL",
            NoteCollection::Archived => "archived_at IS NOT NULL",
        };
        let order_clause = note_order(collection, sort);
        let sql = format!(
            "SELECT id, title, body, color, pinned
             FROM notes
             WHERE {collection_clause}
               AND created_at != {DRAFT_CREATED_AT}
               AND deleted_at IS NULL
               AND (instr(stickies_casefold(title), ?1) > 0
                    OR instr(stickies_casefold(body), ?1) > 0)
             ORDER BY {order_clause}"
        );
        let mut statement = self.connection.prepare(&sql)?;
        let query = gtk::glib::casefold(query).to_string();
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
            transaction.pragma_update(None, "user_version", 1)?;
            transaction.commit()?;
        }

        if version < 2 {
            let transaction = self.connection.transaction()?;
            transaction.execute_batch(include_str!("../migrations/002_note_style.sql"))?;
            transaction.pragma_update(None, "user_version", 2)?;
            transaction.commit()?;
        }
        if version < 3 {
            let transaction = self.connection.transaction()?;
            transaction.execute_batch(include_str!("../migrations/003_note_colors.sql"))?;
            transaction.pragma_update(None, "user_version", 3)?;
            transaction.commit()?;
        }
        if version < 4 {
            let transaction = self.connection.transaction()?;
            transaction.execute_batch(include_str!("../migrations/004_note_formatting.sql"))?;
            transaction.pragma_update(None, "user_version", SCHEMA_VERSION)?;
            transaction.commit()?;
        }
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
    color.name()
}

fn parse_color(color: &str) -> Result<NoteColor, StorageError> {
    NoteColor::ALL
        .into_iter()
        .find(|value| value.name() == color)
        .ok_or_else(|| StorageError::InvalidColor(color.to_owned()))
}

fn unix_timestamp_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_millis() as i64)
}

fn note_order(collection: NoteCollection, sort: NoteSort) -> &'static str {
    match (collection, sort) {
        (NoteCollection::Active | NoteCollection::All, NoteSort::Recent) => {
            "pinned DESC, updated_at DESC, sort_order ASC, id DESC"
        }
        (NoteCollection::Active | NoteCollection::All, NoteSort::Title) => {
            "pinned DESC, title COLLATE NOCASE ASC, updated_at DESC, id DESC"
        }
        (NoteCollection::Archived, NoteSort::Recent) => "updated_at DESC, sort_order ASC, id DESC",
        (NoteCollection::Archived, NoteSort::Title) => {
            "title COLLATE NOCASE ASC, updated_at DESC, id DESC"
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn draft_queries_preserve_visible_notes_until_first_nonblank_save() {
        let storage = Storage::open(Path::new(":memory:")).unwrap();
        let existing_blank = storage.create_note("", "", NoteColor::Blue).unwrap();
        for title in ["One", "Two", "Three", "Four", "Five"] {
            storage
                .create_note(title, "Body", NoteColor::Yellow)
                .unwrap();
        }
        let before = storage.load_deck_notes(4, NoteSort::Recent).unwrap();
        let visible = storage
            .search_notes(NoteCollection::All, "", NoteSort::Recent)
            .unwrap();
        let mut draft = storage.create_draft(NoteColor::Green).unwrap();
        assert!(storage.is_draft(draft.id).unwrap());
        draft.title = "\u{2003} ".into();
        draft.body = "\n\t".into();
        storage.update_note(&draft).unwrap();
        storage
            .save_note_style(draft.id, &crate::config::NoteStyle::default())
            .unwrap();
        storage.set_note_pinned(draft.id, true).unwrap();
        assert_eq!(
            storage.load_deck_notes(4, NoteSort::Recent).unwrap(),
            before
        );
        assert_eq!(
            storage
                .search_notes(NoteCollection::All, "", NoteSort::Recent)
                .unwrap(),
            visible
        );
        for collection in [
            NoteCollection::All,
            NoteCollection::Active,
            NoteCollection::Archived,
        ] {
            for sort in [NoteSort::Recent, NoteSort::Title] {
                assert!(
                    storage
                        .search_notes(collection, "", sort)
                        .unwrap()
                        .iter()
                        .all(|n| n.id != draft.id)
                );
            }
        }
        assert!(storage.is_draft(draft.id).unwrap());
        draft.body = "Written 東京".into();
        let spans = vec![crate::text_editing::Span {
            start: 0,
            end: 7,
            mark: crate::text_editing::Mark::Bold,
        }];
        storage.update_formatted_note(&draft, &spans).unwrap();
        assert!(!storage.is_draft(draft.id).unwrap());
        assert!(storage.note_dates(draft.id).unwrap().0 > 0);
        assert_eq!(
            storage.load_deck_notes(4, NoteSort::Recent).unwrap()[0].id,
            draft.id
        );
        assert_eq!(
            storage
                .search_notes(NoteCollection::Active, "東京", NoteSort::Recent)
                .unwrap()[0]
                .id,
            draft.id
        );
        assert_eq!(storage.load_formatting(draft.id).unwrap(), spans);
        // Once published, clearing the text never turns a note into a draft again.
        draft.title.clear();
        draft.body.clear();
        storage.update_formatted_note(&draft, &[]).unwrap();
        assert!(!storage.is_draft(draft.id).unwrap());
        let notes = storage
            .search_notes(NoteCollection::Active, "", NoteSort::Recent)
            .unwrap();
        assert!(notes.iter().any(|n| n.id == draft.id));
        assert!(notes.iter().any(|n| n.id == existing_blank.id));
    }

    #[test]
    fn draft_failed_save_stays_hidden_until_retry_succeeds() {
        let storage = Storage::open(Path::new(":memory:")).unwrap();
        let mut draft = storage.create_draft(NoteColor::Yellow).unwrap();
        draft.title = "Ready to publish".into();
        storage
            .connection
            .pragma_update(None, "query_only", true)
            .unwrap();
        assert!(storage.update_note(&draft).is_err());
        assert!(storage.is_draft(draft.id).unwrap());
        assert!(storage.load_note(draft.id).unwrap().title.is_empty());
        assert!(
            storage
                .load_deck_notes(4, NoteSort::Recent)
                .unwrap()
                .is_empty()
        );
        assert!(
            storage
                .search_notes(NoteCollection::All, "", NoteSort::Recent)
                .unwrap()
                .is_empty()
        );
        storage
            .connection
            .pragma_update(None, "query_only", false)
            .unwrap();
        storage.update_note(&draft).unwrap();
        assert_eq!(
            storage
                .search_notes(NoteCollection::Active, "Ready", NoteSort::Recent)
                .unwrap(),
            vec![draft]
        );
    }

    #[test]
    fn draft_startup_cleanup_preserves_existing_blanks_and_published_notes() {
        let path = std::env::temp_dir().join(format!(
            "stickies-drafts-{}-{}.db",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let (orphan, existing, published) = {
            let storage = Storage::open(&path).unwrap();
            let existing = storage.create_note("", "", NoteColor::Yellow).unwrap();
            let mut orphan = storage.create_draft(NoteColor::Blue).unwrap();
            orphan.body = " \n\u{2003}".into();
            storage.update_note(&orphan).unwrap();
            storage
                .save_note_style(orphan.id, &crate::config::NoteStyle::default())
                .unwrap();
            let mut published = storage.create_draft(NoteColor::Green).unwrap();
            published.body = "Saved before exit".into();
            storage.update_note(&published).unwrap();
            (orphan.id, existing.id, published.id)
        };
        let storage = Storage::open(&path).unwrap();
        assert!(storage.load_note(orphan).is_err());
        assert!(storage.load_note(existing).is_ok());
        assert!(storage.load_note(published).is_ok());
        assert_eq!(
            storage
                .search_notes(NoteCollection::All, "", NoteSort::Recent)
                .unwrap()
                .len(),
            2
        );
        assert_eq!(
            storage
                .connection
                .query_row("SELECT count(*) FROM note_styles", [], |row| row
                    .get::<_, i64>(0))
                .unwrap(),
            0
        );
        assert_eq!(
            storage
                .connection
                .query_row("PRAGMA user_version", [], |row| row.get::<_, i64>(0))
                .unwrap(),
            SCHEMA_VERSION
        );
        drop(storage);
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn search_created_title_is_visible_immediately_without_draft_save() {
        let storage = Storage::open(Path::new(":memory:")).unwrap();
        let note = storage
            .create_note("Search-created title", "", NoteColor::Yellow)
            .unwrap();
        assert!(!storage.is_draft(note.id).unwrap());
        assert_eq!(
            storage
                .search_notes(NoteCollection::Active, "Search-created", NoteSort::Recent)
                .unwrap(),
            vec![note.clone()]
        );
        assert_eq!(
            storage.load_deck_notes(4, NoteSort::Recent).unwrap(),
            vec![note]
        );
    }

    #[test]
    fn search_casefolds_unicode_titles_and_bodies_without_changing_literal_matching() {
        let storage = Storage::open(Path::new(":memory:")).unwrap();
        let note = storage
            .create_note(
                "Ólafur Đặng Straße Σίσυφος café",
                "東京 ПРИВЕТ 100% _ literal's",
                NoteColor::Blue,
            )
            .unwrap();
        for query in [
            "ólafur",
            "đặng",
            "STRASSE",
            "σίσυφοσ",
            "привет",
            "東京",
            "%",
            "_",
            "LITERAL'S",
            "",
        ] {
            assert_eq!(
                storage
                    .search_notes(NoteCollection::Active, query, NoteSort::Recent)
                    .unwrap(),
                vec![note.clone()],
                "query {query:?}"
            );
        }
        assert!(
            storage
                .search_notes(NoteCollection::Active, "cafe", NoteSort::Recent)
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn edge_slots_follow_main_sort_and_search_reaches_overflow() {
        let storage = Storage::open(Path::new(":memory:")).unwrap();
        let notes: Vec<_> = ["Z", "B", "A", "D", "C", "E", "Archived", "Deleted"]
            .into_iter()
            .map(|title| storage.create_note(title, "Text", NoteColor::Blue).unwrap())
            .collect();
        storage
            .connection
            .execute_batch("UPDATE notes SET updated_at = id * 100; UPDATE notes SET updated_at = 900 WHERE title = 'Z'; UPDATE notes SET updated_at = 800 WHERE title = 'B';")
            .unwrap();
        storage.set_note_pinned(notes[0].id, true).unwrap();
        storage.set_note_pinned(notes[1].id, true).unwrap();
        storage.archive_note(notes[6].id).unwrap();
        storage.delete_note(notes[7].id).unwrap();
        for (sort, titles) in [
            (NoteSort::Recent, ["Z", "B", "E", "C"]),
            (NoteSort::Title, ["B", "Z", "A", "C"]),
        ] {
            let edge = storage.load_deck_notes(4, sort).unwrap();
            assert_eq!(
                edge.iter().map(|n| n.title.as_str()).collect::<Vec<_>>(),
                titles
            );
            let main = storage
                .search_notes(NoteCollection::Active, "", sort)
                .unwrap();
            assert_eq!(edge, main[..4]);
            assert_eq!(main.len(), 6);
        }
        let overflow = storage
            .search_notes(NoteCollection::Active, "D", NoteSort::Title)
            .unwrap();
        assert_eq!(overflow, vec![notes[3].clone()]);
    }

    #[test]
    fn undo_delete_restores_the_previous_collection_and_pin() {
        let storage = Storage::open(Path::new(":memory:")).unwrap();
        for archived in [false, true] {
            let note = storage
                .create_note("Keep", "Content", NoteColor::Green)
                .unwrap();
            storage.set_note_pinned(note.id, true).unwrap();
            if archived {
                storage.archive_note(note.id).unwrap();
            }
            storage.delete_note(note.id).unwrap();
            storage.undo_delete(note.id).unwrap();
            let collection = if archived {
                NoteCollection::Archived
            } else {
                NoteCollection::Active
            };
            let notes = storage
                .search_notes(collection, "", NoteSort::Recent)
                .unwrap();
            let restored = notes.iter().find(|n| n.id == note.id).unwrap();
            assert_eq!(restored.body, "Content");
            assert!(restored.pinned);
            assert!(storage.undo_delete(note.id).is_err());
        }
    }

    #[test]
    fn existing_notes_survive_style_migration_and_overrides_round_trip() {
        let connection = Connection::open_in_memory().unwrap();
        connection
            .execute_batch(include_str!("../migrations/001_initial.sql"))
            .unwrap();
        connection.pragma_update(None, "user_version", 1).unwrap();
        let mut storage = Storage { connection };
        let note = storage
            .create_note("Existing note", "Keep this content", NoteColor::Blue)
            .unwrap();
        storage.migrate().unwrap();
        assert_eq!(
            storage.load_note(note.id).unwrap().body,
            "Keep this content"
        );
        assert_eq!(
            storage.load_note_style(note.id).unwrap(),
            crate::config::NoteStyle::default()
        );
        storage
            .connection
            .execute(
                "INSERT INTO note_styles (note_id, settings) VALUES (?1, ?2)",
                params![
                    note.id.value() as i64,
                    "font = 'mono'\npaper = 'dots'\nshape = 'torn'"
                ],
            )
            .unwrap();
        let legacy = storage.load_note_style(note.id).unwrap();
        assert_eq!(legacy.font, Some(crate::config::Font::Mono));
        assert_eq!(legacy.text_size, crate::config::NoteTextSize::Medium);
        assert_eq!(
            legacy.legacy_text_weight,
            crate::config::LegacyTextWeight::Regular
        );
        assert_eq!(
            legacy.resolve(&crate::config::Config::default()).font,
            crate::config::Font::Mono
        );
        let style = crate::config::NoteStyle {
            text_size: crate::config::NoteTextSize::Large,
            legacy_text_weight: crate::config::LegacyTextWeight::Bold,
            paper: Some(crate::config::Paper::Dots),
            shape: Some(crate::config::Shape::Torn),
            ..crate::config::NoteStyle::default()
        };
        storage.save_note_style(note.id, &style).unwrap();
        assert_eq!(storage.load_note_style(note.id).unwrap(), style);
        storage.archive_note(note.id).unwrap();
        storage.restore_note(note.id).unwrap();
        assert_eq!(storage.load_note_style(note.id).unwrap(), style);
    }

    #[test]
    fn palette_migration_preserves_notes_styles_and_metadata() {
        let connection = Connection::open_in_memory().unwrap();
        connection
            .execute_batch(include_str!("../migrations/001_initial.sql"))
            .unwrap();
        connection
            .execute_batch(include_str!("../migrations/002_note_style.sql"))
            .unwrap();
        connection
            .execute_batch(
                "INSERT INTO notes VALUES
            (1, 'Amber', 'Active', 'yellow', 0, 20, 111, 222, NULL, NULL),
            (2, 'Blue', 'Pinned', 'blue', 1, 10, 333, 444, NULL, NULL),
            (3, 'Lilac', 'Archived', 'purple', 0, 30, 555, 666, 777, NULL),
            (4, 'Sage', 'Deleted', 'green', 1, 40, 888, 999, 1000, 1001);
            INSERT INTO note_styles VALUES (2, 'text_size = \"large\"');",
            )
            .unwrap();
        connection.pragma_update(None, "user_version", 2).unwrap();
        connection
            .pragma_update(None, "foreign_keys", true)
            .unwrap();
        fn snapshot(connection: &Connection) -> Vec<Vec<rusqlite::types::Value>> {
            connection
                .prepare("SELECT * FROM notes ORDER BY id")
                .unwrap()
                .query_map([], |row| (0..10).map(|i| row.get(i)).collect())
                .unwrap()
                .collect::<Result<_, _>>()
                .unwrap()
        }
        let before = snapshot(&connection);
        let mut storage = Storage { connection };
        storage.migrate().unwrap();
        assert_eq!(snapshot(&storage.connection), before);
        assert_eq!(
            storage.load_note_style(NoteId::new(2)).unwrap().text_size,
            crate::config::NoteTextSize::Large
        );
        assert_eq!(
            storage
                .connection
                .query_row("SELECT count(*) FROM pragma_foreign_key_check", [], |row| {
                    row.get::<_, i64>(0)
                })
                .unwrap(),
            0
        );
        for color in NoteColor::ALL {
            let note = storage
                .create_note(color.label(), "Palette", color)
                .unwrap();
            storage.archive_note(note.id).unwrap();
            storage.delete_note(note.id).unwrap();
            storage.undo_delete(note.id).unwrap();
            storage.restore_note(note.id).unwrap();
            assert_eq!(storage.load_note(note.id).unwrap().color, color);
        }
    }

    #[test]
    fn failed_palette_migration_rolls_back_tables_and_version() {
        let connection = Connection::open_in_memory().unwrap();
        connection
            .execute_batch(include_str!("../migrations/001_initial.sql"))
            .unwrap();
        connection
            .execute_batch(include_str!("../migrations/002_note_style.sql"))
            .unwrap();
        // A corrupt orphan style must reject the migration without dropping data.
        connection
            .execute_batch("INSERT INTO note_styles VALUES (99, 'text_size = \"large\"');")
            .unwrap();
        connection.pragma_update(None, "user_version", 2).unwrap();
        connection
            .pragma_update(None, "foreign_keys", true)
            .unwrap();
        let mut storage = Storage { connection };
        let note = storage
            .create_note("Keep", "Content", NoteColor::Blue)
            .unwrap();
        assert!(storage.migrate().is_err());
        assert_eq!(storage.load_note(note.id).unwrap(), note);
        assert_eq!(
            storage
                .connection
                .query_row("PRAGMA user_version", [], |row| row.get::<_, i64>(0))
                .unwrap(),
            2
        );
        assert_eq!(
            storage.load_note_style(NoteId::new(99)).unwrap().text_size,
            crate::config::NoteTextSize::Large
        );
        assert_eq!(storage.connection.query_row("SELECT count(*) FROM sqlite_master WHERE name IN ('notes_new', 'note_styles_new')", [], |row| row.get::<_, i64>(0)).unwrap(), 0);
    }

    #[test]
    fn a_failed_write_preserves_saved_content_until_retry() {
        let connection = Connection::open_in_memory().expect("database should open");
        let mut storage = Storage { connection };
        storage.migrate().expect("migration should run");
        let mut note = storage
            .create_note("Work", "Saved", NoteColor::Yellow)
            .unwrap();
        note.body = "Unsaved".into();

        storage
            .connection
            .pragma_update(None, "query_only", true)
            .unwrap();
        assert!(storage.update_note(&note).is_err());
        assert_eq!(
            storage.load_deck_notes(5, NoteSort::Recent).unwrap()[0].body,
            "Saved"
        );

        storage
            .connection
            .pragma_update(None, "query_only", false)
            .unwrap();
        storage.update_note(&note).unwrap();
        assert_eq!(
            storage.load_deck_notes(5, NoteSort::Recent).unwrap()[0].body,
            "Unsaved"
        );
    }

    #[test]
    fn empty_databases_stay_empty_and_user_notes_survive_reopening() {
        let path = std::env::temp_dir().join(format!(
            "stickies-storage-{}-{}.db",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time must follow the Unix epoch")
                .as_nanos()
        ));
        {
            let storage = Storage::open(&path).expect("database should open");
            assert!(
                storage
                    .load_deck_notes(5, NoteSort::Recent)
                    .unwrap()
                    .is_empty()
            );
        }

        {
            let storage = Storage::open(&path).expect("empty database should reopen");
            assert!(
                storage
                    .load_deck_notes(5, NoteSort::Recent)
                    .unwrap()
                    .is_empty()
            );
            let mut note = storage
                .create_note("Work", "Initial body", NoteColor::Yellow)
                .expect("user note should be created");
            note.body = "Changed body".to_owned();
            storage
                .update_note(&note)
                .expect("edited note should be stored");
        }

        {
            let storage = Storage::open(&path).expect("database should reopen");
            let notes = storage
                .load_deck_notes(5, NoteSort::Recent)
                .expect("stored notes should load");
            assert_eq!(notes.len(), 1);
            assert_eq!(notes[0].title, "Work");
            assert_eq!(notes[0].body, "Changed body");
            storage.delete_note(notes[0].id).unwrap();
        }

        {
            let storage = Storage::open(&path).expect("database should reopen after deletion");
            assert!(
                storage
                    .load_deck_notes(5, NoteSort::Recent)
                    .unwrap()
                    .is_empty()
            );
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
        assert!(
            matches!(error, StorageError::UnsupportedSchema(version) if version == SCHEMA_VERSION + 1)
        );
        assert_eq!(
            storage
                .connection
                .query_row("PRAGMA user_version", [], |row| row.get::<_, i64>(0))
                .expect("schema version should remain readable"),
            SCHEMA_VERSION + 1
        );
    }

    #[test]
    fn create_archive_delete_and_deck_limit_are_persisted() {
        let connection = Connection::open_in_memory().expect("database should open");
        let mut storage = Storage { connection };
        storage.migrate().expect("migration should run");
        storage
            .create_note("Existing", "", NoteColor::Blue)
            .expect("existing note should be created");

        let first = storage
            .create_note("First", "Body", NoteColor::Yellow)
            .expect("note should be created");
        let second = storage
            .create_note("Second", "", NoteColor::Green)
            .expect("note should be created");
        assert_eq!(
            storage
                .load_deck_notes(1, NoteSort::Recent)
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
                .load_deck_notes(5, NoteSort::Recent)
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
        let storage = Storage::open(Path::new(":memory:")).expect("database should open");

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

        let all = storage
            .search_notes(NoteCollection::All, "ginger", NoteSort::Recent)
            .unwrap();
        assert_eq!(all.len(), 2);
        assert!(all.contains(&active) && all.contains(&archived));
        assert!(!storage.is_archived(active.id).unwrap());
        assert!(storage.is_archived(archived.id).unwrap());

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
                .load_deck_notes(5, NoteSort::Recent)
                .expect("deck should load pinned note")[0]
                .pinned
        );
    }
}
