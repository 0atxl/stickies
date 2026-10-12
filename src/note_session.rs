use crate::{
    app::{Note, NoteColor, NoteId},
    config::NoteStyle,
    edit_history::{EditState, Field, History},
    storage::{Storage, StorageError},
};
use gtk::{glib, prelude::*};
use std::{
    cell::{Cell, RefCell},
    rc::{Rc, Weak},
    time::{Duration, Instant},
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
    new_blank: Cell<bool>,
    decorating: Cell<bool>,
    render_source: RefCell<Option<glib::SourceId>>,
    views: RefCell<Vec<glib::WeakRef<gtk::TextView>>>,
    title_views: RefCell<Vec<glib::WeakRef<gtk::Entry>>>,
    error: RefCell<Option<String>>,
    style_error: RefCell<Option<String>>,
    pin_error: RefCell<Option<String>>,
    save_source: RefCell<Option<glib::SourceId>>,
    listeners: RefCell<Vec<Weak<dyn Fn()>>>,
    on_saved: Rc<dyn Fn()>,
    history: RefCell<History>,
    edit_depth: Cell<usize>,
    mutation_depth: Cell<usize>,
    isolated: Cell<bool>,
    formatting_error: Option<String>,
}

impl NoteSession {
    pub fn new(note: Note, storage: Rc<Storage>, on_saved: impl Fn() + 'static) -> Rc<Self> {
        let start = Instant::now();
        Self::new_with_clock(note, storage, on_saved, move || start.elapsed())
    }
    pub fn new_with_clock(
        note: Note,
        storage: Rc<Storage>,
        on_saved: impl Fn() + 'static,
        clock: impl Fn() -> Duration + 'static,
    ) -> Rc<Self> {
        let title = gtk::EntryBuffer::new(Some(note.title.as_str()));
        let body = gtk::TextBuffer::new(None::<&gtk::TextTagTable>);
        body.set_enable_undo(false);
        body.set_text(&note.body);
        let (style, error) = match storage.load_note_style(note.id) {
            Ok(style) => (style, None),
            Err(error) => (NoteStyle::default(), Some(error.to_string())),
        };
        let formatting_error = match storage.load_formatting(note.id) {
            Ok(spans) => {
                crate::text_editing::apply(&body, &spans);
                None
            }
            Err(error) => Some(format!("Could not load text formatting: {error}")),
        };
        let initial = EditState {
            title: note.title,
            body: note.body,
            spans: crate::text_editing::spans(&body),
            cursor: body.cursor_position(),
            selection: body.cursor_position(),
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
            new_blank: Cell::new(false),
            decorating: Cell::new(false),
            render_source: RefCell::new(None),
            views: RefCell::new(Vec::new()),
            title_views: RefCell::new(Vec::new()),
            error: RefCell::new(None),
            style_error: RefCell::new(error),
            pin_error: RefCell::new(None),
            save_source: RefCell::new(None),
            listeners: RefCell::new(Vec::new()),
            on_saved: Rc::new(on_saved),
            history: RefCell::new(History::new(initial, clock)),
            edit_depth: Cell::new(0),
            mutation_depth: Cell::new(0),
            isolated: Cell::new(false),
            formatting_error,
        });
        let weak = Rc::downgrade(&session);
        session.title.connect_text_notify(move |_| {
            if let Some(session) = weak.upgrade()
                && !session.decorating.get()
            {
                if session.isolated.get() {
                    session.queue_record();
                } else {
                    session.record_edit();
                }
                session.schedule_save();
            }
        });
        let weak = Rc::downgrade(&session);
        session.body.connect_changed(move |_| {
            if let Some(session) = weak.upgrade() {
                if session.decorating.get() {
                    return;
                }
                session.queue_record();
                session.schedule_save();
            }
        });
        for signal in ["insert-text", "delete-range"] {
            let weak = Rc::downgrade(&session);
            session.body.connect_local(signal, false, move |values| {
                if let Some(session) = weak.upgrade()
                    && !session.decorating.get()
                {
                    let start = values[1].get::<gtk::TextIter>().unwrap();
                    if signal == "delete-range" {
                        let end = values[2].get::<gtk::TextIter>().unwrap();
                        let removed = session.body.text(&start, &end, true);
                        if start.starts_line() && matches!(removed.as_str(), "- [ ] " | "- [x] ") {
                            session.isolate_next_edit();
                        }
                    }
                    let offset = crate::text_editing::offset(&session.body, &start);
                    session.note_change(Field::Body, offset);
                    session.mutation_depth.set(session.mutation_depth.get() + 1);
                }
                None
            });
            let weak = Rc::downgrade(&session);
            session.body.connect_local(signal, true, move |_| {
                if let Some(session) = weak.upgrade()
                    && !session.decorating.get()
                {
                    session.mutation_depth.set(session.mutation_depth.get() - 1);
                    session.record_edit();
                }
                None
            });
        }
        let weak = Rc::downgrade(&session);
        session.body.connect_begin_user_action(move |_| {
            if let Some(session) = weak.upgrade()
                && !session.decorating.get()
            {
                session.edit_depth.set(session.edit_depth.get() + 1);
            }
        });
        let weak = Rc::downgrade(&session);
        session.body.connect_end_user_action(move |_| {
            if let Some(session) = weak.upgrade()
                && !session.decorating.get()
            {
                session.edit_depth.set(session.edit_depth.get() - 1);
                session.record_edit();
            }
        });
        let weak = Rc::downgrade(&session);
        session.body.connect_mark_set(move |buffer, _, mark| {
            if let Some(session) = weak.upgrade()
                && session.mutation_depth.get() == 0
                && session.edit_depth.get() == 0
                && (mark == &buffer.get_insert() || mark == &buffer.selection_bound())
            {
                session.break_typing();
            }
        });
        let weak = Rc::downgrade(&session);
        session.body.connect_paste_done(move |_, _| {
            if let Some(session) = weak.upgrade() {
                session.finish_isolated_edit();
            }
        });
        session.refresh_checklists();
        session.record_edit();
        session
    }

    fn queue_record(self: &Rc<Self>) {
        if self.render_source.borrow().is_none() {
            let weak = Rc::downgrade(self);
            let source = glib::idle_add_local_once(move || {
                if let Some(session) = weak.upgrade() {
                    session.render_source.borrow_mut().take();
                    session.refresh_checklists();
                    session.record_edit();
                    session.notify();
                }
            });
            self.render_source.replace(Some(source));
        }
    }

    fn edit_state(&self) -> EditState {
        EditState {
            title: self.title.text().to_string(),
            body: self
                .body
                .text(&self.body.start_iter(), &self.body.end_iter(), true)
                .to_string(),
            spans: crate::text_editing::spans(&self.body),
            cursor: crate::text_editing::offset(
                &self.body,
                &self.body.iter_at_mark(&self.body.get_insert()),
            ),
            selection: crate::text_editing::offset(
                &self.body,
                &self.body.iter_at_mark(&self.body.selection_bound()),
            ),
        }
    }
    fn record_edit(&self) {
        if self.decorating.get() || self.edit_depth.get() > 0 || self.mutation_depth.get() > 0 {
            return;
        }
        let state = self.edit_state();
        if self.history.borrow_mut().record(state, self.isolated.get()) {
            self.isolated.set(false);
        }
    }
    pub fn break_typing(&self) {
        if !self.decorating.get() {
            self.record_edit();
            self.history.borrow_mut().boundary();
        }
    }
    pub fn note_change(&self, field: Field, start: i32) {
        if !self.decorating.get() {
            self.history
                .borrow_mut()
                .note_change(field, start.max(0) as usize);
        }
    }
    pub fn isolate_next_edit(&self) {
        self.break_typing();
        self.isolated.set(true);
    }
    pub fn finish_isolated_edit(&self) {
        self.record_edit();
        self.isolated.set(false);
        self.history.borrow_mut().boundary();
    }
    pub fn edit_action<T>(&self, action: impl FnOnce() -> T) -> T {
        self.isolate_next_edit();
        self.edit_depth.set(self.edit_depth.get() + 1);
        let result = action();
        self.edit_depth.set(self.edit_depth.get() - 1);
        if self.edit_depth.get() == 0 {
            self.finish_isolated_edit();
        }
        result
    }
    pub fn undo(self: &Rc<Self>, redo: bool) {
        self.record_edit();
        let Some(restored) = self.history.borrow_mut().restore(redo) else {
            return;
        };
        let state = restored.state;
        self.decorating.set(true);
        self.title.set_text(&state.title);
        self.body.set_text(&state.body);
        crate::text_editing::apply(&self.body, &state.spans);
        self.decorating.set(false);
        self.refresh_checklists();
        let (cursor, selection) = if restored.field == Field::Body {
            (restored.cursor, restored.selection)
        } else {
            (state.cursor, state.selection)
        };
        self.body.select_range(
            &crate::text_editing::iter(&self.body, cursor),
            &crate::text_editing::iter(&self.body, selection),
        );
        if restored.field == Field::Title {
            for view in self.title_views.borrow().iter().filter_map(|v| v.upgrade()) {
                if view.buffer() == self.title {
                    view.set_position(restored.cursor);
                }
            }
        }
        self.schedule_save();
    }
    pub fn toggle_mark(self: &Rc<Self>, mark: crate::text_editing::Mark) {
        let Some((start, end)) = self.body.selection_bounds() else {
            return;
        };
        self.isolate_next_edit();
        let tag = mark.tag(&self.body);
        let mut at = start;
        let mut all = true;
        while at < end {
            if at.child_anchor().is_none() && !at.has_tag(&tag) {
                all = false;
                break;
            }
            at.forward_char();
        }
        if all {
            self.body.remove_tag(&tag, &start, &end);
        } else {
            self.body.apply_tag(&tag, &start, &end);
        }
        self.finish_isolated_edit();
        self.schedule_save();
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
    pub fn attach_title(&self, view: &gtk::Entry) {
        let mut views = self.title_views.borrow_mut();
        views.retain(|v| v.upgrade().is_some());
        if !views.iter().any(|v| v.upgrade().as_ref() == Some(view)) {
            views.push(view.downgrade());
        }
    }

    pub fn mark_new_blank(&self) {
        self.new_blank.set(self.is_blank());
    }

    fn is_blank(&self) -> bool {
        let (start, end) = self.body.bounds();
        self.title.text().trim().is_empty() && self.body.text(&start, &end, true).trim().is_empty()
    }

    // Release only this view. A new blank remains editable in any other window.
    pub fn release_view(&self, view: &gtk::TextView) -> bool {
        if !self.flush() {
            return false;
        }
        let other_view = self
            .views
            .borrow()
            .iter()
            .filter_map(|v| v.upgrade())
            .any(|other| other != *view && other.buffer() == self.body);
        if self.new_blank.get() && !other_view {
            if self.is_blank() && self.storage.is_archived(self.id()).ok() == Some(false) {
                if let Err(error) = self.storage.delete_note(self.id()) {
                    self.error.replace(Some(format!(
                        "Could not remove the empty note. Retry closing it. {error}"
                    )));
                    self.notify();
                    return false;
                }
                // Refresh after the closing/switching view has released its selection.
                let changed = self.on_saved.clone();
                glib::idle_add_local_once(move || changed());
            }
            self.new_blank.set(false);
        }
        self.views
            .borrow_mut()
            .retain(|v| v.upgrade().is_some_and(|other| other != *view));
        true
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
                button.update_property(&[gtk::accessible::Property::Label("Completed")]);
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
                    session.edit_action(|| {
                        session.body.begin_user_action();
                        session.decorating.set(true);
                        session.body.delete(&mut at, &mut end);
                        session
                            .body
                            .insert(&mut at, if b.is_active() { "x" } else { " " });
                        session.decorating.set(false);
                        session.body.end_user_action();
                        session.refresh_checklists();
                    });
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
        self.formatting_error.clone().or_else(|| {
            self.error
                .borrow()
                .clone()
                .or_else(|| self.style_error.borrow().clone())
                .or_else(|| self.pin_error.borrow().clone())
        })
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
        if self.formatting_error.is_some() {
            return false;
        }
        self.record_edit();
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
        match self
            .storage
            .update_formatted_note(&note, &crate::text_editing::spans(&self.body))
        {
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

/// Completed text keeps Pango's font-specific strike metrics and the theme's ink.
pub fn style_checklists(buffer: &gtk::TextBuffer, config: &crate::config::Config) {
    let Some(tag) = buffer.tag_table().lookup("completed") else {
        return;
    };
    let color = if config.theme == crate::config::Theme::Minimal && crate::appearance::dark(config)
    {
        gtk::gdk::RGBA::new(0.72, 0.73, 0.74, 1.)
    } else {
        gtk::gdk::RGBA::new(0.37, 0.35, 0.32, 1.)
    };
    tag.set_foreground_rgba(Some(&color));
}

pub fn readonly_checklists(view: &gtk::TextView) {
    let buffer = view.buffer();
    let hidden = gtk::TextTag::builder()
        .name("check-marker")
        .invisible(true)
        .build();
    buffer.tag_table().add(&hidden);
    for line in 0..buffer.line_count() {
        let Some(mut start) = buffer.iter_at_line(line) else {
            continue;
        };
        let mut end = start;
        end.forward_to_line_end();
        let text = buffer.text(&start, &end, true);
        if !text.starts_with("- [ ] ") && !text.starts_with("- [x] ") {
            continue;
        }
        let anchor = buffer.create_child_anchor(&mut start);
        let mut start = buffer.iter_at_child_anchor(&anchor);
        start.forward_char();
        let mut end = start;
        end.forward_chars(6);
        buffer.apply_tag(&hidden, &start, &end);
        let check = gtk::CheckButton::new();
        check.add_css_class("inline-check");
        check.set_active(text.starts_with("- [x] "));
        check.set_can_target(false);
        check.set_focusable(false);
        check.update_property(&[gtk::accessible::Property::Label("Completed")]);
        view.add_child_at_anchor(&check, &anchor);
    }
    decorate_checklists(&buffer);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "requires a GTK display; uses a disposable database"]
    fn typing_groups_follow_gtk_actions_in_title_and_body() {
        gtk::init().unwrap();
        let root = std::env::temp_dir().join(format!("stickies-grouping-{}", std::process::id()));
        let storage = Rc::new(Storage::open(&root.join("notes.db")).unwrap());
        let note = storage.create_note("", "", NoteColor::Yellow).unwrap();
        let time = Rc::new(Cell::new(Duration::ZERO));
        let clock = time.clone();
        let session =
            NoteSession::new_with_clock(note, storage.clone(), || {}, move || clock.get());
        let body = gtk::TextView::with_buffer(&session.body);
        crate::editor::connect_checklist_backspace(&body);
        let title = gtk::Entry::with_buffer(&session.title);
        session.attach_view(&body);
        session.attach_title(&title);
        let weak = Rc::downgrade(&session);
        crate::text_editing::bind_history(&body, move || weak.upgrade());
        let weak = Rc::downgrade(&session);
        crate::text_editing::bind_title_history(&title, move || weak.upgrade());
        let layout = gtk::Box::new(gtk::Orientation::Vertical, 0);
        layout.append(&title);
        layout.append(&body);
        let window = gtk::Window::new();
        window.set_child(Some(&layout));
        window.present();
        let wait = |done: &dyn Fn() -> bool| {
            let deadline = Instant::now() + Duration::from_secs(2);
            while !done() && Instant::now() < deadline {
                while glib::MainContext::default().iteration(false) {}
                std::thread::sleep(Duration::from_millis(1));
            }
            assert!(done());
        };
        wait(&|| window.is_mapped());
        body.grab_focus();
        let text = || {
            session
                .body
                .text(&session.body.start_iter(), &session.body.end_iter(), true)
                .to_string()
        };
        let type_body = |input: &str| {
            for c in input.chars() {
                time.set(time.get() + Duration::from_millis(20));
                body.emit_insert_at_cursor(&c.to_string());
            }
        };
        type_body("hello world");
        session.undo(false);
        assert_eq!(text(), "");
        session.undo(true);
        assert_eq!(text(), "hello world");
        assert_eq!(session.body.cursor_position(), 11);
        time.set(time.get() + Duration::from_millis(1200));
        type_body("!");
        session.undo(false);
        assert_eq!(text(), "hello world");
        session.undo(true);
        type_body("\nnext");
        session.undo(false);
        assert_eq!(text(), "hello world!\n");
        session.undo(false);
        assert_eq!(text(), "hello world!");
        session.undo(true);
        session.undo(true);
        for _ in 0..3 {
            body.emit_backspace();
        }
        assert_eq!(text(), "hello world!\nn");
        session.undo(false);
        assert_eq!(text(), "hello world!\nnext");
        body.clipboard().set_text(" PASTE");
        body.emit_by_name::<()>("paste-clipboard", &[]);
        wait(&|| text().ends_with(" PASTE"));
        session.undo(false);
        assert_eq!(text(), "hello world!\nnext");
        type_body("z");
        session.undo(true);
        assert_eq!(
            text(),
            "hello world!\nnextz",
            "new typing must discard paste redo"
        );
        session.body.select_range(
            &session.body.start_iter(),
            &crate::text_editing::iter(&session.body, 5),
        );
        body.emit_by_name::<()>("cut-clipboard", &[]);
        assert_eq!(text(), " world!\nnextz");
        session.undo(false);
        assert_eq!(text(), "hello world!\nnextz");
        session.body.select_range(
            &session.body.start_iter(),
            &crate::text_editing::iter(&session.body, 5),
        );
        session.toggle_mark(crate::text_editing::Mark::Bold);
        assert_eq!(crate::text_editing::spans(&session.body).len(), 1);
        session.undo(false);
        assert_eq!(text(), "hello world!\nnextz");
        assert!(crate::text_editing::spans(&session.body).is_empty());
        session.undo(true);
        assert_eq!(crate::text_editing::spans(&session.body)[0].end, 5);
        title.grab_focus();
        let entry = title.delegate().unwrap().downcast::<gtk::Text>().unwrap();
        for c in "café 東京".chars() {
            entry.emit_insert_at_cursor(c.to_string());
        }
        let keys = title
            .observe_controllers()
            .iter::<glib::Object>()
            .flatten()
            .filter_map(|c| c.downcast::<gtk::EventControllerKey>().ok())
            .find(|c| c.propagation_phase() == gtk::PropagationPhase::Capture)
            .unwrap();
        assert!(keys.emit_by_name::<bool>(
            "key-pressed",
            &[
                &gtk::gdk::Key::z,
                &0u32,
                &gtk::gdk::ModifierType::CONTROL_MASK
            ]
        ));
        assert_eq!(title.text(), "");
        assert_eq!(title.position(), 0);
        assert!(keys.emit_by_name::<bool>(
            "key-pressed",
            &[
                &gtk::gdk::Key::y,
                &0u32,
                &gtk::gdk::ModifierType::CONTROL_MASK
            ]
        ));
        assert_eq!(title.text(), "café 東京");
        assert_eq!(title.position(), 7);
        entry.emit_insert_at_cursor("a");
        entry.emit_move_cursor(gtk::MovementStep::LogicalPositions, -1, false);
        entry.emit_move_cursor(gtk::MovementStep::LogicalPositions, 1, false);
        entry.emit_insert_at_cursor("b");
        session.undo(false);
        assert_eq!(title.text(), "café 東京a");
        session.undo(false);
        assert_eq!(title.text(), "café 東京");
        title.clipboard().set_text(" title paste");
        entry.emit_paste_clipboard();
        wait(&|| title.text().ends_with(" title paste"));
        session.undo(false);
        assert_eq!(title.text(), "café 東京");
        title.select_region(0, 4);
        entry.emit_cut_clipboard();
        assert_eq!(title.text(), " 東京");
        session.undo(false);
        assert_eq!(title.text(), "café 東京");
        assert_eq!(title.position(), 4);
        title.set_position(-1);
        entry.emit_insert_at_cursor("a");
        title.emit_by_name::<()>("activate", &[]);
        entry.emit_insert_at_cursor("b");
        session.undo(false);
        assert_eq!(title.text(), "café 東京a");
        session.undo(false);
        assert_eq!(title.text(), "café 東京");
        session.body.set_text("Item");
        session.body.place_cursor(&session.body.end_iter());
        session.edit_action(|| crate::editor::toggle_line(&body, "- "));
        session.body.place_cursor(&session.body.end_iter());
        assert!(session.edit_action(|| crate::editor::continue_list(&body)));
        assert_eq!(text(), "- Item\n- ");
        session.undo(false);
        assert_eq!(text(), "- Item");
        session.undo(false);
        assert_eq!(text(), "Item");
        session.body.set_text("- [ ] Item");
        while glib::MainContext::default().iteration(false) {}
        let check = session.body.start_iter().child_anchor().unwrap().widgets()[0]
            .clone()
            .downcast::<gtk::CheckButton>()
            .unwrap();
        check.set_active(true);
        assert_eq!(text(), "- [x] Item");
        session.undo(false);
        assert_eq!(text(), "- [ ] Item");
        session
            .body
            .place_cursor(&crate::text_editing::iter(&session.body, 6));
        body.emit_backspace();
        assert_eq!(text(), "Item");
        session.undo(false);
        assert_eq!(text(), "- [ ] Item");
        assert!(session.flush());
        window.close();
        drop(session);
        drop(storage);
        std::fs::remove_dir_all(root).unwrap();
    }

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
        let first = crate::editor::Editor::new(session.clone(), config.clone(), true, || {});
        let second = crate::editor::Editor::new(session.clone(), config, false, || {});
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
        first
            .controls
            .weights
            .iter()
            .find(|(_, weight)| *weight == crate::config::NoteTextWeight::Bold)
            .unwrap()
            .0
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
            second
                .controls
                .weights
                .iter()
                .find(|(_, weight)| { *weight == crate::config::NoteTextWeight::Bold })
                .unwrap()
                .0
                .is_active()
        );
        assert!(
            !swatch(second.root.upcast_ref(), "Medium text")
                .unwrap()
                .is_active()
        );
        assert!(
            !second
                .controls
                .weights
                .iter()
                .find(|(_, weight)| { *weight == crate::config::NoteTextWeight::Regular })
                .unwrap()
                .0
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
