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
    ArchiveOpenNote,
    DeleteOpenNote,
    CollapseDeck,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Event {
    Deck(DeckState),
    NoteAdded(NoteId),
    NoteArchived(NoteId),
    NoteDeleted(NoteId),
}

pub struct AppState {
    notes: Vec<Note>,
    deck: DeckState,
}

impl AppState {
    pub fn new(notes: Vec<Note>) -> Self {
        Self {
            notes,
            deck: DeckState::Dormant,
        }
    }

    pub fn notes(&self) -> &[Note] {
        &self.notes
    }

    pub fn note(&self, id: NoteId) -> Option<&Note> {
        self.notes.iter().find(|note| note.id == id)
    }

    pub fn replace_notes(&mut self, notes: Vec<Note>) -> bool {
        if matches!(self.deck, DeckState::Open(_)) {
            return false;
        }
        self.notes = notes;
        self.deck = DeckState::Dormant;
        true
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
                self.note(id)?;
                self.deck = DeckState::Open(id);
                Some(Event::Deck(self.deck))
            }
            Action::ArchiveOpenNote => self.remove_open_note(false),
            Action::DeleteOpenNote => self.remove_open_note(true),
            Action::CollapseDeck => {
                self.deck = DeckState::Dormant;
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
        let DeckState::Open(id) = self.deck else {
            return None;
        };
        let index = self.notes.iter().position(|note| note.id == id)?;
        self.notes.remove(index);
        self.deck = DeckState::Dormant;

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

        assert_eq!(state.deck, DeckState::Dormant);
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
    fn an_unknown_note_cannot_be_opened() {
        let mut state = state();

        assert_eq!(state.dispatch(Action::OpenNote(NoteId::new(99))), None);
        assert_eq!(state.deck, DeckState::Dormant);
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
        assert_eq!(archived.deck, DeckState::Dormant);

        let mut deleted = state();
        deleted.dispatch(Action::OpenNote(NoteId::new(1)));
        assert_eq!(
            deleted.dispatch(Action::DeleteOpenNote),
            Some(Event::NoteDeleted(NoteId::new(1)))
        );
        assert!(deleted.notes().is_empty());
    }

    #[test]
    fn creating_keeps_pinned_notes_first() {
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
    }
}
