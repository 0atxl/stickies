use crate::{
    app::{Note, NoteColor, NoteId},
    config::NoteStyle,
    storage::{Storage, StorageError},
};
use gtk::{glib, prelude::*};
use std::{
    cell::{Cell, RefCell},
    rc::{Rc, Weak},
    time::Duration,
};

/// One live note, shared by every editor displaying it.
pub struct NoteSession {
    pub title: gtk::EntryBuffer,
    pub body: gtk::TextBuffer,
    id: NoteId,
    color: Cell<NoteColor>,
    pinned: Cell<bool>,
    style: RefCell<NoteStyle>,
    storage: Rc<Storage>,
    dirty: Cell<bool>,
    decorating: Cell<bool>,
    render_source: RefCell<Option<glib::SourceId>>,
    views: RefCell<Vec<glib::WeakRef<gtk::TextView>>>,
    error: RefCell<Option<String>>,
    style_error: RefCell<Option<String>>,
    pin_error: RefCell<Option<String>>,
    save_source: RefCell<Option<glib::SourceId>>,
    listeners: RefCell<Vec<Weak<dyn Fn()>>>,
    on_saved: Box<dyn Fn()>,
}

impl NoteSession {
    pub fn new(note: Note, storage: Rc<Storage>, on_saved: impl Fn() + 'static) -> Rc<Self> {
        let title = gtk::EntryBuffer::new(Some(note.title.as_str()));
        let body = gtk::TextBuffer::new(None::<&gtk::TextTagTable>);
        body.set_text(&note.body);
        let (style, error) = match storage.load_note_style(note.id) {
            Ok(style) => (style, None),
            Err(error) => (NoteStyle::default(), Some(error.to_string())),
        };
        let session = Rc::new(Self {
            title,
            body,
            id: note.id,
            color: Cell::new(note.color),
            pinned: Cell::new(note.pinned),
            style: RefCell::new(style),
            storage,
            dirty: Cell::new(false),
            decorating: Cell::new(false),
            render_source: RefCell::new(None),
            views: RefCell::new(Vec::new()),
            error: RefCell::new(None),
            style_error: RefCell::new(error),
            pin_error: RefCell::new(None),
            save_source: RefCell::new(None),
            listeners: RefCell::new(Vec::new()),
            on_saved: Box::new(on_saved),
        });
        let weak = Rc::downgrade(&session);
        session.title.connect_text_notify(move |_| {
            if let Some(session) = weak.upgrade() {
                session.schedule_save();
            }
        });
        let weak = Rc::downgrade(&session);
        session.body.connect_changed(move |_| {
            if let Some(session) = weak.upgrade() {
                if session.decorating.get() {
                    return;
                }
                if session.render_source.borrow().is_none() {
                    let weak = Rc::downgrade(&session);
                    let source = glib::idle_add_local_once(move || {
                        if let Some(session) = weak.upgrade() {
                            session.render_source.borrow_mut().take();
                            session.refresh_checklists();
                            session.notify();
                        }
                    });
                    session.render_source.replace(Some(source));
                }
                session.schedule_save();
            }
        });
        session.refresh_checklists();
        session
    }

    pub fn attach_view(self: &Rc<Self>, view: &gtk::TextView) {
        let mut views = self.views.borrow_mut();
        views.retain(|v| v.upgrade().is_some());
        if !views.iter().any(|v| v.upgrade().as_ref() == Some(view)) {
            views.push(view.downgrade());
        }
        drop(views);
        self.refresh_checklists();
    }

