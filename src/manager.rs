use std::{cell::Cell, cell::RefCell, rc::Rc};

use gtk::prelude::*;

use crate::{
    app::{Note, NoteId},
    storage::{NoteCollection, NoteSort, Storage},
};

struct ManagerUi {
    window: gtk::ApplicationWindow,
    storage: Rc<Storage>,
    collection: Cell<NoteCollection>,
    sort: Cell<NoteSort>,
    notes: RefCell<Vec<Note>>,
    search: gtk::SearchEntry,
    list: gtk::ListBox,
    title: gtk::Label,
    body: gtk::TextView,
    pin: gtk::Button,
    archive: gtk::Button,
    restore: gtk::Button,
    delete: gtk::Button,
    status: gtk::Label,
    on_notes_changed: Box<dyn Fn()>,
}

#[derive(Clone)]
pub struct ManagerWindow {
    window: gtk::ApplicationWindow,
    _ui: Rc<ManagerUi>,
}

impl ManagerWindow {
    pub fn present(&self) {
        self.window.present();
    }

    pub fn refresh(&self) {
        self._ui.refresh();
    }

    pub fn window(&self) -> &gtk::ApplicationWindow {
        &self.window
    }
}

impl ManagerUi {
    fn refresh(&self) {
        while let Some(child) = self.list.first_child() {
            self.list.remove(&child);
        }

        let query = self.search.text();
        let notes =
            match self
                .storage
                .search_notes(self.collection.get(), query.as_str(), self.sort.get())
            {
                Ok(notes) => notes,
                Err(error) => {
                    eprintln!("Stickies could not load All Notes: {error}");
                    self.status.set_text("Could not load notes");
                    self.notes.borrow_mut().clear();
                    self.clear_preview();
                    return;
                }
            };

        for note in &notes {
            self.list.append(&note_row(note));
        }

        let count = notes.len();
        self.status.set_text(&match count {
            0 => "No notes found".to_owned(),
            1 => "1 note".to_owned(),
            _ => format!("{count} notes"),
        });
        self.notes.replace(notes);

        if count == 0 {
            self.clear_preview();
        } else if let Some(first) = self.list.row_at_index(0) {
            self.list.select_row(Some(&first));
        }
    }

    fn selected_note(&self) -> Option<Note> {
        let row = self.list.selected_row()?;
        let index = usize::try_from(row.index()).ok()?;
        self.notes.borrow().get(index).cloned()
    }

    fn show_selected(&self, row: Option<&gtk::ListBoxRow>) {
        let Some(row) = row else {
            self.clear_preview();
            return;
        };
        let Ok(index) = usize::try_from(row.index()) else {
            self.clear_preview();
            return;
        };
        let notes = self.notes.borrow();
        let Some(note) = notes.get(index) else {
            self.clear_preview();
            return;
        };

        self.title.set_text(display_title(note));
        self.body.buffer().set_text(&note.body);
        self.pin
            .set_label(if note.pinned { "Unpin" } else { "Pin" });
        self.set_action_state(true);
    }

    fn clear_preview(&self) {
        self.title.set_text("Select a note");
        self.body.buffer().set_text("");
        self.set_action_state(false);
    }

    fn set_action_state(&self, has_selection: bool) {
        let archived = self.collection.get() == NoteCollection::Archived;
        self.pin.set_visible(!archived);
        self.archive.set_visible(!archived);
        self.restore.set_visible(archived);
        self.pin.set_sensitive(has_selection && !archived);
        self.archive.set_sensitive(has_selection && !archived);
        self.restore.set_sensitive(has_selection && archived);
        self.delete.set_sensitive(has_selection);
    }

    fn pin_selected(&self) {
        let Some(note) = self.selected_note() else {
            return;
        };
        if self.collection.get() != NoteCollection::Active {
            return;
        }

        match self.storage.set_note_pinned(note.id, !note.pinned) {
            Ok(()) => self.finish_mutation(),
            Err(error) => self.show_mutation_error("change the pin", error),
        }
    }

    fn archive_selected(&self) {
        let Some(note) = self.selected_note() else {
            return;
        };
        if self.collection.get() != NoteCollection::Active {
            return;
        }

        match self.storage.archive_note(note.id) {
            Ok(()) => self.finish_mutation(),
            Err(error) => self.show_mutation_error("archive the note", error),
        }
    }

