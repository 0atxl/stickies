//! Inline formatting uses Unicode character offsets in the plain-text body.
//! GTK child anchors belong to the view and never enter stored ranges or history.
use crate::note_session::NoteSession;
use gtk::prelude::*;
use serde::{Deserialize, Serialize};
use std::rc::Rc;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Mark {
    Bold,
    Italic,
    Underline,
    Strike,
}
impl Mark {
    pub const ALL: [Self; 4] = [Self::Bold, Self::Italic, Self::Underline, Self::Strike];
    fn name(self) -> &'static str {
        match self {
            Self::Bold => "inline-bold",
            Self::Italic => "inline-italic",
            Self::Underline => "inline-underline",
            Self::Strike => "inline-strike",
        }
    }
    pub fn tag(self, buffer: &gtk::TextBuffer) -> gtk::TextTag {
        buffer.tag_table().lookup(self.name()).unwrap_or_else(|| {
            let builder = gtk::TextTag::builder().name(self.name());
            let tag = match self {
                Self::Bold => builder.weight(700).build(),
                Self::Italic => builder.style(gtk::pango::Style::Italic).build(),
                Self::Underline => builder.underline(gtk::pango::Underline::Single).build(),
                Self::Strike => builder.strikethrough(true).build(),
            };
            buffer.tag_table().add(&tag);
            tag
        })
    }
}
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Span {
    pub start: i32,
    pub end: i32,
    pub mark: Mark,
}

pub fn offset(buffer: &gtk::TextBuffer, at: &gtk::TextIter) -> i32 {
    buffer.text(&buffer.start_iter(), at, true).chars().count() as i32
}
pub fn iter(buffer: &gtk::TextBuffer, offset: i32) -> gtk::TextIter {
    let mut at = buffer.start_iter();
    let mut remaining = offset;
    while !at.is_end() {
        if at.child_anchor().is_none() {
            if remaining <= 0 {
                break;
            }
            remaining -= 1;
        }
        at.forward_char();
    }
    at
}
pub fn spans(buffer: &gtk::TextBuffer) -> Vec<Span> {
    let mut spans = Vec::new();
    for mark in Mark::ALL {
        let Some(tag) = buffer.tag_table().lookup(mark.name()) else {
            continue;
        };
        let mut at = buffer.start_iter();
        let mut position = 0;
        let mut start = None;
        loop {
            if at.child_anchor().is_some() {
                at.forward_char();
                continue;
            }
            let tagged = !at.is_end() && at.has_tag(&tag);
            match (start, tagged) {
                (None, true) => start = Some(position),
                (Some(begin), false) => {
                    spans.push(Span {
                        start: begin,
                        end: position,
                        mark,
                    });
                    start = None;
                }
                _ => {}
            }
            if at.is_end() {
                break;
            }
            at.forward_char();
            position += 1;
        }
    }
    spans
}
pub fn apply(buffer: &gtk::TextBuffer, spans: &[Span]) {
    for mark in Mark::ALL {
        let tag = mark.tag(buffer);
        buffer.remove_tag(&tag, &buffer.start_iter(), &buffer.end_iter());
    }
    for span in spans {
        if span.start >= 0 && span.end > span.start {
            buffer.apply_tag(
                &span.mark.tag(buffer),
                &iter(buffer, span.start),
                &iter(buffer, span.end),
            );
        }
    }
}