    fn refresh_checklists(self: &Rc<Self>) {
        if self.decorating.replace(true) {
            return;
        }
        decorate_checklists(&self.body);
        let hidden = self
            .body
            .tag_table()
            .lookup("check-marker")
            .unwrap_or_else(|| {
                let tag = gtk::TextTag::builder()
                    .name("check-marker")
                    .invisible(true)
                    .build();
                self.body.tag_table().add(&tag);
                tag
            });
        let (start, end) = self.body.bounds();
        self.body.remove_tag(&hidden, &start, &end);
        // Keep anchors before hidden markers so GTK lays them out correctly.
        // Text markers remain the persisted state; anchors are presentation only.
        for line in 0..self.body.line_count() {
            let Some(start) = self.body.iter_at_line(line) else {
                continue;
            };
            let mut end = start;
            end.forward_to_line_end();
            let text = self.body.text(&start, &end, true);
            let checklist = text.starts_with("- [ ] ") || text.starts_with("- [x] ");
            let start_offset = start.offset();
            let mut scan = end;
            while scan.offset() > start_offset {
                scan.backward_char();
                if scan.child_anchor().is_some() && (!checklist || scan.offset() != start_offset) {
                    let mut after = scan;
                    after.forward_char();
                    self.body.delete(&mut scan, &mut after);
                }
            }
            if !checklist {
                continue;
            }
            let mut start = self.body.iter_at_line(line).unwrap();
            let anchor = start
                .child_anchor()
                .unwrap_or_else(|| self.body.create_child_anchor(&mut start));
            let mut marker_start = self.body.iter_at_child_anchor(&anchor);
            marker_start.forward_char();
            let mut marker_end = marker_start;
            marker_end.forward_chars(6);
            self.body.apply_tag(&hidden, &marker_start, &marker_end);
            let active = text.starts_with("- [x] ");
            for view in self
                .views
                .borrow()
                .iter()
                .filter_map(|v| v.upgrade())
                .filter(|v| v.buffer() == self.body)
            {
                let existing = anchor.widgets().into_iter().find(|w| {
                    w.ancestor(gtk::TextView::static_type()).as_ref() == Some(view.upcast_ref())
                });
                if let Some(button) = existing.and_then(|w| w.downcast::<gtk::CheckButton>().ok()) {
                    button.set_active(active);
                    continue;
                }
                let button = gtk::CheckButton::new();
                button.add_css_class("inline-check");
                button.set_active(active);
                button.set_tooltip_text(Some("Mark item complete"));
                let weak = Rc::downgrade(self);
                let target = anchor.clone();
                button.connect_toggled(move |b| {
                    let Some(session) = weak.upgrade() else {
                        return;
                    };
                    if session.decorating.get() || target.is_deleted() {
                        return;
                    }
                    let mut at = session.body.iter_at_child_anchor(&target);
                    at.forward_chars(4);
                    let mut end = at;
                    end.forward_char();
                    session.body.begin_user_action();
                    session.decorating.set(true);
                    session.body.delete(&mut at, &mut end);
                    session
                        .body
                        .insert(&mut at, if b.is_active() { "x" } else { " " });
                    session.decorating.set(false);
                    session.body.end_user_action();
                    session.refresh_checklists();
                    session.schedule_save();
                });
                view.add_child_at_anchor(&button, &anchor);
            }
        }
        self.decorating.set(false);
    }

    pub fn id(&self) -> NoteId {
        self.id
    }
    pub fn color(&self) -> NoteColor {
        self.color.get()
    }
    pub fn pinned(&self) -> bool {
        self.pinned.get()
    }
    pub fn style(&self) -> NoteStyle {
        self.style.borrow().clone()
    }
    pub fn error(&self) -> Option<String> {
        self.error
            .borrow()
            .clone()
            .or_else(|| self.style_error.borrow().clone())
            .or_else(|| self.pin_error.borrow().clone())
    }
    pub fn is_dirty(&self) -> bool {
        self.dirty.get()
    }

    /// Views retain the returned subscription; dead views are not kept alive.
    pub fn subscribe(&self, callback: impl Fn() + 'static) -> Rc<dyn Fn()> {
        let callback: Rc<dyn Fn()> = Rc::new(callback);
        self.listeners.borrow_mut().push(Rc::downgrade(&callback));
        callback
    }

    fn notify(&self) {
        let callbacks: Vec<_> = {
            let mut listeners = self.listeners.borrow_mut();
            listeners.retain(|listener| listener.strong_count() > 0);
            listeners.iter().filter_map(Weak::upgrade).collect()
        };
        for callback in callbacks {
            callback();
        }
    }

    pub fn set_style(&self, style: NoteStyle) {
        match self.storage.save_note_style(self.id(), &style) {
            Ok(()) => {
                self.style.replace(style);
                self.style_error.borrow_mut().take();
                (self.on_saved)();
            }
            Err(error) => {
                self.style_error
                    .replace(Some(format!("Could not save appearance: {error}")));
            }
        }
        self.notify();
    }

    pub fn set_pinned(&self, pinned: bool) -> Result<(), StorageError> {
        let result = self.storage.set_note_pinned(self.id(), pinned);
        match &result {
            Ok(()) => {
                self.pinned.set(pinned);
                self.pin_error.borrow_mut().take();
                (self.on_saved)();
            }
            Err(error) => {
                self.pin_error
                    .replace(Some(format!("Could not change pin: {error}")));
            }
        }
        self.notify();
        result
    }

    pub fn sync_pin(&self) {
        if let Ok(note) = self.storage.load_note(self.id()) {
            self.pinned.set(note.pinned);
            self.notify();
        }
    }