    fn restore_selected(&self) {
        let Some(note) = self.selected_note() else {
            return;
        };
        if self.collection.get() != NoteCollection::Archived {
            return;
        }

        match self.storage.restore_note(note.id) {
            Ok(()) => self.finish_mutation(),
            Err(error) => self.show_mutation_error("restore the note", error),
        }
    }

    fn confirm_delete(self: &Rc<Self>) {
        let Some(note) = self.selected_note() else {
            return;
        };
        let dialog = gtk::MessageDialog::builder()
            .transient_for(&self.window)
            .modal(true)
            .message_type(gtk::MessageType::Question)
            .buttons(gtk::ButtonsType::Cancel)
            .text("Delete note?")
            .secondary_text(format!(
                "Delete \"{}\"? This cannot be undone yet.",
                display_title(&note)
            ))
            .build();
        dialog.add_button("Delete", gtk::ResponseType::Accept);

        let note_id = note.id;
        let ui = Rc::downgrade(self);
        dialog.run_async(move |dialog, response| {
            if response == gtk::ResponseType::Accept
                && let Some(ui) = ui.upgrade()
            {
                ui.delete_note(note_id);
            }
            dialog.close();
        });
    }

    fn delete_note(&self, note_id: NoteId) {
        match self.storage.delete_note(note_id) {
            Ok(()) => self.finish_mutation(),
            Err(error) => self.show_mutation_error("delete the note", error),
        }
    }

    fn finish_mutation(&self) {
        (self.on_notes_changed)();
        self.refresh();
    }

    fn show_mutation_error(&self, action: &str, error: impl std::fmt::Display) {
        eprintln!("Stickies could not {action}: {error}");
        self.status.set_text(&format!("Could not {action}"));
    }
}

pub fn build_window(
    application: &gtk::Application,
    storage: Rc<Storage>,
    on_notes_changed: impl Fn() + 'static,
) -> ManagerWindow {
    let window = gtk::ApplicationWindow::builder()
        .application(application)
        .title("All Notes")
        .default_width(900)
        .default_height(600)
        .build();

    let root = gtk::Box::new(gtk::Orientation::Vertical, 12);
    root.set_margin_top(16);
    root.set_margin_bottom(16);
    root.set_margin_start(16);
    root.set_margin_end(16);

    let controls = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let search = gtk::SearchEntry::new();
    search.set_placeholder_text(Some("Search titles and notes"));
    search.set_hexpand(true);
    let sort = gtk::DropDown::from_strings(&["Recently edited", "Title"]);
    sort.set_tooltip_text(Some("Sort notes"));
    let active = gtk::ToggleButton::with_label("Active");
    let archived = gtk::ToggleButton::with_label("Archived");
    archived.set_group(Some(&active));
    active.set_active(true);
    controls.append(&search);
    controls.append(&sort);
    controls.append(&active);
    controls.append(&archived);

    let content = gtk::Paned::new(gtk::Orientation::Horizontal);
    content.set_wide_handle(true);
    content.set_vexpand(true);

    let list = gtk::ListBox::new();
    list.set_selection_mode(gtk::SelectionMode::Single);
    let list_scroll = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .min_content_width(300)
        .child(&list)
        .build();

    let preview = gtk::Box::new(gtk::Orientation::Vertical, 12);
    preview.set_margin_start(16);
    let title = gtk::Label::new(Some("Select a note"));
    title.set_xalign(0.0);
    title.set_wrap(true);
    title.add_css_class("title-2");
    let body = gtk::TextView::new();
    body.set_editable(false);
    body.set_cursor_visible(false);
    body.set_wrap_mode(gtk::WrapMode::WordChar);
    body.set_left_margin(12);
    body.set_right_margin(12);
    body.set_top_margin(12);
    body.set_bottom_margin(12);
    let body_scroll = gtk::ScrolledWindow::builder()
        .vexpand(true)
        .child(&body)
        .build();

    let actions = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let pin = gtk::Button::with_label("Pin");
    let archive = gtk::Button::with_label("Archive");
    let restore = gtk::Button::with_label("Restore");
    let delete = gtk::Button::with_label("Delete");
    actions.append(&pin);
    actions.append(&archive);
    actions.append(&restore);
    actions.append(&delete);
    preview.append(&title);
    preview.append(&body_scroll);
    preview.append(&actions);

    content.set_start_child(Some(&list_scroll));
    content.set_end_child(Some(&preview));
    content.set_position(340);

    let status = gtk::Label::new(None);
    status.set_xalign(0.0);
    root.append(&controls);
    root.append(&content);
    root.append(&status);
    window.set_child(Some(&root));

    let ui = Rc::new(ManagerUi {
        window: window.clone(),
        storage,
        collection: Cell::new(NoteCollection::Active),
        sort: Cell::new(NoteSort::Recent),
        notes: RefCell::new(Vec::new()),
        search,
        list,
        title,
        body,
        pin,
        archive,
        restore,
        delete,
        status,
        on_notes_changed: Box::new(on_notes_changed),
    });

    connect_manager_signals(&ui, active, archived, sort);
    ui.refresh();
    ManagerWindow { window, _ui: ui }
}

