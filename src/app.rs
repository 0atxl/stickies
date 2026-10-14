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
    Orange,
    Red,
    Pink,
    Lime,
    Teal,
    Cyan,
    Indigo,
    Magenta,
    Brown,
    Gray,
    Black,
    White,
}

impl NoteColor {
    pub const ALL: [Self; 16] = [
        Self::Yellow,
        Self::Orange,
        Self::Red,
        Self::Pink,
        Self::Green,
        Self::Lime,
        Self::Teal,
        Self::Cyan,
        Self::Blue,
        Self::Indigo,
        Self::Purple,
        Self::Magenta,
        Self::Brown,
        Self::Gray,
        Self::Black,
        Self::White,
    ];

    pub const fn name(self) -> &'static str {
        match self {
            Self::Yellow => "yellow",
            Self::Blue => "blue",
            Self::Purple => "purple",
            Self::Green => "green",
            Self::Orange => "orange",
            Self::Red => "red",
            Self::Pink => "pink",
            Self::Lime => "lime",
            Self::Teal => "teal",
            Self::Cyan => "cyan",
            Self::Indigo => "indigo",
            Self::Magenta => "magenta",
            Self::Brown => "brown",
            Self::Gray => "gray",
            Self::Black => "black",
            Self::White => "white",
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Yellow => "Amber",
            Self::Blue => "Blue",
            Self::Purple => "Lilac",
            Self::Green => "Sage",
            Self::Orange => "Orange",
            Self::Red => "Red",
            Self::Pink => "Pink",
            Self::Lime => "Lime",
            Self::Teal => "Teal",
            Self::Cyan => "Cyan",
            Self::Indigo => "Indigo",
            Self::Magenta => "Magenta",
            Self::Brown => "Brown",
            Self::Gray => "Gray",
            Self::Black => "Black",
            Self::White => "White",
        }
    }

    pub const fn accent(self) -> (u8, u8, u8) {
        match self {
            Self::Yellow => (217, 140, 43),
            Self::Blue => (91, 141, 239),
            Self::Purple => (169, 143, 209),
            Self::Green => (111, 174, 104),
            Self::Orange => (239, 117, 48),
            Self::Red => (218, 72, 72),
            Self::Pink => (232, 103, 156),
            Self::Lime => (153, 185, 57),
            Self::Teal => (43, 153, 143),
            Self::Cyan => (45, 169, 201),
            Self::Indigo => (101, 105, 201),
            Self::Magenta => (183, 77, 177),
            Self::Brown => (154, 112, 82),
            Self::Gray => (136, 144, 153),
            Self::Black => (50, 54, 61),
            Self::White => (244, 244, 244),
        }
    }

    pub const fn fill(self) -> (f64, f64, f64) {
        match self {
            Self::Yellow => (0.98, 0.93, 0.74),
            Self::Blue => (0.71, 0.85, 1.),
            Self::Purple => (0.86, 0.80, 0.94),
            Self::Green => (0.75, 0.97, 0.81),
            Self::Orange => (1., 0.83, 0.67),
            Self::Red => (1., 0.77, 0.76),
            Self::Pink => (1., 0.81, 0.89),
            Self::Lime => (0.89, 0.95, 0.68),
            Self::Teal => (0.69, 0.91, 0.86),
            Self::Cyan => (0.71, 0.92, 0.97),
            Self::Indigo => (0.80, 0.82, 0.96),
            Self::Magenta => (0.95, 0.78, 0.93),
            Self::Brown => (0.88, 0.81, 0.73),
            Self::Gray => (0.85, 0.88, 0.91),
            Self::Black => (0.73, 0.76, 0.80),
            Self::White => (0.98, 0.98, 0.98),
        }
    }
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

    #[cfg(test)]
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