    pub fn set_color(self: &Rc<Self>, color: NoteColor) {
        self.color.set(color);
        self.schedule_save();
    }

    fn schedule_save(self: &Rc<Self>) {
        self.dirty.set(true);
        if let Some(source) = self.save_source.borrow_mut().take() {
            source.remove();
        }
        let weak = Rc::downgrade(self);
        self.save_source.replace(Some(glib::timeout_add_local_once(
            Duration::from_millis(750),
            move || {
                if let Some(session) = weak.upgrade() {
                    session.save_source.borrow_mut().take();
                    session.flush();
                }
            },
        )));
        self.notify();
    }

    pub fn flush(&self) -> bool {
        if let Some(source) = self.save_source.borrow_mut().take() {
            source.remove();
        }
        if !self.dirty.get() {
            return true;
        }
        let (start, end) = self.body.bounds();
        let note = Note {
            id: self.id,
            title: self.title.text().to_string(),
            body: self.body.text(&start, &end, true).to_string(),
            color: self.color.get(),
            pinned: self.pinned.get(),
        };
        match self.storage.update_note(&note) {
            Ok(()) => {
                self.dirty.set(false);
                self.error.borrow_mut().take();
                (self.on_saved)();
                self.notify();
                true
            }
            Err(error) => {
                self.error.replace(Some(format!(
                    "Could not save. Keep this note open and retry. {error}"
                )));
                self.notify();
                false
            }
        }
    }
}

impl Drop for NoteSession {
    fn drop(&mut self) {
        if let Some(source) = self.render_source.get_mut().take() {
            source.remove();
        }
        if let Some(source) = self.save_source.get_mut().take() {
            source.remove();
        }
    }
}

