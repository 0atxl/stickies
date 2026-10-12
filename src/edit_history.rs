//! Session-local snapshots. Typing groups never change persisted formatting.
use crate::text_editing::Span;
use std::time::Duration;

#[derive(Clone, Default, PartialEq)]
pub struct EditState {
    pub title: String,
    pub body: String,
    pub spans: Vec<Span>,
    pub cursor: i32,
    pub selection: i32,
}

#[derive(Clone, Copy, PartialEq)]
pub enum Field {
    Title,
    Body,
}
#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Insert,
    Delete,
}
#[derive(Clone)]
struct Change {
    field: Field,
    start: usize,
    removed: usize,
    inserted: usize,
    newline: bool,
}
impl Change {
    fn between(before: &EditState, after: &EditState) -> Option<Self> {
        let (field, before, after) = if before.title != after.title {
            (Field::Title, &before.title, &after.title)
        } else if before.body != after.body {
            (Field::Body, &before.body, &after.body)
        } else {
            return None;
        };
        let before: Vec<_> = before.chars().collect();
        let after: Vec<_> = after.chars().collect();
        let start = before
            .iter()
            .zip(&after)
            .take_while(|(a, b)| a == b)
            .count();
        let suffix = before[start..]
            .iter()
            .rev()
            .zip(after[start..].iter().rev())
            .take_while(|(a, b)| a == b)
            .count();
        let removed = before.len() - start - suffix;
        let inserted = after.len() - start - suffix;
        Some(Self {
            field,
            start,
            removed,
            inserted,
            newline: after[start..start + inserted].contains(&'\n'),
        })
    }
}
struct Group {
    field: Field,
    kind: Kind,
    caret: usize,
    characters: usize,
    at: Duration,
}
pub struct History {
    states: Vec<Step>,
    index: usize,
    group: Option<Group>,
    clock: Box<dyn Fn() -> Duration>,
    origin: Option<(Field, usize)>,
}
struct Step {
    state: EditState,
    change: Option<Change>,
    format_range: Option<(i32, i32)>,
}
pub struct Restored {
    pub state: EditState,
    pub field: Field,
    pub cursor: i32,
    pub selection: i32,
}
impl History {
    pub fn new(state: EditState, clock: impl Fn() -> Duration + 'static) -> Self {
        Self {
            states: vec![Step {
                state,
                change: None,
                format_range: None,
            }],
            index: 0,
            group: None,
            clock: Box::new(clock),
            origin: None,
        }
    }
    pub fn boundary(&mut self) {
        self.group = None;
        self.origin = None;
    }
    pub fn note_change(&mut self, field: Field, start: usize) {
        self.origin.get_or_insert((field, start));
    }
    pub fn record(&mut self, state: EditState, isolated: bool) -> bool {
        let previous = &self.states[self.index].state;
        if previous.title == state.title
            && previous.body == state.body
            && previous.spans == state.spans
        {
            // Selection changes are metadata, never their own undo step.
            self.states[self.index].state.cursor = state.cursor;
            self.states[self.index].state.selection = state.selection;
            return false;
        }
        let at = (self.clock)();
        let mut change = Change::between(previous, &state);
        if let Some((field, start)) = self.origin.take()
            && let Some(change) = change.as_mut()
            && field == change.field
        {
            let (before, after) = if field == Field::Title {
                (&previous.title, &state.title)
            } else {
                (&previous.body, &state.body)
            };
            let before: Vec<_> = before.chars().collect();
            let after: Vec<_> = after.chars().collect();
            if start + change.removed <= before.len()
                && start + change.inserted <= after.len()
                && before[..start] == after[..start]
                && before[start + change.removed..] == after[start + change.inserted..]
            {
                change.start = start;
            }
        }
        let kind = change
            .as_ref()
            .and_then(|change| match (change.removed, change.inserted) {
                (0, _) => Some(Kind::Insert),
                (_, 0) => Some(Kind::Delete),
                _ => None,
            });
        let typing = change.as_ref().filter(|change| {
            !isolated
                && !change.newline
                && !(previous.title != state.title && previous.body != state.body)
        });
        let merge = typing.zip(kind).is_some_and(|(change, kind)| {
            self.group.as_ref().is_some_and(|group| {
                group.field == change.field
                    && group.kind == kind
                    && at.saturating_sub(group.at) < Duration::from_secs(1)
                    && group.characters + change.inserted + change.removed <= 100
                    && (change.start == group.caret
                        || (kind == Kind::Delete && change.start + change.removed == group.caret))
            })
        });
        let group = typing.zip(kind).map(|(change, kind)| Group {
            field: change.field,
            kind,
            caret: change.start + change.inserted,
            characters: change.inserted
                + change.removed
                + if merge {
                    self.group.as_ref().unwrap().characters
                } else {
                    0
                },
            at,
        });
        let format_range = change.is_none().then_some((state.cursor, state.selection));
        self.states.truncate(self.index + 1);
        if merge {
            let first = self.states[self.index].change.as_ref().unwrap();
            let latest = change.as_mut().unwrap();
            latest.start = latest.start.min(first.start);
            latest.inserted += first.inserted;
            latest.removed += first.removed;
            self.states[self.index] = Step {
                state,
                change,
                format_range,
            };
        } else {
            self.states.push(Step {
                state,
                change,
                format_range,
            });
            if self.states.len() > 100 {
                self.states.remove(0);
            }
            self.index = self.states.len() - 1;
        }
        self.group = group;
        true
    }
    pub fn restore(&mut self, redo: bool) -> Option<Restored> {
        self.boundary();
        let next = if redo {
            self.index + 1
        } else {
            self.index.checked_sub(1)?
        };
        let state = self.states.get(next)?.state.clone();
        let previous = &self.states[self.index].state;
        let step = &self.states[if redo { next } else { self.index }];
        let (field, cursor, selection) = if let Some(change) = &step.change {
            let cursor = (change.start
                + if redo {
                    change.inserted
                } else {
                    change.removed
                }) as i32;
            (change.field, cursor, cursor)
        } else if let Some((cursor, selection)) = step.format_range
            && cursor != selection
        {
            (Field::Body, cursor, selection)
        } else {
            // Preserve the marked range when undoing/redoing formatting alone.
            let changed: Vec<_> = previous
                .spans
                .iter()
                .chain(&state.spans)
                .filter(|span| previous.spans.contains(span) != state.spans.contains(span))
                .collect();
            let start = changed
                .iter()
                .map(|s| s.start)
                .min()
                .unwrap_or(state.selection);
            let end = changed.iter().map(|s| s.end).max().unwrap_or(state.cursor);
            (Field::Body, end, start)
        };
        self.index = next;
        Some(Restored {
            state,
            field,
            cursor,
            selection,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::text_editing::Mark;
    use std::{cell::Cell, rc::Rc};

    fn history() -> (History, Rc<Cell<Duration>>, EditState) {
        let time = Rc::new(Cell::new(Duration::ZERO));
        let clock = time.clone();
        let state = EditState::default();
        (
            History::new(state.clone(), move || clock.get()),
            time,
            state,
        )
    }
    fn type_text(history: &mut History, time: &Cell<Duration>, state: &mut EditState, text: &str) {
        for c in text.chars() {
            state.body.push(c);
            time.set(time.get() + Duration::from_millis(20));
            history.record(state.clone(), false);
        }
    }
    #[test]
    fn quick_hello_world_is_one_step() {
        let (mut history, time, mut state) = history();
        type_text(&mut history, &time, &mut state, "hello world");
        assert_eq!(history.restore(false).unwrap().state.body, "");
        assert!(history.restore(false).is_none());
        let redo = history.restore(true).unwrap();
        assert_eq!(redo.state.body, "hello world");
        assert_eq!((redo.cursor, redo.selection), (11, 11));
    }
    #[test]
    fn pause_separates_typing_without_sleeping() {
        let (mut history, time, mut state) = history();
        type_text(&mut history, &time, &mut state, "hello");
        time.set(time.get() + Duration::from_millis(1200));
        type_text(&mut history, &time, &mut state, " world");
        assert_eq!(history.restore(false).unwrap().state.body, "hello");
        assert_eq!(history.restore(false).unwrap().state.body, "");
    }
    #[test]
    fn newline_is_its_own_step() {
        let (mut history, time, mut state) = history();
        type_text(&mut history, &time, &mut state, "hello\nworld");
        assert_eq!(history.restore(false).unwrap().state.body, "hello\n");
        assert_eq!(history.restore(false).unwrap().state.body, "hello");
        assert_eq!(history.restore(false).unwrap().state.body, "");
    }
    #[test]
    fn backspace_run_is_separate_from_typing() {
        let (mut history, time, mut state) = history();
        type_text(&mut history, &time, &mut state, "hello world");
        for _ in 0..5 {
            state.body.pop();
            history.record(state.clone(), false);
        }
        assert_eq!(history.restore(false).unwrap().state.body, "hello world");
        assert_eq!(history.restore(false).unwrap().state.body, "");
    }
    #[test]
    fn paste_is_isolated_even_when_adjacent_to_typing() {
        let (mut history, time, mut state) = history();
        type_text(&mut history, &time, &mut state, "hello");
        state.body.push_str(" pasted");
        history.record(state.clone(), true);
        type_text(&mut history, &time, &mut state, "!");
        assert_eq!(history.restore(false).unwrap().state.body, "hello pasted");
        assert_eq!(history.restore(false).unwrap().state.body, "hello");
        assert_eq!(history.restore(false).unwrap().state.body, "");
    }
    #[test]
    fn bold_toggle_restores_spans_and_text_together() {
        let (mut history, time, mut state) = history();
        type_text(&mut history, &time, &mut state, "hello");
        state.spans.push(Span {
            start: 0,
            end: 5,
            mark: Mark::Bold,
        });
        history.record(state.clone(), true);
        let undo = history.restore(false).unwrap();
        assert_eq!(undo.state.body, "hello");
        assert!(undo.state.spans.is_empty());
        assert_eq!((undo.cursor, undo.selection), (5, 0));
        assert_eq!(history.restore(true).unwrap().state.spans, state.spans);
    }
    #[test]
    fn new_edit_after_undo_clears_redo() {
        let (mut history, time, mut state) = history();
        type_text(&mut history, &time, &mut state, "hello");
        state = history.restore(false).unwrap().state;
        type_text(&mut history, &time, &mut state, "new");
        assert!(history.restore(true).is_none());
        assert_eq!(history.restore(false).unwrap().state.body, "");
    }
    #[test]
    fn unicode_uses_character_offsets_and_groups() {
        let (mut history, time, mut state) = history();
        let text = "café 東京 👩🏽‍💻";
        type_text(&mut history, &time, &mut state, text);
        assert_eq!(history.restore(false).unwrap().state.body, "");
        let redo = history.restore(true).unwrap();
        assert_eq!(redo.state.body, text);
        assert_eq!(redo.cursor, text.chars().count() as i32);
    }
    #[test]
    fn cursor_movement_and_character_cap_start_new_steps() {
        let (mut history, time, mut state) = history();
        type_text(&mut history, &time, &mut state, &"a".repeat(101));
        assert_eq!(
            history.restore(false).unwrap().state.body.chars().count(),
            100
        );
        state = history.restore(true).unwrap().state;
        history.boundary();
        type_text(&mut history, &time, &mut state, "b");
        assert_eq!(
            history.restore(false).unwrap().state.body.chars().count(),
            101
        );
    }
    #[test]
    fn title_groups_in_the_same_history() {
        let (mut history, time, mut state) = history();
        for c in "title".chars() {
            state.title.push(c);
            history.record(state.clone(), false);
        }
        type_text(&mut history, &time, &mut state, "body");
        let body = history.restore(false).unwrap();
        assert_eq!(body.state.title, "title");
        assert_eq!(body.state.body, "");
        let title = history.restore(false).unwrap();
        assert!(title.field == Field::Title);
        assert_eq!(title.state.title, "");
        assert_eq!(history.restore(true).unwrap().cursor, 5);
    }
    #[test]
    fn repeated_text_keeps_the_real_changed_range() {
        let (mut history, time, mut state) = history();
        type_text(&mut history, &time, &mut state, "aaaa");
        history.boundary();
        history.note_change(Field::Body, 0);
        state.body.remove(0);
        history.record(state.clone(), false);
        assert_eq!(history.restore(false).unwrap().cursor, 1);
        assert_eq!(history.restore(true).unwrap().cursor, 0);
    }
    #[test]
    fn deleting_across_a_newline_keeps_one_run() {
        let (mut history, time, mut state) = history();
        type_text(&mut history, &time, &mut state, "ab\ncd");
        for _ in 0..5 {
            state.body.pop();
            history.record(state.clone(), false);
        }
        assert_eq!(history.restore(false).unwrap().state.body, "ab\ncd");
        assert_eq!(history.restore(false).unwrap().state.body, "ab\n");
    }
    #[test]
    fn partial_formatting_keeps_the_toggled_range_after_cursor_moves() {
        let mut state = EditState {
            body: "abcdefghijklmnopqrst".into(),
            spans: vec![Span {
                start: 0,
                end: 20,
                mark: Mark::Bold,
            }],
            ..EditState::default()
        };
        let mut history = History::new(state.clone(), || Duration::ZERO);
        state.cursor = 10;
        state.selection = 5;
        state.spans = vec![
            Span {
                start: 0,
                end: 5,
                mark: Mark::Bold,
            },
            Span {
                start: 10,
                end: 20,
                mark: Mark::Bold,
            },
        ];
        history.record(state.clone(), true);
        state.cursor = 20;
        state.selection = 20;
        history.record(state, false);
        let undo = history.restore(false).unwrap();
        assert_eq!((undo.cursor, undo.selection), (10, 5));
        assert_eq!(undo.state.spans[0].end, 20);
        let redo = history.restore(true).unwrap();
        assert_eq!((redo.cursor, redo.selection), (10, 5));
    }
}
