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
        if !modifiers.contains(gtk::gdk::ModifierType::CONTROL_MASK)
            || !weak_view.upgrade().is_some_and(|view| view.is_editable())
        {
            return gtk::glib::Propagation::Proceed;
        }
        let Some(session) = session() else {
            return gtk::glib::Propagation::Proceed;
        };
        match key.to_lower() {
            gtk::gdk::Key::z => {
                session.undo(modifiers.contains(gtk::gdk::ModifierType::SHIFT_MASK))
            }
            gtk::gdk::Key::y => session.undo(true),
            gtk::gdk::Key::b => session.toggle_mark(Mark::Bold),
            gtk::gdk::Key::i => session.toggle_mark(Mark::Italic),
            gtk::gdk::Key::u => session.toggle_mark(Mark::Underline),
            _ => return gtk::glib::Propagation::Proceed,
        }
        gtk::glib::Propagation::Stop
    });
    view.add_controller(keys);
}

pub fn copy_text(body: &gtk::TextView) {
    let buffer = body.buffer();
    let (start, end) = buffer.bounds();
    body.clipboard().set_text(&buffer.text(&start, &end, true));
}

/// A nonmodal popover follows the body's selection and releases old buffer signals.
pub struct SelectionTools {
    pub popover: gtk::Popover,
    update: Rc<dyn Fn()>,
    dismissed: Rc<std::cell::Cell<bool>>,
    host: gtk::glib::WeakRef<gtk::Overlay>,
}
impl SelectionTools {
    pub fn new(
        view: &gtk::TextView,
        host: &gtk::Overlay,
        session: impl Fn() -> Option<Rc<NoteSession>> + 'static,
        footer: &[gtk::Popover],
    ) -> Self {
        let session: Rc<dyn Fn() -> Option<Rc<NoteSession>>> = Rc::new(session);
        let popover = gtk::Popover::new();
        popover.add_css_class("selection-tools");
        popover.set_autohide(false);
        popover.set_has_arrow(false);
        popover.set_position(gtk::PositionType::Top);
        popover.set_halign(gtk::Align::Start);
        popover.set_valign(gtk::Align::Start);
        host.add_overlay(&popover);
        host.set_measure_overlay(&popover, false);
        let row = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        for (mark, label, tooltip) in [
            (Mark::Bold, "B", "Bold"),
            (Mark::Italic, "I", "Italic"),
            (Mark::Underline, "U", "Underline"),
            (Mark::Strike, "S", "Strikethrough"),
        ] {
            let button = gtk::Button::with_label(label);
            button.set_tooltip_text(Some(tooltip));
            button.set_focus_on_click(false);
            let session = session.clone();
            button.connect_clicked(move |_| {
                if let Some(session) = session() {
                    session.toggle_mark(mark);
                }
            });
            row.append(&button);
        }
        popover.set_child(Some(&row));
        let weak_view = view.downgrade();
        let weak_host = host.downgrade();
        let weak_popup = popover.downgrade();
        let panels: Vec<_> = footer.iter().map(|p| p.downgrade()).collect();
        let dismissed = Rc::new(std::cell::Cell::new(false));
        let suppressed = dismissed.clone();
        let update: Rc<dyn Fn()> = Rc::new(move || {
            let (Some(view), Some(popup), Some(host)) = (
                weak_view.upgrade(),
                weak_popup.upgrade(),
                weak_host.upgrade(),
            ) else {
                return;
            };
            let selection = view.buffer().selection_bounds();
            if suppressed.get()
                || (!view.has_focus()
                    && !popup.state_flags().contains(gtk::StateFlags::FOCUS_WITHIN))
                || !view.is_mapped()
                || !view.is_editable()
                || selection.is_none()
                || panels
                    .iter()
                    .any(|p| p.upgrade().is_some_and(|p| p.is_visible()))
            {
                popup.popdown();
                return;
            }
            let (start, _) = selection.unwrap();
            let rect = view.iter_location(&start);
            let visible = view.visible_rect();
            let (x, y) = view.buffer_to_window_coords(
                gtk::TextWindowType::Widget,
                rect.x()
                    .clamp(visible.x(), visible.x() + visible.width().max(1) - 1),
                rect.y()
                    .clamp(visible.y(), visible.y() + visible.height().max(1) - 1),
            );
            let Some(point) =
                view.compute_point(&host, &gtk::graphene::Point::new(x as f32, y as f32))
            else {
                return;
            };
            let (x, y) = (point.x().round() as i32, point.y().round() as i32);
            popup.set_pointing_to(Some(&gtk::gdk::Rectangle::new(
                x,
                y,
                rect.width().max(1),
                rect.height().max(1),
            )));
            if !popup.is_visible() {
                popup.popup();
            }
        });
        let handler = Rc::new(std::cell::RefCell::new(
            None::<(gtk::TextBuffer, gtk::glib::SignalHandlerId)>,
        ));
        let bind = {
            let handler = handler.clone();
            let update = update.clone();
            let dismissed = dismissed.clone();
            move |view: &gtk::TextView| {
                if let Some((buffer, id)) = handler.borrow_mut().take() {
                    buffer.disconnect(id);
                }
                let buffer = view.buffer();
                let refresh = update.clone();
                let dismissed = dismissed.clone();
                let id = buffer.connect_mark_set(move |_, _, _| {
                    dismissed.set(false);
                    refresh();
                });
                handler.replace(Some((buffer, id)));
                update();
            }
        };
        bind(view);
        view.connect_buffer_notify(move |view| bind(view));
        let refresh = update.clone();
        view.connect_map(move |_| refresh());
        let weak = popover.downgrade();
        view.connect_unmap(move |_| {
            if let Some(p) = weak.upgrade() {
                p.popdown();
            }
        });
        let refresh = update.clone();
        view.connect_editable_notify(move |_| refresh());
        let refresh = update.clone();
        view.connect_has_focus_notify(move |_| refresh());

        for adjustment in [view.hadjustment(), view.vadjustment()]
            .into_iter()
            .flatten()
        {
            let refresh = update.clone();
            adjustment.connect_value_changed(move |_| refresh());
        }
        for panel in footer {
            let refresh = update.clone();
            panel.connect_visible_notify(move |_| refresh());
        }
        Self {
            popover,
            update,
            dismissed,
            host: host.downgrade(),
        }
    }
    pub fn refresh(&self) {
        (self.update)();
    }
    pub fn dismiss(&self) -> bool {
        let visible = self.popover.is_visible();
        self.dismissed.set(true);
        self.popover.popdown();
        visible
    }
}
impl Drop for SelectionTools {
    fn drop(&mut self) {
        self.popover.set_visible(false);
        if let Some(host) = self.host.upgrade()
            && self.popover.parent().is_some()
        {
            host.remove_overlay(&self.popover);
        }
    }
}