pub fn decorate_checklists(buffer: &gtk::TextBuffer) {
    let tag = buffer.tag_table().lookup("completed").unwrap_or_else(|| {
        let tag = gtk::TextTag::builder()
            .name("completed")
            .strikethrough(true)
            .build();
        buffer.tag_table().add(&tag);
        tag
    });
    let (start, end) = buffer.bounds();
    buffer.remove_tag(&tag, &start, &end);
    for line in 0..buffer.line_count() {
        if let Some(start) = buffer.iter_at_line(line) {
            let mut end = start;
            end.forward_to_line_end();
            if buffer.text(&start, &end, true).starts_with("- [x] ") {
                let mut content = start;
                if content.child_anchor().is_some() {
                    content.forward_char();
                }
                content.forward_chars(6);
                buffer.apply_tag(&tag, &content, &end);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "requires a GTK display; uses a disposable database"]
    fn shared_editors_keep_one_save_owner_and_retain_failed_edits() {
        crate::ui_assets::register().unwrap();
        gtk::init().unwrap();
        let path = std::env::temp_dir().join(format!(
            "stickies-session-{}-{}.sqlite",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let storage = Rc::new(Storage::open(&path).unwrap());
        let note = storage
            .create_note("Title", "Original", NoteColor::Yellow)
            .unwrap();
        let id = note.id;
        let saves = Rc::new(Cell::new(0));
        let count = saves.clone();
        let session = NoteSession::new(note, storage.clone(), move || count.set(count.get() + 1));
        let config = Rc::new(RefCell::new(crate::config::Config::default()));
        let first = crate::editor::Editor::new(session.clone(), config.clone(), || {});
        let second = crate::editor::Editor::new(session.clone(), config, || {});
        assert_eq!(first.body.buffer(), second.body.buffer());
        assert_eq!(first.title.buffer(), second.title.buffer());
        first.title.set_text("Shared title");
        second.body.buffer().set_text("- [x] Shared body");
        while glib::MainContext::default().iteration(false) {}
        assert_eq!(first.title.text(), "Shared title");
        assert!(
            first
                .body
                .buffer()
                .iter_at_offset(7)
                .has_tag(&session.body.tag_table().lookup("completed").unwrap())
        );

        let anchor = session.body.iter_at_offset(0).child_anchor().unwrap();
        let checks: Vec<gtk::CheckButton> = anchor
            .widgets()
            .into_iter()
            .filter_map(|w| w.downcast().ok())
            .collect();
        assert_eq!(checks.len(), 2);
        checks[0].set_active(false);
        assert!(checks.iter().all(|b| !b.is_active()));
        checks[1].set_active(true);
        assert!(checks.iter().all(|b| b.is_active()));
        assert_eq!(
            session
                .body
                .text(&session.body.start_iter(), &session.body.end_iter(), true),
            "- [x] Shared body"
        );

        let window = gtk::Window::builder()
            .child(&second.root)
            .resizable(false)
            .build();
        window.present();
        let layout = glib::MainLoop::new(None, false);
        let finish = layout.clone();
        glib::timeout_add_local_once(Duration::from_millis(100), move || finish.quit());
        layout.run();
        let check = checks
            .iter()
            .find(|check| {
                check.ancestor(gtk::TextView::static_type()).as_ref()
                    == Some(second.body.upcast_ref())
            })
            .unwrap();
        let bounds = check.compute_bounds(&second.body).unwrap();
        assert!(
            bounds.x() >= 0. && bounds.y() >= 0.,
            "first checkbox must be laid out inside the view: {bounds:?}"
        );

        fn swatch(widget: &gtk::Widget, label: &str) -> Option<gtk::ToggleButton> {
            if widget.tooltip_text().as_deref() == Some(label) {
                return widget.clone().downcast().ok();
            }
            let mut child = widget.first_child();
            while let Some(widget) = child {
                if let Some(button) = swatch(&widget, label) {
                    return Some(button);
                }
                child = widget.next_sibling();
            }
            None
        }
        swatch(first.root.upcast_ref(), "Blue")
            .unwrap()
            .emit_clicked();
        assert!(
            swatch(second.root.upcast_ref(), "Blue")
                .unwrap()
                .is_active()
        );
        assert!(
            !swatch(second.root.upcast_ref(), "Amber")
                .unwrap()
                .is_active()
        );

        for color in NoteColor::ALL {
            swatch(second.root.upcast_ref(), color.label())
                .unwrap()
                .emit_clicked();
            assert_eq!(session.color(), color);
            assert!(
                swatch(first.root.upcast_ref(), color.label())
                    .unwrap()
                    .is_active()
            );
            assert!(session.flush());
            assert_eq!(
                Storage::open(&path).unwrap().load_note(id).unwrap().color,
                color
            );
        }
        // Leave one pending color edit for the closing-view autosave check.
        swatch(second.root.upcast_ref(), "Blue")
            .unwrap()
            .emit_clicked();

        first.keep.set_active(true);
        assert!(!second.keep.is_active(), "Always open belongs to one view");
        assert!(!session.pinned(), "individual Pin must not change ordering");
        assert!(!storage.load_note(id).unwrap().pinned);
        first.keep.set_active(false);

        swatch(first.root.upcast_ref(), "Large text")
            .unwrap()
            .emit_clicked();
        swatch(first.root.upcast_ref(), "Bold text")
            .unwrap()
            .emit_clicked();
        let text_style = session.style();
        assert_eq!(text_style.text_size, crate::config::NoteTextSize::Large);
        assert_eq!(text_style.text_weight, crate::config::NoteTextWeight::Bold);
        assert_eq!(storage.load_note_style(id).unwrap(), text_style);
        assert!(
            swatch(second.root.upcast_ref(), "Large text")
                .unwrap()
                .is_active()
        );
        assert!(
            swatch(second.root.upcast_ref(), "Bold text")
                .unwrap()
                .is_active()
        );
        assert!(
            !swatch(second.root.upcast_ref(), "Medium text")
                .unwrap()
                .is_active()
        );
        assert!(
            !swatch(second.root.upcast_ref(), "Regular text")
                .unwrap()
                .is_active()
        );

        // Count content saves separately from these immediate style writes.
        saves.set(0);
        // A closing view must not cancel the other view's pending autosave.
        first.stop();
        drop(first);
        let main_loop = glib::MainLoop::new(None, false);
        let finish = main_loop.clone();
        glib::timeout_add_local_once(Duration::from_millis(850), move || finish.quit());
        main_loop.run();
        assert_eq!(saves.get(), 1);
        let saved = storage.load_note(id).unwrap();
        assert_eq!(saved.title, "Shared title");
        assert_eq!(saved.body, "- [x] Shared body");
        let reopened = NoteSession::new(saved, storage.clone(), || {});
        assert_eq!(reopened.style(), text_style);
        assert_eq!(
            Storage::open(&path).unwrap().load_note_style(id).unwrap(),
            text_style
        );
        let reopened_view = gtk::TextView::with_buffer(&reopened.body);
        reopened.attach_view(&reopened_view);
        let reopened_check = reopened
            .body
            .iter_at_offset(0)
            .child_anchor()
            .unwrap()
            .widgets()[0]
            .clone()
            .downcast::<gtk::CheckButton>()
            .unwrap();
        assert!(reopened_check.is_active());
        drop(reopened_view);
        drop(reopened);

        let connection = rusqlite::Connection::open(&path).unwrap();
        connection.execute_batch("CREATE TRIGGER reject_edit BEFORE UPDATE ON notes BEGIN SELECT RAISE(FAIL, 'test write failure'); END;").unwrap();
        second.body.buffer().set_text("Must survive failure");
        assert!(!second.flush());
        assert!(session.dirty.get());
        assert!(session.error().is_some());
        assert_eq!(storage.load_note(id).unwrap().body, "- [x] Shared body");
        connection
            .execute_batch("DROP TRIGGER reject_edit;")
            .unwrap();
        assert!(second.flush());
        assert!(!session.dirty.get());
        assert!(session.error().is_none());
        assert_eq!(storage.load_note(id).unwrap().body, "Must survive failure");
        assert_eq!(saves.get(), 2);

        // Ordinary editing must preserve the hidden marker and remove it when
        // backspacing over the checkbox at the start of the visible line.
        session.body.set_text("- [ ] AB");
        while glib::MainContext::default().iteration(false) {}
        session.body.place_cursor(&session.body.iter_at_offset(8));
        second.body.emit_by_name::<()>("backspace", &[]);
        while glib::MainContext::default().iteration(false) {}
        assert_eq!(
            session
                .body
                .text(&session.body.start_iter(), &session.body.end_iter(), true),
            "- [ ] B"
        );
        session.body.place_cursor(&session.body.iter_at_offset(7));
        second.body.emit_by_name::<()>("backspace", &[]);
        while glib::MainContext::default().iteration(false) {}
        assert_eq!(
            session
                .body
                .text(&session.body.start_iter(), &session.body.end_iter(), true),
            "B"
        );

        session.body.set_text("- [ ] First\n- [x] Second");
        while glib::MainContext::default().iteration(false) {}
        session
            .body
            .select_range(&session.body.start_iter(), &session.body.end_iter());
        session.body.delete_selection(true, true);
        session.body.insert_at_cursor("- [ ] Replacement");
        while glib::MainContext::default().iteration(false) {}
        assert_eq!(
            session
                .body
                .iter_at_offset(0)
                .child_anchor()
                .unwrap()
                .widgets()
                .len(),
            1
        );
        assert!(session.flush());
        assert_eq!(storage.load_note(id).unwrap().body, "- [ ] Replacement");

        connection.execute_batch("CREATE TRIGGER reject_pin BEFORE UPDATE ON notes BEGIN SELECT RAISE(FAIL, 'pin failure'); END;").unwrap();
        assert!(session.set_pinned(true).is_err());
        assert!(session.error().unwrap().contains("Could not change pin"));
        session.body.insert_at_cursor("!");
        assert!(!session.flush());
        connection
            .execute_batch("DROP TRIGGER reject_pin;")
            .unwrap();
        session.set_pinned(true).unwrap();
        assert!(
            session.error().unwrap().contains("Keep this note open"),
            "pin retry must not hide body-save failure"
        );
        assert!(session.flush());
        assert!(session.error().is_none());

        connection.execute_batch("CREATE TRIGGER reject_style BEFORE INSERT ON note_styles BEGIN SELECT RAISE(FAIL, 'style failure'); END;").unwrap();
        session.set_style(NoteStyle::default());
        assert!(
            session
                .error()
                .unwrap()
                .contains("Could not save appearance")
        );
        assert_eq!(
            session.style(),
            text_style,
            "failed style writes retain the previous style"
        );
        swatch(second.root.upcast_ref(), "Small text")
            .unwrap()
            .emit_clicked();
        assert!(
            swatch(second.root.upcast_ref(), "Large text")
                .unwrap()
                .is_active()
        );
        connection
            .execute_batch("DROP TRIGGER reject_style;")
            .unwrap();
        session.set_style(NoteStyle::default());
        assert!(session.error().is_none());

        let other = storage
            .create_note("Other", "- [ ] Other", NoteColor::Yellow)
            .unwrap();
        let other = NoteSession::new(other, storage.clone(), || {});
        second.body.set_buffer(Some(&other.body));
        other.attach_view(&second.body);
        second.body.set_buffer(Some(&session.body));
        session.attach_view(&second.body);
        assert_eq!(
            session
                .body
                .start_iter()
                .child_anchor()
                .unwrap()
                .widgets()
                .len(),
            1
        );
        drop(other);
        window.set_child(None::<&gtk::Widget>);
        window.close();
        second.stop();
        drop(second);
        let weak = Rc::downgrade(&session);
        drop(session);
        assert!(
            weak.upgrade().is_none(),
            "subscriptions must not retain closed views"
        );
        drop(connection);
        drop(storage);
        std::fs::remove_file(path).unwrap();
    }
}
