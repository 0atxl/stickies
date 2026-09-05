#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct NoteId(u64);

impl NoteId {
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    pub const fn value(self) -> u64 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NoteColor {
    Yellow,
    Blue,
    Purple,
    Green,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Note {
    pub id: NoteId,
    pub title: String,
    pub body: String,
    pub color: NoteColor,
    pub pinned: bool,
}

impl Note {
    pub fn new(id: NoteId, title: &str, body: &str, color: NoteColor) -> Self {
        Self {
            id,
            title: title.to_owned(),
            body: body.to_owned(),
            color,
            pinned: false,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DeckState {
    Dormant,
    Tabs,
    Open(NoteId),
}

#[derive(Debug, Eq, PartialEq)]
pub enum Action {
    ShowTabs,
    AddNote(Note),
    OpenNote(NoteId),
    UpdateOpenNoteTitle(String),
    UpdateOpenNoteBody(String),
    ArchiveOpenNote,
    DeleteOpenNote,
    SetKeepOpen(bool),
    CollapseDeck,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Event {
    Deck(DeckState),
    NoteAdded(NoteId),
    NoteEdited(NoteId),
    NoteArchived(NoteId),
    NoteDeleted(NoteId),
    KeepOpen(bool),
}

pub struct AppState {
    notes: Vec<Note>,
    deck: DeckState,
    keep_open: bool,
    pending_note: Option<NoteId>,
}

impl AppState {
    pub fn new(notes: Vec<Note>) -> Self {
        Self {
            notes,
            deck: DeckState::Dormant,
            keep_open: false,
            pending_note: None,
        }
    }

    pub fn notes(&self) -> &[Note] {
        &self.notes
    }

    pub fn note(&self, id: NoteId) -> Option<&Note> {
        self.notes.iter().find(|note| note.id == id)
    }

    pub fn replace_notes(&mut self, notes: Vec<Note>) -> bool {
        if matches!(self.deck, DeckState::Open(_)) || self.pending_note.is_some() {
            return false;
        }
        self.notes = notes;
        self.deck = DeckState::Dormant;
        self.keep_open = false;
        true
    }

    pub fn pending_note(&self) -> Option<&Note> {
        self.pending_note.and_then(|id| self.note(id))
    }

    pub fn note_saved(&mut self, id: NoteId) {
        if self.pending_note == Some(id) {
            self.pending_note = None;
        }
    }

    pub const fn deck(&self) -> DeckState {
        self.deck
    }

    pub const fn keep_open(&self) -> bool {
        self.keep_open
    }

    pub fn dispatch(&mut self, action: Action) -> Option<Event> {
        match action {
            Action::ShowTabs => {
                if matches!(self.deck, DeckState::Open(_)) {
                    return None;
                }

                self.deck = DeckState::Tabs;
                Some(Event::Deck(self.deck))
            }
            Action::AddNote(note) => {
                if self.note(note.id).is_some() {
                    return None;
                }

                let id = note.id;
                self.insert_recent_note(note);
                Some(Event::NoteAdded(id))
            }
            Action::OpenNote(id) => {
                if self.pending_note.is_some() {
                    return None;
                }
                self.note(id)?;
                self.deck = DeckState::Open(id);
                self.keep_open = false;
                Some(Event::Deck(self.deck))
            }
            Action::UpdateOpenNoteTitle(title) => {
                let DeckState::Open(id) = self.deck else {
                    return None;
                };
                let index = self.notes.iter().position(|note| note.id == id)?;

                if self.notes[index].title == title {
                    return None;
                }

                self.notes[index].title = title;
                let note = self.notes.remove(index);
                self.insert_recent_note(note);
                self.pending_note = Some(id);
                Some(Event::NoteEdited(id))
            }
            Action::UpdateOpenNoteBody(body) => {
                let DeckState::Open(id) = self.deck else {
                    return None;
                };
                let index = self.notes.iter().position(|note| note.id == id)?;

                if self.notes[index].body == body {
                    return None;
                }

                self.notes[index].body = body;
                let note = self.notes.remove(index);
                self.insert_recent_note(note);
                self.pending_note = Some(id);
                Some(Event::NoteEdited(id))
            }
            Action::ArchiveOpenNote => self.remove_open_note(false),
            Action::DeleteOpenNote => self.remove_open_note(true),
            Action::SetKeepOpen(keep_open) => {
                if !matches!(self.deck, DeckState::Open(_)) || self.keep_open == keep_open {
                    return None;
                }

                self.keep_open = keep_open;
                Some(Event::KeepOpen(keep_open))
            }
            Action::CollapseDeck => {
                if self.pending_note.is_some() {
                    return None;
                }
                self.deck = DeckState::Dormant;
                self.keep_open = false;
                Some(Event::Deck(self.deck))
            }
        }
    }

    fn insert_recent_note(&mut self, note: Note) {
        let index = if note.pinned {
            0
        } else {
            self.notes.iter().take_while(|note| note.pinned).count()
        };
        self.notes.insert(index, note);
    }

    fn remove_open_note(&mut self, delete: bool) -> Option<Event> {
        if self.pending_note.is_some() {
            return None;
        }
        let DeckState::Open(id) = self.deck else {
            return None;
        };
        let index = self.notes.iter().position(|note| note.id == id)?;
        self.notes.remove(index);
        self.deck = DeckState::Dormant;
        self.keep_open = false;

        if delete {
            Some(Event::NoteDeleted(id))
        } else {
            Some(Event::NoteArchived(id))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state() -> AppState {
        AppState::new(vec![Note::new(
            NoteId::new(1),
            "Work",
            "Initial body",
            NoteColor::Yellow,
        )])
    }

    #[test]
    fn deck_moves_from_dormant_to_tabs_to_open_and_back() {
        let mut state = state();
        let note_id = NoteId::new(1);

        assert_eq!(state.deck(), DeckState::Dormant);
        assert_eq!(
            state.dispatch(Action::ShowTabs),
            Some(Event::Deck(DeckState::Tabs))
        );
        assert_eq!(
            state.dispatch(Action::OpenNote(note_id)),
            Some(Event::Deck(DeckState::Open(note_id)))
        );
        assert_eq!(
            state.dispatch(Action::CollapseDeck),
            Some(Event::Deck(DeckState::Dormant))
        );
    }

    #[test]
    fn editing_an_open_note_updates_its_body() {
        let mut state = state();
        let note_id = NoteId::new(1);
        state.dispatch(Action::AddNote(Note::new(
            NoteId::new(2),
            "Later note",
            "",
            NoteColor::Blue,
        )));
        state.dispatch(Action::OpenNote(note_id));

        assert_eq!(
            state.dispatch(Action::UpdateOpenNoteBody("Changed body".to_owned())),
            Some(Event::NoteEdited(note_id))
        );
        assert_eq!(
            state.note(note_id).map(|note| note.body.as_str()),
            Some("Changed body")
        );
        assert_eq!(state.notes()[0].id, note_id);
    }

    #[test]
    fn an_unknown_note_cannot_be_opened() {
        let mut state = state();

        assert_eq!(state.dispatch(Action::OpenNote(NoteId::new(99))), None);
        assert_eq!(state.deck(), DeckState::Dormant);
    }

    #[test]
    fn keep_open_only_applies_to_the_current_open_session() {
        let mut state = state();

        assert_eq!(state.dispatch(Action::SetKeepOpen(true)), None);
        state.dispatch(Action::OpenNote(NoteId::new(1)));
        assert_eq!(
            state.dispatch(Action::SetKeepOpen(true)),
            Some(Event::KeepOpen(true))
        );
        assert!(state.keep_open());

        state.dispatch(Action::CollapseDeck);
        assert!(!state.keep_open());
    }

    #[test]
    fn a_note_can_be_added_and_its_title_edited() {
        let mut state = state();
        let note = Note::new(NoteId::new(2), "Untitled note", "", NoteColor::Yellow);

        assert_eq!(
            state.dispatch(Action::AddNote(note)),
            Some(Event::NoteAdded(NoteId::new(2)))
        );
        state.dispatch(Action::OpenNote(NoteId::new(2)));
        assert_eq!(
            state.dispatch(Action::UpdateOpenNoteTitle("Ideas".to_owned())),
            Some(Event::NoteEdited(NoteId::new(2)))
        );
        assert_eq!(
            state.note(NoteId::new(2)).map(|note| note.title.as_str()),
            Some("Ideas")
        );
    }

    #[test]
    fn archiving_or_deleting_removes_the_open_note() {
        let mut archived = state();
        archived.dispatch(Action::OpenNote(NoteId::new(1)));
        assert_eq!(
            archived.dispatch(Action::ArchiveOpenNote),
            Some(Event::NoteArchived(NoteId::new(1)))
        );
        assert!(archived.notes().is_empty());
        assert_eq!(archived.deck(), DeckState::Dormant);

        let mut deleted = state();
        deleted.dispatch(Action::OpenNote(NoteId::new(1)));
        assert_eq!(
            deleted.dispatch(Action::DeleteOpenNote),
            Some(Event::NoteDeleted(NoteId::new(1)))
        );
        assert!(deleted.notes().is_empty());
    }

    #[test]
    fn pending_edits_block_session_changes_until_the_same_note_is_saved() {
        let mut state = state();
        let id = NoteId::new(1);
        state.dispatch(Action::AddNote(Note::new(
            NoteId::new(2),
            "Other",
            "",
            NoteColor::Blue,
        )));
        state.dispatch(Action::OpenNote(id));
        state.dispatch(Action::SetKeepOpen(true));
        state.dispatch(Action::UpdateOpenNoteBody("Unsaved".to_owned()));

        state.note_saved(NoteId::new(2));
        for action in [
            Action::CollapseDeck,
            Action::OpenNote(NoteId::new(2)),
            Action::ArchiveOpenNote,
            Action::DeleteOpenNote,
            Action::ShowTabs,
        ] {
            assert_eq!(state.dispatch(action), None);
        }
        assert!(!state.replace_notes(Vec::new()));
        assert_eq!(state.deck(), DeckState::Open(id));
        assert!(state.keep_open());
        assert_eq!(state.pending_note().unwrap().body, "Unsaved");

        state.note_saved(id);
        assert!(state.pending_note().is_none());
        // Even a saved open note must survive a background deck refresh.
        assert!(!state.replace_notes(Vec::new()));
        assert_eq!(state.deck(), DeckState::Open(id));
        assert!(state.keep_open());
        assert_eq!(
            state.dispatch(Action::CollapseDeck),
            Some(Event::Deck(DeckState::Dormant))
        );
        assert!(state.replace_notes(Vec::new()));
    }

    #[test]
    fn creating_and_editing_keep_pinned_notes_first() {
        let mut pinned = Note::new(NoteId::new(1), "Pinned", "", NoteColor::Yellow);
        pinned.pinned = true;
        let mut state = AppState::new(vec![pinned]);
        for id in [2, 3] {
            state.dispatch(Action::AddNote(Note::new(
                NoteId::new(id),
                "Other",
                "",
                NoteColor::Blue,
            )));
        }
        assert_eq!(
            state
                .notes()
                .iter()
                .map(|note| note.id.value())
                .collect::<Vec<_>>(),
            vec![1, 3, 2]
        );
        state.dispatch(Action::OpenNote(NoteId::new(2)));
        for action in [
            Action::UpdateOpenNoteTitle("Edited title".to_owned()),
            Action::UpdateOpenNoteBody("Edited body".to_owned()),
        ] {
            state.dispatch(action);
            assert_eq!(
                state
                    .notes()
                    .iter()
                    .map(|note| note.id.value())
                    .collect::<Vec<_>>(),
                vec![1, 2, 3]
            );
        }
    }
}
