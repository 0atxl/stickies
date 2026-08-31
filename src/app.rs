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
}

impl Note {
    pub fn new(id: NoteId, title: &str, body: &str, color: NoteColor) -> Self {
        Self {
            id,
            title: title.to_owned(),
            body: body.to_owned(),
            color,
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
    OpenNote(NoteId),
    UpdateOpenNoteBody(String),
    SetKeepOpen(bool),
    CollapseDeck,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Event {
    Deck(DeckState),
    NoteEdited(NoteId),
    KeepOpen(bool),
}

pub struct AppState {
    notes: Vec<Note>,
    deck: DeckState,
    keep_open: bool,
}

impl AppState {
    pub fn new(notes: Vec<Note>) -> Self {
        Self {
            notes,
            deck: DeckState::Dormant,
            keep_open: false,
        }
    }

    pub fn notes(&self) -> &[Note] {
        &self.notes
    }

    pub fn note(&self, id: NoteId) -> Option<&Note> {
        self.notes.iter().find(|note| note.id == id)
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
            Action::OpenNote(id) => {
                self.note(id)?;
                self.deck = DeckState::Open(id);
                self.keep_open = false;
                Some(Event::Deck(self.deck))
            }
            Action::UpdateOpenNoteBody(body) => {
                let DeckState::Open(id) = self.deck else {
                    return None;
                };
                let note = self.notes.iter_mut().find(|note| note.id == id)?;

                if note.body == body {
                    return None;
                }

                note.body = body;
                Some(Event::NoteEdited(id))
            }
            Action::SetKeepOpen(keep_open) => {
                if !matches!(self.deck, DeckState::Open(_)) || self.keep_open == keep_open {
                    return None;
                }

                self.keep_open = keep_open;
                Some(Event::KeepOpen(keep_open))
            }
            Action::CollapseDeck => {
                self.deck = DeckState::Dormant;
                self.keep_open = false;
                Some(Event::Deck(self.deck))
            }
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
        state.dispatch(Action::OpenNote(note_id));

        assert_eq!(
            state.dispatch(Action::UpdateOpenNoteBody("Changed body".to_owned())),
            Some(Event::NoteEdited(note_id))
        );
        assert_eq!(
            state.note(note_id).map(|note| note.body.as_str()),
            Some("Changed body")
        );
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
}