/// Keep body history shortcuts attached when All Notes replaces its buffer.
pub fn bind_history(view: &gtk::TextView, session: impl Fn() -> Option<Rc<NoteSession>> + 'static) {
    let keys = gtk::EventControllerKey::new();
    keys.set_propagation_phase(gtk::PropagationPhase::Capture);
    let weak_view = view.downgrade();
    keys.connect_key_pressed(move |_, key, _, modifiers| {
        if key.to_lower() != gtk::gdk::Key::z
            || !modifiers.contains(gtk::gdk::ModifierType::CONTROL_MASK)
            || !weak_view.upgrade().is_some_and(|view| view.is_editable())
        {
            return gtk::glib::Propagation::Proceed;
        }
        let Some(session) = session() else {
            return gtk::glib::Propagation::Proceed;
        };
        session.undo(modifiers.contains(gtk::gdk::ModifierType::SHIFT_MASK));
        gtk::glib::Propagation::Stop
    });
    view.add_controller(keys);
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore = "requires a GTK display; uses a disposable database"]
    fn formatting_survives_unicode_checklists_history_and_reopen() {
        gtk::init().unwrap();
        let root = std::env::temp_dir().join(format!("stickies-formatting-{}", std::process::id()));
        let storage = Rc::new(crate::storage::Storage::open(&root.join("notes.db")).unwrap());
        let note = storage
            .create_note(
                "Format",
                "- [ ] café 東京 👩🏽‍💻",
                crate::app::NoteColor::Yellow,
            )
            .unwrap();
        let id = note.id;
        let session = NoteSession::new(note, storage.clone(), || {});
        let view = gtk::TextView::with_buffer(&session.body);
        session.attach_view(&view);
        let weak_session = Rc::downgrade(&session);
        bind_history(&view, move || weak_session.upgrade());
        let keys = view
            .observe_controllers()
            .iter::<gtk::glib::Object>()
            .flatten()
            .find_map(|controller| controller.downcast::<gtk::EventControllerKey>().ok())
            .unwrap();
        let key =
            |key, modifiers| keys.emit_by_name::<bool>("key-pressed", &[&key, &0u32, &modifiers]);

        session
            .body
            .select_range(&iter(&session.body, 6), &iter(&session.body, 10));
        session.toggle_mark(Mark::Underline);
        let expected = vec![Span {
            start: 6,
            end: 10,
            mark: Mark::Underline,
        }];
        assert_eq!(spans(&session.body), expected);
        for removed in [
            gtk::gdk::Key::b,
            gtk::gdk::Key::i,
            gtk::gdk::Key::u,
            gtk::gdk::Key::f,
            gtk::gdk::Key::y,
        ] {
            assert!(!key(removed, gtk::gdk::ModifierType::CONTROL_MASK));
            assert_eq!(spans(&session.body), expected);
        }

        session.body.insert(&mut iter(&session.body, 8), "é");
        assert_eq!(spans(&session.body)[0].end, 11);
        assert!(key(gtk::gdk::Key::z, gtk::gdk::ModifierType::CONTROL_MASK));
        assert_eq!(spans(&session.body), expected);
        assert!(key(gtk::gdk::Key::z, gtk::gdk::ModifierType::CONTROL_MASK));
        assert!(spans(&session.body).is_empty());
        assert!(key(
            gtk::gdk::Key::z,
            gtk::gdk::ModifierType::CONTROL_MASK | gtk::gdk::ModifierType::SHIFT_MASK
        ));
        assert_eq!(spans(&session.body), expected);
        session.body.place_cursor(&iter(&session.body, 8));
        crate::editor::toggle_line(&view, "- [ ] ");
        assert_eq!(
            spans(&session.body),
            expected,
            "prefix changes preserve formatting"
        );
        assert!(session.flush());
        assert_eq!(storage.load_note(id).unwrap().body, "- [x] café 東京 👩🏽‍💻");
        assert_eq!(storage.load_formatting(id).unwrap(), expected);
        let reopened = NoteSession::new(storage.load_note(id).unwrap(), storage.clone(), || {});
        assert_eq!(spans(&reopened.body), expected);
        // A rejected update must preserve both text and its formatting metadata.
        let db = rusqlite::Connection::open(root.join("notes.db")).unwrap();
        db.execute_batch("CREATE TRIGGER reject BEFORE UPDATE ON notes BEGIN SELECT RAISE(FAIL, 'write rejected'); END;").unwrap();
        session.body.set_text("replacement");
        assert!(!session.flush());
        assert_eq!(storage.load_formatting(id).unwrap(), expected);
        assert!(storage.load_note(id).unwrap().body.starts_with("- [x]"));
        db.execute_batch("DROP TRIGGER reject;").unwrap();
        assert!(session.flush());
        assert!(storage.load_formatting(id).unwrap().is_empty());
        drop(db);
        drop(reopened);
        drop(view);
        drop(session);
        drop(storage);
        std::fs::remove_dir_all(root).unwrap();
    }
}