#[cfg(test)]
pub(crate) fn capture_native(name: &str) {
    if let Ok(path) = std::env::var("STICKIES_NATIVE_CAPTURES") {
        let path = std::path::Path::new(&path);
        std::fs::create_dir_all(path).unwrap();
        assert!(
            std::process::Command::new("grim")
                .arg(path.join(format!("{name}.png")))
                .status()
                .unwrap()
                .success()
        );
    }
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
        assert!(!key(gtk::gdk::Key::f, gtk::gdk::ModifierType::CONTROL_MASK));

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
        session.body.set_text("Keyboard formatting");
        session
            .body
            .select_range(&iter(&session.body, 0), &iter(&session.body, 8));
        for key_name in [gtk::gdk::Key::b, gtk::gdk::Key::i, gtk::gdk::Key::u] {
            assert!(key(key_name, gtk::gdk::ModifierType::CONTROL_MASK));
        }
        assert_eq!(spans(&session.body).len(), 3);
        assert!(key(gtk::gdk::Key::z, gtk::gdk::ModifierType::CONTROL_MASK));
        assert_eq!(spans(&session.body).len(), 2);
        assert!(key(gtk::gdk::Key::y, gtk::gdk::ModifierType::CONTROL_MASK));
        assert_eq!(spans(&session.body).len(), 3);
        assert!(session.flush());
        drop(db);
        drop(reopened);
        drop(view);
        drop(session);
        drop(storage);
        std::fs::remove_dir_all(root).unwrap();
    }
}