fn connect_manager_signals(
    ui: &Rc<ManagerUi>,
    active: gtk::ToggleButton,
    archived: gtk::ToggleButton,
    sort: gtk::DropDown,
) {
    {
        let search = ui.search.clone();
        let ui = Rc::downgrade(ui);
        search.connect_search_changed(move |_| {
            if let Some(ui) = ui.upgrade() {
                ui.refresh();
            }
        });
    }
    {
        let ui = Rc::downgrade(ui);
        active.connect_toggled(move |button| {
            if button.is_active()
                && let Some(ui) = ui.upgrade()
            {
                ui.collection.set(NoteCollection::Active);
                ui.refresh();
            }
        });
    }
    {
        let ui = Rc::downgrade(ui);
        archived.connect_toggled(move |button| {
            if button.is_active()
                && let Some(ui) = ui.upgrade()
            {
                ui.collection.set(NoteCollection::Archived);
                ui.refresh();
            }
        });
    }
    {
        let ui = Rc::downgrade(ui);
        sort.connect_selected_notify(move |dropdown| {
            if let Some(ui) = ui.upgrade() {
                let sort = if dropdown.selected() == 1 {
                    NoteSort::Title
                } else {
                    NoteSort::Recent
                };
                ui.sort.set(sort);
                ui.refresh();
            }
        });
    }
    {
        let list = ui.list.clone();
        let ui = Rc::downgrade(ui);
        list.connect_row_selected(move |_, row| {
            if let Some(ui) = ui.upgrade() {
                ui.show_selected(row);
            }
        });
    }
    {
        let pin = ui.pin.clone();
        let ui = Rc::downgrade(ui);
        pin.connect_clicked(move |_| {
            if let Some(ui) = ui.upgrade() {
                ui.pin_selected();
            }
        });
    }
    {
        let archive = ui.archive.clone();
        let ui = Rc::downgrade(ui);
        archive.connect_clicked(move |_| {
            if let Some(ui) = ui.upgrade() {
                ui.archive_selected();
            }
        });
    }
    {
        let restore = ui.restore.clone();
        let ui = Rc::downgrade(ui);
        restore.connect_clicked(move |_| {
            if let Some(ui) = ui.upgrade() {
                ui.restore_selected();
            }
        });
    }
    {
        let delete = ui.delete.clone();
        let ui = Rc::downgrade(ui);
        delete.connect_clicked(move |_| {
            if let Some(ui) = ui.upgrade() {
                ui.confirm_delete();
            }
        });
    }
}

fn note_row(note: &Note) -> gtk::ListBoxRow {
    let content = gtk::Box::new(gtk::Orientation::Vertical, 4);
    content.set_margin_top(10);
    content.set_margin_bottom(10);
    content.set_margin_start(12);
    content.set_margin_end(12);

    let heading = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let title = gtk::Label::new(Some(display_title(note)));
    title.set_xalign(0.0);
    title.set_hexpand(true);
    title.set_ellipsize(gtk::pango::EllipsizeMode::End);
    heading.append(&title);
    if note.pinned {
        let pinned = gtk::Label::new(Some("Pinned"));
        pinned.add_css_class("dim-label");
        heading.append(&pinned);
    }

    let preview = gtk::Label::new(Some(&body_preview(&note.body)));
    preview.set_xalign(0.0);
    preview.set_ellipsize(gtk::pango::EllipsizeMode::End);
    preview.add_css_class("dim-label");
    content.append(&heading);
    content.append(&preview);

    let row = gtk::ListBoxRow::new();
    row.set_child(Some(&content));
    row
}

fn display_title(note: &Note) -> &str {
    if note.title.trim().is_empty() {
        "Untitled note"
    } else {
        &note.title
    }
}

fn body_preview(body: &str) -> String {
    let one_line = body.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut characters = one_line.chars();
    let preview = characters.by_ref().take(80).collect::<String>();
    if characters.next().is_some() {
        format!("{preview}...")
    } else {
        preview
    }
}
