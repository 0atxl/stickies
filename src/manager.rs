use crate::{
    app::{Note, NoteColor, NoteId},
    appearance,
    config::{Config, Paper, Theme},
    note_controls::NoteControls,
    note_session::NoteSession,
    storage::{NoteCollection, NoteSort, Storage},
    ui_assets,
};
use adw::prelude::*;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ManagerChange {
    Pin,
    Create,
    Archive,
    Restore,
    Delete,
}

pub struct Callbacks {
    pub sort: NoteSort,
    pub sorted: Box<dyn Fn(NoteSort)>,
    pub session: Box<dyn Fn(NoteId) -> Option<Rc<NoteSession>>>,
    pub changed: Box<dyn Fn(NoteId, ManagerChange)>,
    pub popout: Box<dyn Fn(NoteId)>,
    pub settings: Rc<dyn Fn(Config) -> Result<(), String>>,
}
struct ManagerUi {
    window: adw::ApplicationWindow,
    settings_panel: crate::settings::Panel,
    storage: Rc<Storage>,
    callbacks: Callbacks,
    config: Rc<RefCell<Config>>,
    collection: Cell<NoteCollection>,
    sort: Cell<NoteSort>,
    selected: Cell<Option<NoteId>>,
    busy: Cell<bool>,
    notes: RefCell<Vec<Note>>,
    session: RefCell<Option<Rc<NoteSession>>>,
    subscription: RefCell<Option<Rc<dyn Fn()>>>,
    list: gtk::ListBox,
    search: gtk::SearchEntry,
    title: gtk::Entry,
    body: gtk::TextView,
    pin: gtk::ToggleButton,
    popout: gtk::Button,
    more: gtk::MenuButton,
    controls: NoteControls,
    pattern_choice: gtk::DropDown,
    syncing_style: Cell<bool>,
    edit_actions: gtk::Box,
    last_error: RefCell<Option<(NoteId, String)>>,
    error_toast: RefCell<Option<adw::Toast>>,
    archive: gtk::Button,
    restore: gtk::Button,
    dates: gtk::Label,
    save_state: gtk::Label,
    save_indicator: gtk::Stack,
    filters: Vec<(gtk::Button, NoteCollection)>,
    pages: gtk::Stack,
    empty_title: gtk::Label,
    empty_text: gtk::Label,
    empty_action: gtk::Button,
    toast: adw::ToastOverlay,
    resolved: Rc<RefCell<Config>>,
    color: Rc<Cell<NoteColor>>,
    seed: Rc<Cell<u64>>,
    background: gtk::DrawingArea,
    font: gtk::CssProvider,
}
#[derive(Clone)]
pub struct ManagerWindow(Rc<ManagerUi>);
impl ManagerWindow {
    pub fn present(&self) {
        self.0.window.present();
        self.0.sync_editor();
    }
    pub fn refresh(&self) {
        self.0.refresh(None);
    }
    pub fn refresh_appearance(&self) {
        self.0.settings_panel.refresh();
        self.0.sync_editor();
    }
    pub fn window(&self) -> &adw::ApplicationWindow {
        &self.0.window
    }
    pub fn flush(&self) -> bool {
        self.0.flush()
    }
}
impl ManagerUi {
    fn flush(&self) -> bool {
        let session = self.session.borrow().clone();
        session.is_none_or(|s| s.flush())
    }
    fn refresh(self: &Rc<Self>, preferred: Option<NoteId>) {
        if self.busy.replace(true) {
            return;
        }
        for (button, collection) in &self.filters {
            if self.collection.get() == *collection {
                button.add_css_class("selected");
            } else {
                button.remove_css_class("selected");
            }
        }
        let previous = self.selected.get();
        let query = self.search.text();
        let notes =
            match self
                .storage
                .search_notes(self.collection.get(), query.as_str(), self.sort.get())
            {
                Ok(notes) => notes,
                Err(e) => {
                    self.message(&format!("Could not load notes: {e}"));
                    self.busy.set(false);
                    return;
                }
            };
        let target = preferred
            .or(previous)
            .filter(|id| notes.iter().any(|n| n.id == *id));
        if target != previous && !self.flush() {
            self.busy.set(false);
            return;
        }
        while let Some(child) = self.list.first_child() {
            self.list.remove(&child);
        }
        for note in &notes {
            self.list.append(&note_row(
                note,
                self.storage.is_archived(note.id).unwrap_or(false),
                self.storage
                    .note_dates(note.id)
                    .ok()
                    .map(|(_, updated)| updated),
            ));
        }
        self.notes.replace(notes);
        if let Some(id) = target {
            let index = self.notes.borrow().iter().position(|n| n.id == id).unwrap();
            self.list
                .select_row(self.list.row_at_index(index as i32).as_ref());
            self.show_note(id);
        } else {
            self.clear_editor();
        }
        self.busy.set(false);
    }
    fn select(self: &Rc<Self>, row: Option<&gtk::ListBoxRow>) {
        if self.busy.replace(true) {
            return;
        }
        let id = row.and_then(|r| self.notes.borrow().get(r.index() as usize).map(|n| n.id));
        if id != self.selected.get() {
            if self.flush() {
                if let Some(id) = id {
                    self.show_note(id);
                } else {
                    self.clear_editor();
                }
            } else if let Some(old) = self.selected.get() {
                let index = self.notes.borrow().iter().position(|n| n.id == old);
                if let Some(i) = index {
                    self.list
                        .select_row(self.list.row_at_index(i as i32).as_ref());
                }
            }
        }
        self.busy.set(false);
    }
    fn show_note(self: &Rc<Self>, id: NoteId) {
        let archived = match self.storage.is_archived(id) {
            Ok(value) => value,
            Err(e) => {
                self.message(&format!("Could not load note: {e}"));
                return;
            }
        };
        if self.selected.get() != Some(id) || self.session.borrow().is_none() {
            let note = self
                .notes
                .borrow()
                .iter()
                .find(|n| n.id == id)
                .cloned()
                .unwrap();
            self.subscription.borrow_mut().take();
            self.session.borrow_mut().take();
            if !archived {
                let Some(session) = (self.callbacks.session)(id) else {
                    return;
                };
                self.title.set_buffer(&session.title);
                self.body.set_buffer(Some(&session.body));
                session.attach_view(&self.body);
                let weak = Rc::downgrade(self);
                self.subscription.replace(Some(session.subscribe(move || {
                    if let Some(ui) = weak.upgrade() {
                        ui.sync_editor();
                    }
                })));
                self.session.replace(Some(session));
            } else {
                self.title
                    .set_buffer(&gtk::EntryBuffer::new(Some(note.title.as_str())));
                let buffer = gtk::TextBuffer::new(None::<&gtk::TextTagTable>);
                buffer.set_text(&note.body);
                crate::note_session::decorate_checklists(&buffer);
                self.body.set_buffer(Some(&buffer));
            }
        }
        self.selected.set(Some(id));
        self.title.set_editable(!archived);
        self.body.set_editable(!archived);
        self.body.set_cursor_visible(!archived);
        self.pin.set_visible(!archived);
        self.popout.set_visible(!archived);
        self.archive.set_visible(!archived);
        self.restore.set_visible(archived);
        self.more.set_sensitive(true);
        self.pages.set_visible_child_name("editor");
        self.sync_editor();
    }
    fn clear_editor(&self) {
        self.selected.set(None);
        self.last_error.borrow_mut().take();
        if let Some(toast) = self.error_toast.borrow_mut().take() {
            toast.dismiss();
        }
        self.subscription.borrow_mut().take();
        self.session.borrow_mut().take();
        self.title.set_buffer(&gtk::EntryBuffer::new(None::<&str>));
        self.body
            .set_buffer(Some(&gtk::TextBuffer::new(None::<&gtk::TextTagTable>)));
        let has_notes = !self.notes.borrow().is_empty();
        let searching = !self.search.text().is_empty();
        let archived = self.collection.get() == NoteCollection::Archived;
        self.empty_title.set_text(if has_notes {
            "Choose a note"
        } else if searching {
            "No matching notes"
        } else if archived {
            "Archive is empty"
        } else {
            "A little space for your thoughts"
        });
        self.empty_text.set_text(if has_notes {
            "Select a note from the list to view or edit it."
        } else if searching {
            "Try another word or clear your search."
        } else if archived {
            "Notes you archive will appear here."
        } else {
            "Capture an idea, a list, or something to remember."
        });
        self.empty_action.set_label(if has_notes {
            "Create a note"
        } else if searching {
            "Clear search"
        } else if archived {
            "Back to notes"
        } else {
            "Create a note"
        });
        self.pages.set_visible_child_name("empty");
    }
    fn sync_editor(&self) {
        let defaults = self.config.borrow().clone();
        let (above, below) = match defaults.theme {
            Theme::Paper => (1, 3),
            Theme::Playful => (1, 3),
            Theme::Minimal => (1, 3),
        };
        self.body.set_pixels_above_lines(above);
        self.body.set_pixels_below_lines(below);
        if defaults.theme == Theme::Minimal {
            self.window.add_css_class("minimal");
        } else {
            self.window.remove_css_class("minimal");
        }
        if defaults.theme == Theme::Minimal && appearance::dark(&defaults) {
            self.window.add_css_class("minimal-dark");
        } else {
            self.window.remove_css_class("minimal-dark");
        }
        if defaults.theme == Theme::Playful {
            self.window.add_css_class("playful");
        } else {
            self.window.remove_css_class("playful");
        }
        let session = self.session.borrow().clone();
        let style = session.as_ref().map(|s| s.style()).unwrap_or_else(|| {
            self.selected
                .get()
                .and_then(|id| self.storage.load_note_style(id).ok())
                .unwrap_or_default()
        });
        self.resolved.replace(style.resolve(&defaults));
        if let Some(session) = session {
            self.color.set(session.color());
            self.pin.set_active(session.pinned());
            self.pin.set_tooltip_text(Some(if session.pinned() {
                "Unpin note"
            } else {
                "Pin note"
            }));
            let error = session.error();
            let error_key = error.clone().map(|message| (session.id(), message));
            if (self.window.is_visible() || error_key.is_none())
                && *self.last_error.borrow() != error_key
            {
                self.last_error.replace(error_key);
                if let Some(toast) = self.error_toast.borrow_mut().take() {
                    toast.dismiss();
                }
                if let Some(error) = &error {
                    let toast = adw::Toast::builder().title(error).use_markup(false).build();
                    self.toast.add_toast(toast.clone());
                    self.error_toast.replace(Some(toast));
                }
            }
            self.save_state
                .set_text(if error.is_some() { "Save failed" } else { "" });
            if error.is_some() {
                self.save_state.add_css_class("error");
            } else {
                self.save_state.remove_css_class("error");
            }
            self.save_indicator
                .set_visible_child_name(if error.is_some() {
                    "error"
                } else if session.is_dirty() {
                    "saving"
                } else {
                    "saved"
                });
            self.save_indicator
                .set_tooltip_text(Some(if error.is_some() {
                    "Could not save note"
                } else if session.is_dirty() {
                    "Saving"
                } else {
                    "Saved"
                }));
            if !defaults.reduced_motion
                && gtk::Settings::default().is_some_and(|s| s.is_gtk_enable_animations())
            {
                self.save_indicator.add_css_class("saving-motion");
            } else {
                self.save_indicator.remove_css_class("saving-motion");
            }
            self.save_indicator
                .set_transition_duration(if defaults.reduced_motion { 0 } else { 140 });
            if let Some(index) = self
                .notes
                .borrow()
                .iter()
                .position(|n| n.id == session.id())
                && let Some(row) = self.list.row_at_index(index as i32)
                && let Some(content) = row.child().and_downcast::<gtk::Box>()
                && let Some(text) = content.last_child().and_downcast::<gtk::Box>()
            {
                if let Some(heading) = text.first_child().and_downcast::<gtk::Box>() {
                    if let Some(pin) = heading.last_child() {
                        pin.set_visible(session.pinned());
                    }
                    let title = heading.first_child().and_downcast::<gtk::Label>().unwrap();
                    if let Some(date) = title.next_sibling().and_downcast::<gtk::Label>() {
                        date.set_text(&short_date(
                            self.storage
                                .note_dates(session.id())
                                .ok()
                                .map(|(_, updated)| updated),
                        ));
                    }
                    let live_title = session.title.text();
                    title.set_text(if live_title.is_empty() {
                        "Untitled note"
                    } else {
                        live_title.as_str()
                    });
                }
                if let Some(snippet) = text.last_child().and_downcast::<gtk::Label>() {
                    let buffer = &session.body;
                    let (start, end) = buffer.bounds();
                    snippet.set_text(
                        &buffer
                            .text(&start, &end, true)
                            .lines()
                            .collect::<Vec<_>>()
                            .join(" · "),
                    );
                }
            }
        } else if let Some(id) = self.selected.get() {
            self.last_error.borrow_mut().take();
            if let Some(toast) = self.error_toast.borrow_mut().take() {
                toast.dismiss();
            }
            if let Some(note) = self.notes.borrow().iter().find(|n| n.id == id) {
                self.color.set(note.color);
            }
            self.save_state.set_text("Archived · restore to edit");
            self.save_state.remove_css_class("error");
            self.save_indicator.set_visible_child_name("error");
            self.save_indicator
                .set_tooltip_text(Some("Restore this note to edit it"));
        }
        self.syncing_style.set(true);
        self.controls.sync(&style, self.color.get(), defaults.theme);
        self.pattern_choice
            .set_selected(style.paper.map_or(0, |paper| paper as u32 + 1));
        self.edit_actions.set_sensitive(self.body.is_editable());
        self.controls.root.set_sensitive(self.body.is_editable());
        self.pattern_choice.set_sensitive(self.body.is_editable());
        self.syncing_style.set(false);
        appearance::apply_note_font(&self.font, &self.resolved.borrow(), &style);
        self.seed
            .set(self.selected.get().map_or(0, |id| id.value()));
        self.background.queue_draw();
        ui_assets::refresh_icons(
            self.window.upcast_ref(),
            defaults.theme == Theme::Playful,
            defaults.theme != Theme::Paper,
        );
        if let Some(id) = self.selected.get()
            && let Ok((created, updated)) = self.storage.note_dates(id)
        {
            let date = |millis| {
                gtk::glib::DateTime::from_unix_local(millis / 1000)
                    .ok()
                    .and_then(|d| d.format("%d %b · %H:%M").ok())
                    .map(|s| s.to_string())
                    .unwrap_or_default()
            };
            self.dates.set_text(&format!(
                "Created {}   ·   Updated {}",
                date(created),
                date(updated)
            ));
        }
    }
    fn create(self: &Rc<Self>) {
        if !self.flush() {
            return;
        }
        match self.storage.create_note("", "", NoteColor::Yellow) {
            Ok(note) => {
                if self.collection.get() == NoteCollection::Archived {
                    self.collection.set(NoteCollection::Active);
                }
                self.search.set_text("");
                self.refresh(Some(note.id));
                self.title.grab_focus();
                (self.callbacks.changed)(note.id, ManagerChange::Create);
            }
            Err(e) => self.message(&format!("Could not create note: {e}")),
        }
    }
    fn mutate(self: &Rc<Self>, change: ManagerChange) {
        let Some(id) = self.selected.get() else {
            return;
        };
        if !self.flush() {
            return;
        }
        let result = match change {
            ManagerChange::Create => return,
            ManagerChange::Pin => {
                let session = self.session.borrow().clone();
                if let Some(session) = session {
                    session.set_pinned(!session.pinned())
                } else {
                    self.storage.set_note_pinned(id, true)
                }
            }
            ManagerChange::Archive => self.storage.archive_note(id),
            ManagerChange::Restore => self.storage.restore_note(id),
            ManagerChange::Delete => self.storage.delete_note(id),
        };
        if let Err(e) = result {
            self.sync_editor();
            // Live pin failures are already reported by the shared session.
            if change != ManagerChange::Pin || self.session.borrow().is_none() {
                self.message(&format!("Could not update note: {e}"));
            }
            return;
        }
        if matches!(change, ManagerChange::Archive | ManagerChange::Delete) {
            self.subscription.borrow_mut().take();
            self.session.borrow_mut().take();
            self.selected.set(None);
        }
        (self.callbacks.changed)(id, change);
        self.refresh(None);
        if change == ManagerChange::Delete {
            let toast = adw::Toast::builder()
                .title("Note deleted")
                .button_label("Undo")
                .timeout(6)
                .build();
            let weak = Rc::downgrade(self);
            toast.connect_button_clicked(move |_| {
                if let Some(ui) = weak.upgrade() {
                    match ui.storage.undo_delete(id) {
                        Ok(()) => {
                            (ui.callbacks.changed)(id, ManagerChange::Restore);
                            ui.refresh(Some(id));
                        }
                        Err(e) => ui.message(&format!("Could not undo deletion: {e}")),
                    }
                }
            });
            self.toast.add_toast(toast);
        }
    }
    fn message(&self, text: &str) {
        self.toast.add_toast(adw::Toast::new(text));
    }
}

