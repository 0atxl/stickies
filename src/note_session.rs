use crate::{
    app::{Note, NoteColor, NoteId},
    config::NoteStyle,
    storage::Storage,
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
    error: RefCell<Option<String>>,
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
            error: RefCell::new(error),
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
        session.body.connect_changed(move |buffer| {
            decorate_checklists(buffer);
            if let Some(session) = weak.upgrade() {
                session.schedule_save();
            }
        });
        decorate_checklists(&session.body);
        session
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
        self.error.borrow().clone()
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
                (self.on_saved)();
            }
            Err(error) => {
                self.error
                    .replace(Some(format!("Could not save appearance: {error}")));
            }
        }
        self.notify();
    }

    pub fn set_pinned(&self, pinned: bool) {
        match self.storage.set_note_pinned(self.id(), pinned) {
            Ok(()) => {
                self.pinned.set(pinned);
                (self.on_saved)();
            }
            Err(error) => {
                self.error
                    .replace(Some(format!("Could not change pin: {error}")));
            }
        }
        self.notify();
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
                buffer.apply_tag(&tag, &start, &end);
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
        assert_eq!(first.title.text(), "Shared title");
        assert!(
            first
                .body
                .buffer()
                .start_iter()
                .has_tag(&session.body.tag_table().lookup("completed").unwrap())
        );

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