pub fn build_window(
    application: &gtk::Application,
    storage: Rc<Storage>,
    config: Rc<RefCell<Config>>,
    callbacks: Callbacks,
) -> ManagerWindow {
    let display = gtk::gdk::Display::default().expect("GTK display");
    let css = gtk::CssProvider::new();
    css.load_from_data(&format!(
        "{}\n{}",
        include_str!("../assets/all-notes.css"),
        appearance::palette_css()
    ));
    gtk::style_context_add_provider_for_display(
        &display,
        &css,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION + 1,
    );
    let window = adw::ApplicationWindow::builder()
        .application(application)
        .title("All Notes")
        .default_width(720)
        .default_height(540)
        .width_request(680)
        .height_request(460)
        .build();
    window.add_css_class("management");
    window.connect_destroy(move |_| {
        gtk::style_context_remove_provider_for_display(&display, &css);
    });
    let root = gtk::Box::new(gtk::Orientation::Vertical, 0);
    let header = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    header.add_css_class("main-header");
    let heading = gtk::Label::new(Some("All Notes"));
    heading.add_css_class("panel-title");
    heading.set_xalign(0.);
    heading.set_width_request(112);
    let search = gtk::SearchEntry::new();
    search.add_css_class("search-field");
    search.set_placeholder_text(Some("Search notes"));
    search.set_hexpand(true);
    let search_clamp = adw::Clamp::builder()
        .valign(gtk::Align::Center)
        .maximum_size(380)
        .child(&search)
        .hexpand(true)
        .build();
    let new = ui_assets::icon("plus", "New note");
    let sort_button = menu_button("sort", "Sort notes");
    let recent = gtk::Button::with_label("Recently edited");
    let alphabetical = gtk::Button::with_label("Title");
    set_menu(&sort_button, &[&recent, &alphabetical]);
    let filters: Vec<_> = [
        ("All", NoteCollection::All),
        ("Active", NoteCollection::Active),
        ("Archived", NoteCollection::Archived),
    ]
    .into_iter()
    .map(|(label, collection)| (gtk::Button::with_label(label), collection))
    .collect();
    let settings = ui_assets::icon("settings", "Settings");
    let close = ui_assets::icon("close", "Close All Notes");
    header.append(&heading);
    header.append(&search_clamp);
    for widget in [
        new.upcast_ref::<gtk::Widget>(),
        sort_button.upcast_ref(),
        settings.upcast_ref(),
        close.upcast_ref(),
    ] {
        header.append(widget);
    }
    let handle = gtk::WindowHandle::new();
    handle.set_child(Some(&header));
    root.append(&handle);
    let content = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    let list = gtk::ListBox::new();
    list.add_css_class("note-list");
    let scroll = gtk::ScrolledWindow::builder()
        .overlay_scrolling(false)
        .hscrollbar_policy(gtk::PolicyType::Never)
        .vexpand(true)
        .child(&list)
        .build();
    let list_column = gtk::Box::new(gtk::Orientation::Vertical, 0);
    list_column.set_width_request(240);
    list_column.add_css_class("list-column");
    let filter_row = gtk::Box::new(gtk::Orientation::Horizontal, 4);
    filter_row.add_css_class("collection-filters");
    for (button, _) in &filters {
        filter_row.append(button);
    }
    list_column.append(&scroll);
    content.append(&list_column);
    let pages = gtk::Stack::new();
    pages.set_hexpand(true);
    pages.set_vexpand(true);
    pages.set_hhomogeneous(false);
    pages.set_vhomogeneous(false);
    let editor = gtk::Box::new(gtk::Orientation::Vertical, 6);
    editor.add_css_class("editor-region");
    let resolved = Rc::new(RefCell::new(config.borrow().clone()));
    let color = Rc::new(Cell::new(NoteColor::Yellow));
    let seed = Rc::new(Cell::new(0));
    let background = appearance::paper_background(resolved.clone(), color.clone(), seed.clone());
    let sheet = gtk::Overlay::new();
    sheet.set_child(Some(&background));
    sheet.set_vexpand(true);
    let paper_content = gtk::Box::new(gtk::Orientation::Vertical, 8);
    paper_content.add_css_class("inline-content");
    let title = gtk::Entry::new();
    title.add_css_class("inline-title");
    title.set_placeholder_text(Some("Untitled note"));
    title.set_width_chars(8);
    title.set_hexpand(true);
    let body = gtk::TextView::new();
    crate::editor::connect_checklist_backspace(&body);
    body.add_css_class("inline-body");
    body.set_wrap_mode(gtk::WrapMode::WordChar);
    body.set_pixels_above_lines(3);
    body.set_pixels_below_lines(5);
    body.set_top_margin(6);
    body.set_bottom_margin(8);
    let body_scroll = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .vscrollbar_policy(gtk::PolicyType::Automatic)
        .vexpand(true)
        .child(&body)
        .build();
    paper_content.append(&title);
    paper_content.append(&body_scroll);
    sheet.add_overlay(&paper_content);
    let vertical = adw::Clamp::builder()
        .orientation(gtk::Orientation::Vertical)
        .maximum_size(420)
        .child(&sheet)
        .build();
    let clamp = adw::Clamp::builder()
        .maximum_size(420)
        .tightening_threshold(360)
        .child(&vertical)
        .vexpand(true)
        .build();
    editor.append(&clamp);
    let dates = gtk::Label::new(None);
    dates.add_css_class("metadata");
    dates.set_wrap(true);
    dates.set_margin_top(8);
    editor.append(&dates);
    let save_state = gtk::Label::new(None);
    save_state.add_css_class("save-state");
    save_state.set_wrap(true);
    let save_indicator = gtk::Stack::new();
    save_indicator.set_transition_type(gtk::StackTransitionType::Crossfade);
    save_indicator.set_transition_duration(140);
    save_indicator.set_halign(gtk::Align::Center);
    save_indicator.add_css_class("save-indicator");
    save_indicator.add_named(&save_state, Some("error"));
    let dots = gtk::Box::new(gtk::Orientation::Horizontal, 3);
    dots.set_halign(gtk::Align::Center);
    for i in 0..3 {
        let dot = gtk::Label::new(Some("•"));
        dot.add_css_class("save-dot");
        dot.add_css_class(&format!("dot-{i}"));
        dots.append(&dot);
    }
    save_indicator.add_named(&dots, Some("saving"));
    save_indicator.add_named(&gtk::Label::new(Some("✓")), Some("saved"));
    editor.append(&save_indicator);
    let actions = gtk::Box::new(gtk::Orientation::Horizontal, 16);
    actions.set_halign(gtk::Align::Center);
    actions.set_margin_top(4);
    let pin = gtk::ToggleButton::new();
    pin.set_child(Some(&ui_assets::image("pin")));
    pin.add_css_class("icon-control");
    let popout = ui_assets::icon("popout", "Open in floating window");
    let more = menu_button("more", "Note actions");
    let archive = gtk::Button::with_label("Archive");
    let restore = gtk::Button::with_label("Restore");
    let delete = gtk::Button::with_label("Delete");
    let copy = gtk::Button::with_label("Copy text");
    let checklist = ui_assets::icon("checklist", "Checklist · toggle current item");
    let bullet = appearance::icon("view-list-bullet-symbolic", "Bullet list");
    let numbered = appearance::icon("view-list-ordered-symbolic", "Numbered list");
    let controls = NoteControls::new();
    let pattern_choice =
        gtk::DropDown::from_strings(&["Default pattern", "Plain", "Dots", "Square grid", "Ruled"]);
    pattern_choice.set_tooltip_text(Some(
        "Note canvas pattern; Default pattern follows Appearance",
    ));
    let tools = gtk::Box::new(gtk::Orientation::Vertical, 8);
    tools.add_css_class("main-note-tools");
    let edit_actions = gtk::Box::new(gtk::Orientation::Horizontal, 4);
    edit_actions.set_halign(gtk::Align::Center);
    for button in [&checklist, &bullet, &numbered] {
        edit_actions.append(button);
    }
    tools.append(&edit_actions);
    tools.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
    tools.append(&controls.root);
    tools.append(&pattern_choice);
    tools.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
    let more_actions = gtk::Box::new(gtk::Orientation::Horizontal, 2);
    for button in [&copy, &archive, &restore, &delete] {
        button.set_hexpand(true);
        more_actions.append(button);
    }
    tools.append(&more_actions);
    let tools_scroll = gtk::ScrolledWindow::new();
    tools_scroll.set_hscrollbar_policy(gtk::PolicyType::Never);
    tools_scroll.set_vscrollbar_policy(gtk::PolicyType::Automatic);
    tools_scroll.set_propagate_natural_height(true);
    tools_scroll.set_max_content_height(420);
    tools_scroll.set_child(Some(&tools));
    let tools_popover = gtk::Popover::new();
    tools_popover.set_child(Some(&tools_scroll));
    more.set_popover(Some(&tools_popover));
    actions.add_css_class("note-actions");
    actions.append(&pin);
    actions.append(&popout);
    actions.append(&more);
    editor.append(&actions);
    pages.add_named(&editor, Some("editor"));
    let empty = gtk::Box::new(gtk::Orientation::Vertical, 12);
    empty.set_valign(gtk::Align::Center);
    empty.set_halign(gtk::Align::Center);
    let empty_title = gtk::Label::new(None);
    empty_title.add_css_class("empty-title");
    empty_title.set_wrap(true);
    let empty_text = gtk::Label::new(None);
    empty_text.add_css_class("metadata");
    empty_text.set_wrap(true);
    let empty_action = gtk::Button::new();
    empty_action.add_css_class("empty-action");
    empty_action.set_halign(gtk::Align::Center);
    empty.append(&empty_title);
    empty.append(&empty_text);
    empty.append(&empty_action);
    pages.add_named(&empty, Some("empty"));
    content.append(&pages);
    let drawer = adw::OverlaySplitView::new();
    drawer.set_sidebar_position(gtk::PackType::End);
    drawer.set_min_sidebar_width(260.);
    drawer.set_max_sidebar_width(260.);
    drawer.set_sidebar_width_unit(adw::LengthUnit::Px);
    drawer.set_pin_sidebar(true);
    let workspace = gtk::Box::new(gtk::Orientation::Vertical, 0);
    workspace.append(&filter_row);
    workspace.append(&content);
    drawer.set_content(Some(&workspace));
    drawer.set_show_sidebar(false);
    drawer.set_vexpand(true);
    let appearance_drawer = gtk::Box::new(gtk::Orientation::Vertical, 0);
    appearance_drawer.add_css_class("appearance-drawer");
    let appearance_header = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    appearance_header.add_css_class("appearance-header");
    appearance_header.set_height_request(44);
    let appearance_title = gtk::Label::new(Some("Settings"));
    appearance_title.set_xalign(0.);
    appearance_title.set_hexpand(true);
    appearance_header.append(&appearance_title);
    appearance_drawer.append(&appearance_header);
    let change = callbacks.settings.clone();
    let panel = crate::settings::build(config.clone(), move |c| change(c));
    panel.root.set_vexpand(true);
    appearance_drawer.append(&panel.root);
    drawer.set_sidebar(Some(&appearance_drawer));
    drawer.connect_show_sidebar_notify(|drawer| {
        if drawer.shows_sidebar() && drawer.is_collapsed() {
            // Allocate the native scrim again after it becomes visible, including without animation.
            let weak = drawer.downgrade();
            gtk::glib::idle_add_local_once(move || {
                if let Some(drawer) = weak.upgrade() {
                    drawer.queue_allocate();
                }
            });
        }
    });
    root.append(&drawer);
    let breakpoint =
        adw::Breakpoint::new(adw::BreakpointCondition::parse("max-width: 979px").unwrap());
    breakpoint.add_setter(&drawer, "collapsed", Some(&true.to_value()));
    window.add_breakpoint(breakpoint);
    let toast = adw::ToastOverlay::new();
    toast.set_child(Some(&root));
    window.set_content(Some(&toast));
    let font = gtk::CssProvider::new();
    for w in [title.upcast_ref::<gtk::Widget>(), body.upcast_ref()] {
        w.style_context()
            .add_provider(&font, gtk::STYLE_PROVIDER_PRIORITY_APPLICATION + 2);
    }
    let sort = callbacks.sort;
    let ui = Rc::new(ManagerUi {
        window,
        settings_panel: panel,
        storage,
        callbacks,
        config,
        collection: Cell::new(NoteCollection::All),
        sort: Cell::new(sort),
        selected: Cell::new(None),
        busy: Cell::new(false),
        notes: RefCell::new(Vec::new()),
        session: RefCell::new(None),
        subscription: RefCell::new(None),
        list,
        search,
        title,
        body,
        pin,
        popout,
        more,
        controls,
        pattern_choice,
        edit_actions,
        syncing_style: Cell::new(false),
        last_error: RefCell::new(None),
        error_toast: RefCell::new(None),
        archive,
        restore,
        dates,
        save_state,
        save_indicator,
        filters,
        pages,
        empty_title,
        empty_text,
        empty_action,
        toast,
        resolved,
        color,
        seed,
        background,
        font,
    });
    let weak = Rc::downgrade(&ui);
    ui.list.connect_row_selected(move |_, row| {
        if let Some(ui) = weak.upgrade() {
            ui.select(row);
        }
    });
    let weak = Rc::downgrade(&ui);
    ui.search.connect_search_changed(move |_| {
        if let Some(ui) = weak.upgrade() {
            ui.refresh(None);
        }
    });
    let weak = Rc::downgrade(&ui);
    new.connect_clicked(move |_| {
        if let Some(ui) = weak.upgrade() {
            ui.create();
        }
    });
    for (button, collection) in &ui.filters {
        let weak = Rc::downgrade(&ui);
        let collection = *collection;
        button.connect_clicked(move |_| {
            if let Some(ui) = weak.upgrade()
                && ui.flush()
            {
                ui.collection.set(collection);
                ui.refresh(None);
            }
        });
    }
    for (button, sort) in [
        (&recent, NoteSort::Recent),
        (&alphabetical, NoteSort::Title),
    ] {
        let weak = Rc::downgrade(&ui);
        let menu = sort_button.downgrade();
        button.connect_clicked(move |_| {
            if let Some(ui) = weak.upgrade() {
                ui.sort.set(sort);
                (ui.callbacks.sorted)(sort);
                ui.refresh(None);
                if let Some(menu) = menu.upgrade() {
                    menu.popdown();
                }
            }
        });
    }
    let weak = drawer.downgrade();
    let settings_ui = Rc::downgrade(&ui);
    settings.connect_clicked(move |_| {
        if let Some(ui) = settings_ui.upgrade() {
            ui.settings_panel.refresh();
        }
        if let Some(d) = weak.upgrade() {
            d.set_show_sidebar(!d.shows_sidebar());
        }
    });
    let weak = ui.window.downgrade();
    close.connect_clicked(move |_| {
        if let Some(w) = weak.upgrade() {
            w.close();
        }
    });
    for (button, change) in [
        (&ui.archive, ManagerChange::Archive),
        (&ui.restore, ManagerChange::Restore),
        (&delete, ManagerChange::Delete),
    ] {
        let weak = Rc::downgrade(&ui);
        button.connect_clicked(move |_| {
            if let Some(ui) = weak.upgrade() {
                ui.more.popdown();
                ui.mutate(change);
            }
        });
    }
    let weak = Rc::downgrade(&ui);
    ui.pin.connect_clicked(move |_| {
        if let Some(ui) = weak.upgrade()
            && !ui.busy.get()
        {
            ui.mutate(ManagerChange::Pin);
        }
    });
    let weak = Rc::downgrade(&ui);
    ui.popout.connect_clicked(move |_| {
        if let Some(ui) = weak.upgrade()
            && let Some(id) = ui.selected.get()
        {
            (ui.callbacks.popout)(id);
        }
    });
    let weak = Rc::downgrade(&ui);
    copy.connect_clicked(move |_| {
        if let Some(ui) = weak.upgrade() {
            let b = ui.body.buffer();
            let (s, e) = b.bounds();
            ui.body.clipboard().set_text(&b.text(&s, &e, true));
            ui.more.popdown();
        }
    });
    for (button, prefix) in [(checklist, "- [ ] "), (bullet, "- "), (numbered, "1. ")] {
        let weak = Rc::downgrade(&ui);
        button.connect_clicked(move |_| {
            if let Some(ui) = weak.upgrade() {
                if ui.body.is_editable() {
                    crate::editor::toggle_line(&ui.body, prefix);
                    ui.body.grab_focus();
                }
                ui.more.popdown();
            }
        });
    }
    for (button, size) in &ui.controls.sizes {
        let weak = Rc::downgrade(&ui);
        let size = *size;
        button.connect_toggled(move |button| {
            if let Some(ui) = weak.upgrade()
                && !ui.syncing_style.get()
                && button.is_active()
            {
                let session = ui.session.borrow().clone();
                if let Some(session) = session {
                    let mut style = session.style();
                    style.text_size = size;
                    session.set_style(style);
                }
            }
        });
    }
    for (button, weight) in &ui.controls.weights {
        let weak = Rc::downgrade(&ui);
        let weight = *weight;
        button.connect_toggled(move |button| {
            if let Some(ui) = weak.upgrade()
                && !ui.syncing_style.get()
                && button.is_active()
            {
                let session = ui.session.borrow().clone();
                if let Some(session) = session {
                    let mut style = session.style();
                    style.text_weight = weight;
                    session.set_style(style);
                }
            }
        });
    }
    for (button, color) in &ui.controls.colors {
        let weak = Rc::downgrade(&ui);
        let color = *color;
        button.connect_clicked(move |_| {
            if let Some(ui) = weak.upgrade() {
                let session = ui.session.borrow().clone();
                if let Some(session) = session {
                    session.set_color(color);
                }
            }
        });
    }
    let weak = Rc::downgrade(&ui);
    ui.pattern_choice.connect_selected_notify(move |choice| {
        if let Some(ui) = weak.upgrade()
            && !ui.syncing_style.get()
        {
            let session = ui.session.borrow().clone();
            if let Some(session) = session {
                let mut style = session.style();
                style.paper = [
                    None,
                    Some(Paper::Plain),
                    Some(Paper::Dots),
                    Some(Paper::Grid),
                    Some(Paper::Ruled),
                ]
                .get(choice.selected() as usize)
                .copied()
                .flatten();
                session.set_style(style);
            }
        }
    });
    let keys = gtk::EventControllerKey::new();
    let weak = Rc::downgrade(&ui);
    keys.connect_key_pressed(move |_, key, _, modifiers| {
        if let Some(ui) = weak.upgrade()
            && key == gtk::gdk::Key::Return
            && !modifiers.contains(gtk::gdk::ModifierType::SHIFT_MASK)
            && ui.body.is_editable()
            && crate::editor::continue_list(&ui.body)
        {
            return gtk::glib::Propagation::Stop;
        }
        gtk::glib::Propagation::Proceed
    });
    ui.body.add_controller(keys);
    let weak = Rc::downgrade(&ui);
    ui.empty_action.connect_clicked(move |_| {
        if let Some(ui) = weak.upgrade() {
            if !ui.notes.borrow().is_empty() {
                ui.create();
            } else if !ui.search.text().is_empty() {
                ui.search.set_text("");
                ui.refresh(None);
            } else if ui.collection.get() == NoteCollection::Archived {
                ui.collection.set(NoteCollection::Active);
                ui.refresh(None);
            } else {
                ui.create();
            }
        }
    });
    ui.refresh(None);
    ui.sync_editor();
    ManagerWindow(ui)
}
fn menu_button(icon: &str, label: &str) -> gtk::MenuButton {
    let b = gtk::MenuButton::builder()
        .child(&ui_assets::image(icon))
        .tooltip_text(label)
        .build();
    b.set_valign(gtk::Align::Center);
    b.add_css_class("icon-control");
    b
}
fn set_menu(menu: &gtk::MenuButton, buttons: &[&gtk::Button]) {
    let popover = gtk::Popover::new();
    let box_ = gtk::Box::new(gtk::Orientation::Vertical, 2);
    for button in buttons {
        box_.append(*button);
    }
    popover.set_child(Some(&box_));
    menu.set_popover(Some(&popover));
}
fn note_row(note: &Note, archived: bool, updated: Option<i64>) -> gtk::ListBoxRow {
    let row = gtk::ListBoxRow::new();
    appearance::set_color(&row, note.color);
    let content = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    content.add_css_class("row-content");
    let bar = gtk::Box::new(gtk::Orientation::Vertical, 0);
    bar.add_css_class("tag-bar");
    content.append(&bar);
    let text = gtk::Box::new(gtk::Orientation::Vertical, 5);
    text.set_valign(gtk::Align::Center);
    text.set_hexpand(true);
    let title = gtk::Label::new(Some(if note.title.is_empty() {
        "Untitled note"
    } else {
        &note.title
    }));
    title.add_css_class("row-title");
    let snippet = gtk::Label::new(Some(&note.body.lines().collect::<Vec<_>>().join(" · ")));
    snippet.add_css_class("row-snippet");
    for l in [&title, &snippet] {
        l.set_xalign(0.);
        l.set_ellipsize(gtk::pango::EllipsizeMode::End);
        l.set_width_chars(1);
        l.set_max_width_chars(28);
    }
    let heading = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    title.set_hexpand(true);
    heading.append(&title);
    let date = gtk::Label::new(Some(&short_date(updated)));
    date.add_css_class("row-date");
    date.set_tooltip_text(Some("Last updated"));
    heading.append(&date);
    if archived {
        let badge = ui_assets::image("archive");
        badge.set_tooltip_text(Some("Archived"));
        badge.add_css_class("archive-badge");
        heading.append(&badge);
    }
    let pin = ui_assets::image("pin");
    pin.set_tooltip_text(Some("Pinned"));
    pin.set_visible(note.pinned);
    heading.append(&pin);
    text.append(&heading);
    text.append(&snippet);
    content.append(&text);
    row.set_child(Some(&content));
    row
}

fn short_date(millis: Option<i64>) -> String {
    millis
        .and_then(|ms| gtk::glib::DateTime::from_unix_local(ms / 1000).ok())
        .and_then(|date| date.format("%b %-d").ok())
        .map(|date| date.to_string())
        .unwrap_or_default()
}
