mod app;
mod appearance;
mod autostart;
mod config;
mod editor;
mod manager;
mod note_controls;
mod note_session;
mod platform;
mod settings;
mod storage;
mod ui_assets;

use app::{Action, AppState, NoteColor, NoteId};
use config::{Config, ScreenEdge};
use gtk::{glib, prelude::*};
use gtk4_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};
use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    rc::{Rc, Weak},
    time::Duration,
};
use storage::{NoteCollection, NoteSort, Storage};

const APPLICATION_ID: &str = "dev.stickies.Stickies";
const WIDTH: i32 = 420;
const HEIGHT: i32 = 500;

struct SeparateNote {
    window: gtk::ApplicationWindow,
    editor: Rc<editor::Editor>,
}

struct Ui {
    application: gtk::Application,
    window: gtk::ApplicationWindow,
    root: gtk::Overlay,
    strip_window: gtk::ApplicationWindow,
    strip_root: gtk::Overlay,
    strip_offset: Cell<Option<i32>>,
    horizontal_tab_width: Cell<i32>,
    strip_bounds: Cell<(i32, i32, i32, i32)>,
    panel_bounds: Cell<(i32, i32, i32, i32)>,
    stack: gtk::Stack,
    marker: gtk::Box,
    tabs: gtk::Box,
    edge_sort: Cell<NoteSort>,
    edge_search: gtk::SearchEntry,
    search_results: gtk::ListBox,
    search_ids: RefCell<Vec<NoteId>>,
    search_create: gtk::Button,
    searching: Cell<bool>,
    hovering: Cell<bool>,
    pointer: gtk::DrawingArea,
    pointer_origin: Cell<Option<i32>>,
    config: Rc<RefCell<Config>>,
    config_valid: bool,
    system_animations: bool,
    dnd: Cell<bool>,
    dnd_monitor: RefCell<Option<gtk::gio::FileMonitor>>,
    provider: gtk::CssProvider,
    storage: Rc<Storage>,
    sessions: RefCell<HashMap<NoteId, Weak<note_session::NoteSession>>>,
    docked: RefCell<Option<Rc<editor::Editor>>>,
    docked_offset: Cell<Option<(ScreenEdge, i32)>>,
    deck: RefCell<AppState>,
    separate: RefCell<HashMap<NoteId, SeparateNote>>,
    manager: RefCell<Option<manager::ManagerWindow>>,
    revealed: Cell<bool>,
    generation: Cell<u64>,
}

impl Ui {
    fn refresh(self: &Rc<Self>) {
        if let Some(manager) = self.manager.borrow().as_ref() {
            manager.refresh();
        }
        self.refresh_tabs();
    }
    fn refresh_tabs(self: &Rc<Self>) {
        while let Some(child) = self.tabs.first_child() {
            self.tabs.remove(&child);
        }
        while let Some(child) = self.marker.first_child() {
            self.marker.remove(&child);
        }
        let config = self.config.borrow().clone();
        let notes = match self.storage.load_deck_notes(4, self.edge_sort.get()) {
            Ok(notes) => notes,
            Err(error) => {
                self.error(&format!("Could not load edge notes: {error}"));
                return;
            }
        };
        self.deck.borrow_mut().replace_notes(notes);
        let notes: Vec<_> = self.deck.borrow().notes().iter().take(4).cloned().collect();
        let horizontal = matches!(config.edge, ScreenEdge::Top | ScreenEdge::Bottom);
        let orientation = if horizontal {
            gtk::Orientation::Horizontal
        } else {
            gtk::Orientation::Vertical
        };
        self.tabs.set_orientation(orientation);
        self.marker.set_orientation(orientation);
        // Measure the upright titles, but cap their contribution so a long title
        // cannot widen the whole strip. Vertical tabs retain their reviewed size.
        let tab_width = if horizontal {
            let title_width = notes
                .iter()
                .map(|note| {
                    let layout = self
                        .tabs
                        .create_pango_layout(Some(&note.title.to_uppercase()));
                    layout.set_font_description(Some(&gtk::pango::FontDescription::from_string(
                        "Inter Semi-Bold 11px",
                    )));
                    layout.pixel_size().0
                })
                .max()
                .unwrap_or(0);
            let (screen_width, _) = self.note_limits();
            let available = ((screen_width - 150) / 4).max(34);
            let width = if self.revealed.get() && self.horizontal_tab_width.get() > 0 {
                self.horizontal_tab_width.get()
            } else {
                (title_width + 16).clamp(80, 112)
            }
            .min(available);
            self.horizontal_tab_width.set(width);
            width
        } else {
            self.horizontal_tab_width.set(0);
            34
        };
        let tab_height = if horizontal { 34 } else { 80 };
        let fixed = gtk::Fixed::new();
        fixed.set_size_request(
            if horizontal {
                (notes.len() as i32 * (tab_width + 4) - 4).max(0)
            } else {
                tab_width
            },
            if horizontal {
                tab_height
            } else {
                (notes.len() as i32 * (tab_height + 4) - 4).max(0)
            },
        );
        for (index, note) in notes.iter().enumerate() {
            let dot = gtk::Box::new(gtk::Orientation::Vertical, 0);
            dot.add_css_class("edge-pill");
            appearance::set_color(&dot, note.color);
            self.marker.append(&dot);
            let tab = gtk::Button::new();
            tab.add_css_class("note-tab");
            tab.set_widget_name(&format!("note-{:?}", note.id));
            appearance::set_color(&tab, note.color);
            let title = if note.title.trim().is_empty() {
                "Untitled note"
            } else {
                &note.title
            };
            tab.set_tooltip_text(Some(title));
            let label = gtk::Label::new(Some(&title.to_uppercase()));
            label.add_css_class("tab-title");
            label.set_ellipsize(gtk::pango::EllipsizeMode::End);
            label.set_width_chars(1);
            label.set_max_width_chars(10);
            label.set_xalign(0.5);
            if horizontal {
                label.set_max_width_chars(1);
                tab.set_child(Some(&label));
            } else {
                label.set_size_request(72, 20);
                let text = gtk::Fixed::new();
                text.set_size_request(30, 78);
                text.put(&label, 0., 0.);
                let transform = if config.edge == ScreenEdge::Right {
                    gtk::gsk::Transform::new()
                        .translate(&gtk::graphene::Point::new(25., 3.))
                        .rotate(90.)
                } else {
                    gtk::gsk::Transform::new()
                        .translate(&gtk::graphene::Point::new(5., 75.))
                        .rotate(-90.)
                };
                text.set_child_transform(&label, Some(&transform));
                tab.set_child(Some(&text));
            }
            tab.set_size_request(tab_width, tab_height);
            let (x, y) = if horizontal {
                (index as f64 * f64::from(tab_width + 4), 0.)
            } else {
                (0., index as f64 * f64::from(tab_height + 4))
            };
            fixed.put(&tab, x, y);
            let id = note.id;
            let weak = Rc::downgrade(self);
            tab.connect_clicked(move |tab| {
                if let Some(ui) = weak.upgrade() {
                    let editor = ui.docked.borrow().clone();
                    if editor.is_some_and(|editor| !editor.flush()) {
                        return;
                    }
                    ui.remember_origin(tab.upcast_ref());
                    ui.open_docked(id);
                }
            });
        }
        if notes.is_empty() {
            let dot = gtk::Box::new(gtk::Orientation::Vertical, 0);
            dot.add_css_class("edge-pill");
            appearance::set_color(&dot, NoteColor::Yellow);
            self.marker.append(&dot);
        }
        if !notes.is_empty() {
            self.tabs.append(&fixed);
        }
        let utilities = gtk::Box::new(orientation, 4);
        utilities.add_css_class("edge-utilities");
        for (name, label) in [
            ("plus", "New note"),
            ("search", "Search notes"),
            ("grid", "All Notes"),
        ] {
            let button = ui_assets::icon(name, label);
            button.add_css_class("strip-control");
            button.set_size_request(34, if horizontal { 34 } else { 28 });
            let weak = Rc::downgrade(self);
            button.connect_clicked(move |button| {
                if let Some(ui) = weak.upgrade() {
                    let editor = ui.docked.borrow().clone();
                    if editor.is_some_and(|editor| !editor.flush()) {
                        return;
                    }
                    match name {
                        "plus" => {
                            ui.remember_origin(button.upcast_ref());
                            ui.create_note();
                        }
                        "search" => {
                            ui.remember_origin(button.upcast_ref());
                            ui.show_edge_search();
                        }
                        _ => ui.show_manager(),
                    }
                }
            });
            utilities.append(&button);
        }
        self.tabs.append(&utilities);
        ui_assets::refresh_icons(
            self.tabs.upcast_ref(),
            config.theme == config::Theme::Playful,
            false,
        );
        if self.searching.get() {
            self.refresh_edge_search();
        }
        self.place_edge();
    }
    fn remember_origin(&self, widget: &gtk::Widget) {
        if let Some(bounds) = widget.compute_bounds(&self.strip_root) {
            let horizontal = matches!(
                self.config.borrow().edge,
                ScreenEdge::Top | ScreenEdge::Bottom
            );
            self.pointer_origin.set(Some(if horizontal {
                (bounds.x() + bounds.width() / 2.) as i32
            } else {
                (bounds.y() + bounds.height() / 2.) as i32
            }));
        }
    }
    fn show_edge_search(self: &Rc<Self>) {
        let origin = self.pointer_origin.get();
        if !self.close_docked() {
            return;
        }
        self.pointer_origin.set(origin);
        self.reveal();
        self.searching.set(true);
        self.stack.set_visible(true);
        self.stack.set_visible_child_name("search");
        self.edge_search.set_text("");
        self.refresh_edge_search();
        self.place_edge();
        if !self.window.is_active() {
            self.window.set_keyboard_mode(KeyboardMode::Exclusive);
        }
        self.edge_search.grab_focus();
    }
    fn close_edge_search(&self) {
        self.searching.set(false);
        self.pointer_origin.set(None);
        self.stack.set_visible(false);
        self.place_edge();
    }
    fn refresh_edge_search(self: &Rc<Self>) {
        let query = self.edge_search.text();
        let notes = match self.storage.search_notes(
            NoteCollection::Active,
            query.as_str(),
            self.edge_sort.get(),
        ) {
            Ok(notes) => notes,
            Err(e) => {
                self.error(&format!("Could not search notes: {e}"));
                return;
            }
        };
        let previous = self
            .search_results
            .selected_row()
            .and_then(|row| self.search_ids.borrow().get(row.index() as usize).copied());
        while let Some(child) = self.search_results.first_child() {
            self.search_results.remove(&child);
        }
        let mut ids = Vec::new();
        for note in &notes {
            let row = gtk::Box::new(gtk::Orientation::Horizontal, 10);
            let dot = gtk::Box::new(gtk::Orientation::Vertical, 0);
            dot.add_css_class("edge-pill");
            dot.set_valign(gtk::Align::Center);
            appearance::set_color(&dot, note.color);
            let title = gtk::Label::new(Some(if note.title.trim().is_empty() {
                "Untitled note"
            } else {
                &note.title
            }));
            title.add_css_class("edge-result-title");
            title.set_xalign(0.);
            title.set_ellipsize(gtk::pango::EllipsizeMode::End);
            title.set_width_chars(1);
            title.set_hexpand(true);
            row.append(&dot);
            row.append(&title);
            self.search_results.append(&row);
            ids.push(note.id);
        }
        let selected = previous
            .and_then(|id| ids.iter().position(|i| *i == id))
            .unwrap_or(0);
        self.search_ids.replace(ids);
        self.search_results
            .select_row(self.search_results.row_at_index(selected as i32).as_ref());
        self.search_create.set_visible(notes.is_empty());
        self.search_create.set_sensitive(!query.trim().is_empty());
        self.search_create.set_label(if query.trim().is_empty() {
            "No active notes"
        } else {
            "No matches. Create a note from this search"
        });
    }
    fn activate_edge_search(self: &Rc<Self>) {
        self.refresh_edge_search();
        let id = self
            .search_results
            .selected_row()
            .and_then(|row| self.search_ids.borrow().get(row.index() as usize).copied());
        if let Some(id) = id {
            self.open_docked(id);
        } else {
            let title = self.edge_search.text();
            if !title.trim().is_empty() {
                self.pointer_origin.set(None);
                self.create_note_with_title(title.trim());
            }
        }
    }
    fn schedule_reveal(self: &Rc<Self>) {
        self.hovering.set(true);
        let generation = self.generation.get().wrapping_add(1);
        self.generation.set(generation);
        let weak = Rc::downgrade(self);
        glib::timeout_add_local_once(Duration::from_millis(140), move || {
            if let Some(ui) = weak.upgrade()
                && ui.hovering.get()
                && ui.generation.get() == generation
                && ui.config.borrow().edge_hover_enabled
                && !ui.revealed.get()
            {
                ui.reveal();
            }
        });
    }
    fn reveal(&self) {
        if self.docked.borrow().is_some() {
            return;
        }
        self.generation.set(self.generation.get().wrapping_add(1));
        self.deck.borrow_mut().dispatch(Action::ShowTabs);
        self.revealed.set(true);
        self.marker.set_visible(false);
        self.tabs.set_visible(true);
        self.place_edge();
        self.set_input();
    }
    fn dormant(&self) {
        if self.docked.borrow().is_some() {
            return;
        }
        self.searching.set(false);
        self.deck.borrow_mut().dispatch(Action::CollapseDeck);
        self.revealed.set(false);
        self.strip_offset.set(None);
        self.tabs.set_visible(false);
        self.marker.set_visible(true);
        self.stack.set_visible(false);
        self.strip_window.set_keyboard_mode(KeyboardMode::None);
        self.place_edge();
        self.set_input();
    }
    fn schedule_hide(self: &Rc<Self>) {
        self.hovering.set(false);
        let generation = self.generation.get().wrapping_add(1);
        self.generation.set(generation);
        let weak = Rc::downgrade(self);
        glib::timeout_add_local_once(Duration::from_millis(350), move || {
            if let Some(ui) = weak.upgrade()
                && ui.generation.get() == generation
                && !(ui.searching.get() && (ui.window.is_active() || ui.strip_window.is_active()))
            {
                ui.dormant();
            }
        });
    }
    fn create_note(self: &Rc<Self>) {
        self.create_note_with_title("");
    }
    fn create_note_with_title(self: &Rc<Self>, title: &str) {
        if !self.close_docked() {
            return;
        }
        match self.storage.create_note(title, "", NoteColor::Yellow) {
            Ok(note) => {
                self.deck
                    .borrow_mut()
                    .dispatch(Action::AddNote(note.clone()));
                self.refresh();
                self.open_docked(note.id);
                if let Some(editor) = self.docked.borrow().as_ref() {
                    editor.title.grab_focus();
                }
            }
            Err(error) => self.error(&format!("Could not create a note: {error}")),
        }
    }
    fn note_session(self: &Rc<Self>, id: NoteId) -> Option<Rc<note_session::NoteSession>> {
        if let Some(session) = self.sessions.borrow().get(&id).and_then(Weak::upgrade) {
            return Some(session);
        }
        let note = match self.storage.load_note(id) {
            Ok(note) => note,
            Err(error) => {
                self.error(&format!("Could not open note: {error}"));
                return None;
            }
        };
        let saved = Rc::downgrade(self);
        let session = note_session::NoteSession::new(note, self.storage.clone(), move || {
            if let Some(ui) = saved.upgrade() {
                ui.refresh();
            }
        });
        let mut sessions = self.sessions.borrow_mut();
        sessions.retain(|_, session| session.strong_count() > 0);
        sessions.insert(id, Rc::downgrade(&session));
        Some(session)
    }
    fn new_editor(self: &Rc<Self>, id: NoteId, detached: bool) -> Option<Rc<editor::Editor>> {
        let session = self.note_session(id)?;
        let timeout = Rc::downgrade(self);
        let editor = editor::Editor::new(session, self.config.clone(), move || {
            if let Some(ui) = timeout.upgrade() {
                if detached {
                    ui.close_separate(id);
                } else {
                    ui.close_docked();
                }
            }
        });
        let weak = Rc::downgrade(self);
        editor.close.connect_clicked(move |_| {
            if let Some(ui) = weak.upgrade() {
                if detached {
                    ui.close_separate(id);
                } else {
                    if ui.close_docked() && !ui.strip_window.is_active() {
                        ui.strip_window.set_keyboard_mode(KeyboardMode::Exclusive);
                    }
                }
            }
        });
        editor.separate.set_visible(!detached);
        let weak = Rc::downgrade(self);
        editor.separate.connect_clicked(move |_| {
            if let Some(ui) = weak.upgrade() {
                ui.open_separate(id);
            }
        });
        if !detached {
            editor.set_docked_size(editor::WIDTH, editor::HEIGHT);
            let drag = gtk::GestureDrag::new();
            drag.set_button(1);
            drag.set_propagation_phase(gtk::PropagationPhase::Capture);
            let start = Rc::new(Cell::new((editor::WIDTH, editor::HEIGHT)));
            let pointer_start = Rc::new(Cell::new((0., 0.)));
            let initial = start.clone();
            let initial_pointer = pointer_start.clone();
            let weak = Rc::downgrade(self);
            let weak_editor = Rc::downgrade(&editor);
            drag.connect_drag_begin(move |gesture, _, _| {
                gesture.set_state(gtk::EventSequenceState::Claimed);
                if let Some(editor) = weak_editor.upgrade()
                    && let Some(ui) = weak.upgrade()
                    && let Some(pointer) = ui.docked_pointer(gesture)
                {
                    initial.set(editor.docked_size());
                    initial_pointer.set(pointer);
                    editor.activity();
                }
            });
            let weak = Rc::downgrade(self);
            let weak_editor = Rc::downgrade(&editor);
            drag.connect_drag_update(move |gesture, _, _| {
                if let Some(ui) = weak.upgrade()
                    && let Some(editor) = weak_editor.upgrade()
                    && let Some((x, y)) = ui.docked_pointer(gesture)
                {
                    let (start_x, start_y) = pointer_start.get();
                    let (dx, dy) = (x - start_x, y - start_y);
                    let edge = ui.config.borrow().edge;
                    let (width, height) = start.get();
                    let dx = if edge == ScreenEdge::Right { -dx } else { dx };
                    let dy = if edge == ScreenEdge::Bottom { -dy } else { dy };
                    let (max_width, max_height) = ui.note_limits();
                    editor.set_docked_size(
                        (width + dx as i32).clamp(editor::MIN_WIDTH, max_width),
                        (height + dy as i32).clamp(editor::MIN_HEIGHT, max_height),
                    );
                    ui.place_edge();
                }
            });
            editor.resize.add_controller(drag);
            let weak = Rc::downgrade(self);
            editor.connect_size_changed(move || {
                if let Some(ui) = weak.upgrade() {
                    ui.set_input();
                }
            });
        }
        Some(editor)
    }
    fn open_docked(self: &Rc<Self>, id: NoteId) {
        if self
            .docked
            .borrow()
            .as_ref()
            .is_some_and(|editor| editor.id() == id)
        {
            return;
        }
        let origin = self.pointer_origin.get();
        if !self.close_docked() {
            return;
        }
        self.pointer_origin.set(origin);
        if let Some(note) = self.separate.borrow().get(&id) {
            self.close_edge_search();
            note.window.present();
            return;
        }
        self.reveal();
        let Some(editor) = self.new_editor(id, false) else {
            return;
        };
        // Search can open an active note outside the four visible slots.
        if self.deck.borrow().note(id).is_none()
            && let Ok(note) = self.storage.load_note(id)
        {
            self.deck.borrow_mut().dispatch(Action::AddNote(note));
        }
        self.deck.borrow_mut().dispatch(Action::OpenNote(id));
        self.searching.set(false);
        self.stack.add_named(&editor.root, Some("editor"));
        self.marker.set_visible(false);
        self.window.set_keyboard_mode(KeyboardMode::OnDemand);
        self.docked.replace(Some(editor.clone()));
        self.stack.set_visible_child_name("editor");
        self.stack.set_visible(true);
        self.pointer.set_visible(true);
        self.pointer.queue_draw();
        self.docked_offset.set(None);
        self.window.set_layer(Layer::Overlay);
        self.place_edge();
        self.update_visibility();
        self.set_input();
        if !self.window.is_active() {
            self.window.set_keyboard_mode(KeyboardMode::Exclusive);
        }
        editor.body.grab_focus();
    }
    fn close_docked(self: &Rc<Self>) -> bool {
        let editor = self.docked.borrow().clone();
        if let Some(editor) = editor {
            if !editor.flush() {
                return false;
            }
            editor.stop();
            self.docked.borrow_mut().take();
            self.pointer.set_visible(false);
            self.pointer_origin.set(None);
            self.docked_offset.set(None);
            self.deck.borrow_mut().dispatch(Action::CollapseDeck);
            self.revealed.set(true);
            self.searching.set(false);
            self.marker.set_visible(false);
            self.stack.set_visible(false);
            self.stack.remove(&editor.root);
            self.refresh_tabs();
            self.reveal();
            self.update_visibility();
        }
        true
    }
    fn open_separate(self: &Rc<Self>, id: NoteId) {
        if let Some(note) = self.separate.borrow().get(&id) {
            note.window.present();
            return;
        }
        let is_docked = self.docked.borrow().as_ref().is_some_and(|e| e.id() == id);
        if is_docked && !self.close_docked() {
            return;
        }
        let Some(editor) = self.new_editor(id, true) else {
            return;
        };
        let window = gtk::ApplicationWindow::builder()
            .application(&self.application)
            .title("Stickies · Note")
            .default_width(editor::WIDTH)
            .default_height(editor::HEIGHT)
            .decorated(false)
            .resizable(false)
            .build();
        window.add_css_class("note-window");
        window.set_child(Some(&editor.root));
        // Present as a fixed-size note first so the compositor keeps it floating.
        // Allow resizing once the mapped surface enters its initial monitor.
        window.connect_realize(|window| {
            if let Some(surface) = window.surface() {
                let weak = window.downgrade();
                surface.connect_enter_monitor(move |_, _| {
                    if let Some(window) = weak.upgrade() {
                        window.set_resizable(true);
                    }
                });
            }
        });
        let resize = gtk::GestureClick::new();
        resize.set_button(1);
        resize.set_propagation_phase(gtk::PropagationPhase::Capture);
        let weak = window.downgrade();
        resize.connect_pressed(move |gesture, _, _, _| {
            if let Some(window) = weak.upgrade()
                && let Some(surface) = window.surface()
                && let Ok(toplevel) = surface.downcast::<gtk::gdk::Toplevel>()
                && let Some(event) = gesture.current_event()
                && let Some((x, y)) = event.position()
            {
                toplevel.begin_resize(
                    gtk::gdk::SurfaceEdge::SouthEast,
                    gesture.current_event_device().as_ref(),
                    1,
                    x,
                    y,
                    gesture.current_event_time(),
                );
                gesture.set_state(gtk::EventSequenceState::Claimed);
            }
        });
        editor.resize.add_controller(resize);
        let weak = Rc::downgrade(self);
        window.connect_close_request(move |window| {
            if let Some(ui) = weak.upgrade() {
                let id = ui
                    .separate
                    .borrow()
                    .iter()
                    .find(|(_, note)| note.window == *window)
                    .map(|(id, _)| *id);
                if let Some(id) = id {
                    let editor = ui.separate.borrow().get(&id).map(|n| n.editor.clone());
                    if let Some(editor) = editor {
                        if !editor.flush() {
                            return glib::Propagation::Stop;
                        }
                        editor.stop();
                    }
                    ui.separate.borrow_mut().remove(&id);
                }
            }
            glib::Propagation::Proceed
        });
        {
            let keys = gtk::EventControllerKey::new();
            keys.set_propagation_phase(gtk::PropagationPhase::Capture);
            let weak = window.downgrade();
            keys.connect_key_pressed(move |_, key, _, _| {
                if key == gtk::gdk::Key::Escape {
                    if let Some(window) = weak.upgrade() {
                        window.close();
                    }
                    glib::Propagation::Stop
                } else {
                    glib::Propagation::Proceed
                }
            });
            window.add_controller(keys);
        }
        self.separate.borrow_mut().insert(
            id,
            SeparateNote {
                window: window.clone(),
                editor: editor.clone(),
            },
        );
        window.present();
        editor.body.grab_focus();
    }
    fn close_separate(&self, id: NoteId) {
        let window = self
            .separate
            .borrow()
            .get(&id)
            .map(|note| note.window.clone());
        if let Some(window) = window {
            window.close();
        }
    }
    fn finish_change(self: &Rc<Self>, id: NoteId, change: manager::ManagerChange) {
        if change == manager::ManagerChange::Create {
            self.refresh_tabs();
            return;
        }
        if change == manager::ManagerChange::Pin {
            let session = self.sessions.borrow().get(&id).and_then(Weak::upgrade);
            if let Some(session) = session {
                session.sync_pin();
            }
        }
        if matches!(
            change,
            manager::ManagerChange::Archive | manager::ManagerChange::Delete
        ) {
            if self.docked.borrow().as_ref().is_some_and(|e| e.id() == id) {
                self.deck
                    .borrow_mut()
                    .dispatch(if change == manager::ManagerChange::Archive {
                        Action::ArchiveOpenNote
                    } else {
                        Action::DeleteOpenNote
                    });
                self.close_docked();
            }
            self.close_separate(id);
        }
        self.refresh();
    }
    fn show_manager(self: &Rc<Self>) {
        if let Some(manager) = self.manager.borrow().as_ref() {
            manager.present();
            return;
        }
        let sessions = Rc::downgrade(self);
        let after = Rc::downgrade(self);
        let edit = Rc::downgrade(self);
        let settings = Rc::downgrade(self);
        let sorted = Rc::downgrade(self);
        let manager = manager::build_window(
            &self.application,
            self.storage.clone(),
            self.config.clone(),
            manager::Callbacks {
                sort: self.edge_sort.get(),
                sorted: Box::new(move |sort| {
                    if let Some(ui) = sorted.upgrade() {
                        ui.edge_sort.set(sort);
                        ui.refresh_tabs();
                    }
                }),
                session: Box::new(move |id| sessions.upgrade().and_then(|ui| ui.note_session(id))),
                changed: Box::new(move |id, change| {
                    if let Some(ui) = after.upgrade() {
                        ui.finish_change(id, change);
                    }
                }),
                popout: Box::new(move |id| {
                    if let Some(ui) = edit.upgrade() {
                        ui.open_separate(id);
                    }
                }),
                settings: Rc::new(move |config| {
                    settings
                        .upgrade()
                        .ok_or_else(|| "Stickies is closing".to_owned())?
                        .change_settings(config)
                }),
            },
        );
        let weak = Rc::downgrade(self);
        manager.window().connect_close_request(move |_| {
            if let Some(ui) = weak.upgrade() {
                let manager = ui.manager.borrow().clone();
                if manager.is_some_and(|m| !m.flush()) {
                    return glib::Propagation::Stop;
                }
                ui.manager.borrow_mut().take();
            }
            glib::Propagation::Proceed
        });
        self.manager.replace(Some(manager.clone()));
        manager.present();
    }
    fn change_settings(self: &Rc<Self>, config: Config) -> Result<(), String> {
        if !self.config_valid {
            return Err("The existing configuration is invalid. Correct it before saving settings; it has not been overwritten.".into());
        }
        config
            .save(&config::config_path())
            .map_err(|e| e.to_string())?;
        let login_changed = self.config.borrow().launch_at_login != config.launch_at_login;
        if self.config.borrow().edge != config.edge
            || self.config.borrow().marker_offset != config.marker_offset
        {
            self.docked_offset.set(None);
            self.strip_offset.set(None);
        }
        self.config.replace(config.clone());
        if let Some(settings) = gtk::Settings::default() {
            settings.set_gtk_enable_animations(self.system_animations && !config.reduced_motion);
        }
        appearance::apply(&self.provider, &config);
        self.place_edge();
        self.update_visibility();
        self.refresh_tabs();
        if let Some(editor) = self.docked.borrow().as_ref() {
            editor.refresh_appearance();
        }
        for note in self.separate.borrow().values() {
            note.editor.refresh_appearance();
        }
        if let Some(manager) = self.manager.borrow().as_ref() {
            manager.refresh_appearance();
        }
        if login_changed {
            autostart::reconcile(&autostart::autostart_path(), config.launch_at_login).map_err(
                |e| format!("Settings saved, but launch at login could not be updated: {e}"),
            )?;
        }
        Ok(())
    }
    fn update_visibility(&self) {
        let transient_visible = !(self.config.borrow().hide_in_dnd && self.dnd.get());
        self.strip_window.set_visible(transient_visible);
        self.window
            .set_visible(self.docked.borrow().is_some() || transient_visible);
    }

    fn place_edge(&self) {
        let config = self.config.borrow();
        let (screen_width, screen_height) = self.note_limits();
        let horizontal = matches!(config.edge, ScreenEdge::Top | ScreenEdge::Bottom);
        // Both layer surfaces keep their origin and extent across every state.
        // Only child allocations and their disjoint input regions change.
        for (window, root) in [
            (&self.window, &self.root),
            (&self.strip_window, &self.strip_root),
        ] {
            window.set_anchor(Edge::Left, true);
            window.set_anchor(Edge::Top, true);
            window.set_anchor(Edge::Right, false);
            window.set_anchor(Edge::Bottom, false);
            root.set_size_request(screen_width, screen_height);
            window.set_default_size(screen_width, screen_height);
        }
        let measure = |widget: &gtk::Widget| {
            let visible = widget.is_visible();
            widget.set_visible(true);
            let (_, width, _, _) = widget.measure(gtk::Orientation::Horizontal, -1);
            let (_, height, _, _) = widget.measure(gtk::Orientation::Vertical, width);
            widget.set_visible(visible);
            (
                width - widget.margin_start() - widget.margin_end(),
                height - widget.margin_top() - widget.margin_bottom(),
            )
        };
        let (tw, th) = measure(self.tabs.upcast_ref());
        let available = if horizontal {
            screen_width - tw
        } else {
            screen_height - th
        }
        .max(0);
        let offset = self
            .strip_offset
            .get()
            .unwrap_or((f64::from(available) * config.marker_offset) as i32)
            .clamp(0, available);
        if self.revealed.get() {
            self.strip_offset.set(Some(offset));
        }
        let (tx, ty) = match config.edge {
            ScreenEdge::Left => (0, offset),
            ScreenEdge::Right => (screen_width - tw, offset),
            ScreenEdge::Top => (offset, 0),
            ScreenEdge::Bottom => (offset, screen_height - th),
        };
        let position = |widget: &gtk::Widget, x, y| {
            widget.set_halign(gtk::Align::Start);
            widget.set_valign(gtk::Align::Start);
            widget.set_margin_start(x);
            widget.set_margin_top(y);
        };
        // Measure without placement margins, which otherwise feed back into size.
        position(self.tabs.upcast_ref(), tx, ty);
        let (mw, mh) = measure(self.marker.upcast_ref());
        let (mx, my) = match config.edge {
            ScreenEdge::Left => (0, (offset + (th - mh) / 2).clamp(0, screen_height - mh)),
            ScreenEdge::Right => (
                screen_width - mw,
                (offset + (th - mh) / 2).clamp(0, screen_height - mh),
            ),
            ScreenEdge::Top => ((offset + (tw - mw) / 2).clamp(0, screen_width - mw), 0),
            ScreenEdge::Bottom => (
                (offset + (tw - mw) / 2).clamp(0, screen_width - mw),
                screen_height - mh,
            ),
        };
        position(self.marker.upcast_ref(), mx, my);
        self.strip_bounds.set(if self.revealed.get() {
            (tx, ty, tw, th)
        } else {
            (mx, my, mw, mh)
        });
        let docked = self.docked.borrow();
        // Resolve a visible note's pointer from measured tab geometry, including
        // after a save changes Recent ordering. Overflow keeps its Search origin.
        if let Some(editor) = docked.as_ref()
            && let Some(fixed) = self.tabs.first_child().and_downcast::<gtk::Fixed>()
        {
            let mut child = fixed.first_child();
            while let Some(tab) = child {
                if tab.widget_name() == format!("note-{:?}", editor.id()) {
                    let (x, y) = fixed.child_position(&tab);
                    let (w, h) = measure(&tab);
                    self.pointer_origin.set(Some(
                        offset
                            + if horizontal {
                                x as i32 + w / 2
                            } else {
                                y as i32 + h / 2
                            },
                    ));
                    break;
                }
                child = tab.next_sibling();
            }
        } else if self.searching.get()
            && let Some(utilities) = self.tabs.last_child()
        {
            let (w, h) = measure(&utilities);
            self.pointer_origin.set(Some(
                offset + if horizontal { tw - w / 2 } else { th - h / 2 },
            ));
        }
        let content_open = docked.is_some() || self.searching.get();
        if content_open {
            let (max_width, max_height) = if horizontal {
                (screen_width, screen_height - th - 10)
            } else {
                (screen_width - tw - 10, screen_height)
            };
            let (width, height) = if let Some(editor) = docked.as_ref() {
                let (w, h) = editor.docked_size();
                editor.set_docked_size(w.min(max_width), h.min(max_height));
                editor.set_resize_edge(match config.edge {
                    ScreenEdge::Right => gtk::gdk::SurfaceEdge::SouthWest,
                    ScreenEdge::Bottom => gtk::gdk::SurfaceEdge::NorthEast,
                    _ => gtk::gdk::SurfaceEdge::SouthEast,
                });
                editor.docked_size()
            } else {
                measure(self.stack.child_by_name("search").unwrap().upcast_ref())
            };
            let origin = self
                .pointer_origin
                .get()
                .unwrap_or(offset + if horizontal { tw / 2 } else { th / 2 });
            let along = self
                .docked_offset
                .get()
                .filter(|(edge, _)| *edge == config.edge)
                .map_or(
                    origin - if horizontal { width / 2 } else { height / 2 },
                    |(_, offset)| offset,
                );
            let along = along.clamp(
                0,
                if horizontal {
                    screen_width - width
                } else {
                    screen_height - height
                }
                .max(0),
            );
            if docked.is_some() {
                self.docked_offset.set(Some((config.edge, along)));
            }
            let (x, y) = match config.edge {
                ScreenEdge::Left => (tw + 10, along),
                ScreenEdge::Right => (screen_width - tw - 10 - width, along),
                ScreenEdge::Top => (along, th + 10),
                ScreenEdge::Bottom => (along, screen_height - th - 10 - height),
            };
            position(self.stack.upcast_ref(), x, y);
            self.panel_bounds.set((x, y, width, height));
        }
        let active = docked
            .as_ref()
            .map(|editor| format!("note-{:?}", editor.id()));
        if let Some(group) = self.tabs.first_child() {
            let mut child = group.first_child();
            while let Some(tab) = child {
                if active.as_deref() == Some(tab.widget_name().as_str()) {
                    tab.add_css_class("edge-active");
                } else {
                    tab.remove_css_class("edge-active");
                }
                child = tab.next_sibling();
            }
        }
        if let Some(search) = self
            .tabs
            .last_child()
            .and_then(|group| group.first_child())
            .and_then(|first| first.next_sibling())
        {
            if self.searching.get() {
                search.add_css_class("edge-active");
            } else {
                search.remove_css_class("edge-active");
            }
        }
        self.strip_window.set_layer(if config.hide_in_fullscreen {
            Layer::Top
        } else {
            Layer::Overlay
        });
        self.window
            .set_layer(if config.hide_in_fullscreen && docked.is_none() {
                Layer::Top
            } else {
                Layer::Overlay
            });
        self.strip_window
            .set_keyboard_mode(if content_open || !self.revealed.get() {
                KeyboardMode::None
            } else {
                KeyboardMode::OnDemand
            });
        self.window.set_keyboard_mode(if content_open {
            KeyboardMode::OnDemand
        } else {
            KeyboardMode::None
        });
        self.pointer.set_visible(content_open);
        drop(docked);
        drop(config);
        self.pointer.queue_draw();
        self.root.queue_draw();
        self.set_input();
    }
    fn note_limits(&self) -> (i32, i32) {
        gtk::gdk::Display::default()
            .and_then(|display| {
                display
                    .monitors()
                    .item(0)
                    .and_downcast::<gtk::gdk::Monitor>()
            })
            .map(|monitor| {
                let geometry = monitor.geometry();
                (
                    geometry.width().max(editor::MIN_WIDTH),
                    geometry.height().max(editor::MIN_HEIGHT),
                )
            })
            .unwrap_or((720, 500))
    }
    fn docked_pointer(&self, gesture: &gtk::GestureDrag) -> Option<(f64, f64)> {
        // The panel surface has a fixed screen origin, including during resize.
        gesture.current_event()?.position()
    }
    fn set_input(&self) {
        for (window, bounds) in [
            (&self.strip_window, Some(self.strip_bounds.get())),
            (
                &self.window,
                (self.docked.borrow().is_some() || self.searching.get())
                    .then(|| self.panel_bounds.get()),
            ),
        ] {
            if let Some(surface) = window.surface() {
                let region = gtk::cairo::Region::create();
                if let Some((x, y, width, height)) = bounds {
                    let _ =
                        region.union_rectangle(&gtk::cairo::RectangleInt::new(x, y, width, height));
                }
                surface.set_input_region(Some(&region));
            }
        }
    }
    fn draw_pointer(&self, cr: &gtk::cairo::Context) {
        let (x, y, w, h) = self.panel_bounds.get();
        let bounds = gtk::graphene::Rect::new(x as f32, y as f32, w as f32, h as f32);
        let session = self.docked.borrow().as_ref().and_then(|editor| {
            self.sessions
                .borrow()
                .get(&editor.id())
                .and_then(Weak::upgrade)
        });
        let config = session.as_ref().map_or_else(
            || self.config.borrow().clone(),
            |session| session.style().resolve(&self.config.borrow()),
        );
        let edge = config.edge;
        let horizontal = matches!(edge, ScreenEdge::Top | ScreenEdge::Bottom);
        let start = if horizontal {
            bounds.x() as f64
        } else {
            bounds.y() as f64
        };
        let length = if horizontal {
            bounds.width() as f64
        } else {
            bounds.height() as f64
        };
        let offset = if horizontal {
            self.window.margin(Edge::Left)
        } else {
            self.window.margin(Edge::Top)
        };
        let center = self
            .pointer_origin
            .get()
            .map_or(start + length / 2., |origin| f64::from(origin - offset))
            .clamp(start + 30., start + length - 30.);
        let x = bounds.x() as f64;
        let y = bounds.y() as f64;
        let w = bounds.width() as f64;
        let h = bounds.height() as f64;
        let inset = if session.is_some() { 12. } else { 1. };
        let (tip, a, b) = match edge {
            ScreenEdge::Left => (
                (x - 4., center),
                (x + inset, center - 6.),
                (x + inset, center + 6.),
            ),
            ScreenEdge::Right => (
                (x + w + 4., center),
                (x + w - inset, center + 6.),
                (x + w - inset, center - 6.),
            ),
            ScreenEdge::Top => (
                (center, y - 4.),
                (center + 6., y + inset),
                (center - 6., y + inset),
            ),
            ScreenEdge::Bottom => (
                (center, y + h + 4.),
                (center - 6., y + h - inset),
                (center + 6., y + h - inset),
            ),
        };
        let (r, g, bg) = if let Some(session) = session.as_ref() {
            match config.theme {
                config::Theme::Playful => session.color().fill(),
                config::Theme::Minimal if appearance::dark(&config) => {
                    (39. / 255., 44. / 255., 50. / 255.)
                }
                config::Theme::Minimal => (1., 1., 1.),
                config::Theme::Paper if config.white_paper => (1., 1., 1.),
                config::Theme::Paper => (1., 0.966, 0.864),
            }
        } else if config.theme == config::Theme::Minimal && appearance::dark(&config) {
            (34. / 255., 37. / 255., 44. / 255.)
        } else {
            (248. / 255., 247. / 255., 243. / 255.)
        };
        cr.move_to(a.0, a.1);
        cr.line_to(tip.0, tip.1);
        cr.line_to(b.0, b.1);
        cr.set_source_rgb(r, g, bg);
        let _ = cr.fill_preserve();
        if session.is_some() && config.theme == config::Theme::Playful {
            cr.set_source_rgb(0.10, 0.12, 0.11);
            cr.set_line_width(3.2);
        } else if config.theme == config::Theme::Minimal && appearance::dark(&config) {
            cr.set_source_rgba(0.75, 0.80, 0.86, 0.12);
            cr.set_line_width(0.7);
        } else {
            cr.set_source_rgba(
                0.22,
                0.24,
                0.27,
                if config.theme == config::Theme::Minimal {
                    0.12
                } else {
                    0.16
                },
            );
            cr.set_line_width(0.7);
        }
        let _ = cr.stroke();
    }
    fn error(&self, message: &str) {
        let dialog = gtk::MessageDialog::builder()
            .application(&self.application)
            .message_type(gtk::MessageType::Error)
            .buttons(gtk::ButtonsType::Close)
            .text(message)
            .build();
        dialog.connect_response(|dialog, _| dialog.close());
        dialog.present();
    }
}

fn main() -> glib::ExitCode {
    if let Err(error) = ui_assets::register() {
        eprintln!("Could not load UI assets: {error}");
        return glib::ExitCode::FAILURE;
    }
    if let Err(error) = adw::init() {
        eprintln!("Could not initialize the desktop UI: {error}");
        return glib::ExitCode::FAILURE;
    }
    let application = gtk::Application::builder()
        .application_id(APPLICATION_ID)
        .build();
    application.connect_activate(|application| {
        if application.windows().is_empty() {
            build(application);
        }
    });
    application.run()
}
fn build(application: &gtk::Application) -> Option<Rc<Ui>> {
    let storage = match Storage::open(&storage::database_path()) {
        Ok(storage) => Rc::new(storage),
        Err(error) => {
            let dialog = gtk::MessageDialog::builder()
                .application(application)
                .text("Stickies could not open your notes")
                .secondary_text(error.to_string())
                .buttons(gtk::ButtonsType::Close)
                .build();
            dialog.connect_response(|d, _| d.close());
            dialog.present();
            return None;
        }
    };
    let (config, valid) = match Config::load(&config::config_path()) {
        Ok(config) => (config, true),
        Err(error) => {
            eprintln!("Stickies ignored invalid configuration: {error}");
            (Config::default(), false)
        }
    };
    if valid
        && let Err(error) =
            autostart::reconcile(&autostart::autostart_path(), config.launch_at_login)
    {
        eprintln!("Stickies could not update launch at login: {error}");
    }
    let provider = gtk::CssProvider::new();
    appearance::apply(&provider, &config);
    gtk::style_context_add_provider_for_display(
        &gtk::gdk::Display::default().expect("GTK display"),
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
    let window = gtk::ApplicationWindow::builder()
        .application(application)
        .decorated(false)
        .resizable(false)
        .default_width(WIDTH)
        .default_height(HEIGHT)
        .build();
    window.add_css_class("edge-window");
    window.init_layer_shell();
    window.set_namespace(Some("stickies"));
    // Position against the screen bounds used by our clamps, without reserving space.
    window.set_exclusive_zone(-1);
    window.set_keyboard_mode(KeyboardMode::None);
    let strip_window = gtk::ApplicationWindow::builder()
        .application(application)
        .decorated(false)
        .resizable(false)
        .build();
    strip_window.add_css_class("edge-window");
    strip_window.init_layer_shell();
    strip_window.set_namespace(Some("stickies-strip"));
    strip_window.set_exclusive_zone(-1);
    strip_window.set_keyboard_mode(KeyboardMode::None);
    let strip_root = gtk::Overlay::new();
    strip_root.set_child(Some(&gtk::Box::new(gtk::Orientation::Vertical, 0)));
    let root = gtk::Overlay::new();
    // Keep a transparent render node even when the last panel is hidden.
    // An empty snapshot can leave the previous buffer visible on the compositor.
    let hit_area = gtk::DrawingArea::new();
    hit_area.set_draw_func(|_, cr, _, _| {
        cr.set_operator(gtk::cairo::Operator::Clear);
        let _ = cr.paint();
    });
    hit_area.add_css_class("hit-area");
    root.set_child(Some(&hit_area));
    let stack = gtk::Stack::new();
    stack.set_hhomogeneous(false);
    stack.set_vhomogeneous(false);
    // Different child sizes must switch together with the Layer Shell allocation.
    stack.set_transition_type(gtk::StackTransitionType::None);
    stack.set_transition_duration(0);
    stack.set_visible(false);
    let tabs = gtk::Box::new(gtk::Orientation::Vertical, 12);
    tabs.set_visible(false);
    strip_root.add_overlay(&tabs);
    root.add_overlay(&stack);
    let search_panel = gtk::Box::new(gtk::Orientation::Vertical, 6);
    search_panel.add_css_class("edge-search");
    search_panel.set_size_request(288, 300);
    let edge_search = gtk::SearchEntry::new();
    edge_search.set_placeholder_text(Some("Search active notes"));
    edge_search.set_hexpand(true);
    search_panel.append(&edge_search);
    let search_results = gtk::ListBox::new();
    search_results.set_activate_on_single_click(true);
    let scroll = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .vexpand(true)
        .child(&search_results)
        .build();
    search_panel.append(&scroll);
    let search_create = gtk::Button::with_label("No active notes");
    search_create.add_css_class("edge-create-result");
    search_panel.append(&search_create);
    stack.add_named(&search_panel, Some("search"));
    let marker = gtk::Box::new(gtk::Orientation::Vertical, 6);
    marker.add_css_class("edge-marker");

    marker.set_can_target(false);
    strip_root.add_overlay(&marker);
    let pointer = gtk::DrawingArea::new();
    pointer.set_can_target(false);
    pointer.set_hexpand(true);
    pointer.set_vexpand(true);
    pointer.set_visible(false);
    root.add_overlay(&pointer);
    let ui = Rc::new(Ui {
        system_animations: gtk::Settings::default().is_some_and(|s| s.is_gtk_enable_animations()),
        sessions: RefCell::new(HashMap::new()),
        application: application.clone(),
        window: window.clone(),
        root: root.clone(),
        strip_window: strip_window.clone(),
        strip_root: strip_root.clone(),
        strip_offset: Cell::new(None),
        horizontal_tab_width: Cell::new(0),
        strip_bounds: Cell::new((0, 0, 0, 0)),
        panel_bounds: Cell::new((0, 0, 0, 0)),
        stack,
        marker,
        tabs,
        edge_sort: Cell::new(NoteSort::Recent),
        edge_search,
        search_results,
        search_ids: RefCell::new(Vec::new()),
        search_create,
        searching: Cell::new(false),
        hovering: Cell::new(false),
        pointer,
        pointer_origin: Cell::new(None),
        config: Rc::new(RefCell::new(config)),
        config_valid: valid,
        dnd: Cell::new(false),
        dnd_monitor: RefCell::new(None),
        provider,
        storage,
        docked: RefCell::new(None),
        docked_offset: Cell::new(None),
        deck: RefCell::new(AppState::new(Vec::new())),
        separate: RefCell::new(HashMap::new()),
        manager: RefCell::new(None),
        revealed: Cell::new(false),
        generation: Cell::new(0),
    });
    if let Some(settings) = gtk::Settings::default() {
        settings
            .set_gtk_enable_animations(ui.system_animations && !ui.config.borrow().reduced_motion);
    }
    let weak = Rc::downgrade(&ui);
    ui.edge_search.connect_search_changed(move |_| {
        if let Some(ui) = weak.upgrade()
            && ui.searching.get()
        {
            ui.refresh_edge_search();
        }
    });
    let weak = Rc::downgrade(&ui);
    ui.edge_search.connect_activate(move |_| {
        if let Some(ui) = weak.upgrade() {
            ui.activate_edge_search();
        }
    });
    let weak = Rc::downgrade(&ui);
    ui.search_create.connect_clicked(move |_| {
        if let Some(ui) = weak.upgrade() {
            ui.activate_edge_search();
        }
    });
    let weak = Rc::downgrade(&ui);
    ui.search_results.connect_row_activated(move |_, row| {
        if let Some(ui) = weak.upgrade() {
            let id = ui.search_ids.borrow().get(row.index() as usize).copied();
            if let Some(id) = id {
                ui.open_docked(id);
            }
        }
    });
    for window in [&ui.window, &ui.strip_window] {
        let weak = Rc::downgrade(&ui);
        window.connect_is_active_notify(move |window| {
            // Request focus only for the explicit opening/closing action. Return
            // to on-demand immediately so another window can take focus normally.
            if window.is_active() && window.keyboard_mode() == KeyboardMode::Exclusive {
                window.set_keyboard_mode(KeyboardMode::OnDemand);
            }
            let weak = weak.clone();
            // Focus transfers between our two surfaces finish before dismissal.
            glib::idle_add_local_once(move || {
                if let Some(ui) = weak.upgrade()
                    && ui.searching.get()
                    && !ui.window.is_active()
                    && !ui.strip_window.is_active()
                {
                    ui.close_edge_search();
                    ui.schedule_hide();
                }
            });
        });
    }
    let weak = Rc::downgrade(&ui);
    ui.pointer.set_draw_func(move |_, cr, _, _| {
        if let Some(ui) = weak.upgrade() {
            ui.draw_pointer(cr);
        }
    });
    ui.place_edge();
    ui.refresh_tabs();
    for area in [&root, &strip_root] {
        let motion = gtk::EventControllerMotion::new();
        {
            let weak = Rc::downgrade(&ui);
            motion.connect_enter(move |_, _, _| {
                if let Some(ui) = weak.upgrade()
                    && ui.config.borrow().edge_hover_enabled
                {
                    ui.schedule_reveal();
                }
            });
        }
        {
            let weak = Rc::downgrade(&ui);
            motion.connect_leave(move |_| {
                if let Some(ui) = weak.upgrade() {
                    ui.schedule_hide();
                }
            });
        }
        area.add_controller(motion);
    }
    let click = gtk::GestureClick::new();
    {
        let weak = Rc::downgrade(&ui);
        click.connect_released(move |_, _, _, _| {
            if let Some(ui) = weak.upgrade() {
                ui.reveal();
            }
        });
    }
    strip_root.add_controller(click);
    let drag = gtk::GestureDrag::new();
    let initial = Rc::new(Cell::new(None));
    {
        let weak = Rc::downgrade(&ui);
        let initial = initial.clone();
        drag.connect_drag_begin(move |gesture, _, _| {
            initial.set(None);
            if let Some(ui) = weak.upgrade() {
                if ui.docked.borrow().is_some() || ui.revealed.get() {
                    gesture.set_state(gtk::EventSequenceState::Denied);
                    return;
                }
                ui.hovering.set(false);
                ui.generation.set(ui.generation.get().wrapping_add(1));
                initial.set(Some(ui.config.borrow().marker_offset));
            }
        });
    }
    {
        let weak = Rc::downgrade(&ui);
        drag.connect_drag_end(move |_, x, y| {
            let Some(start) = initial.take() else {
                return;
            };
            if let Some(ui) = weak.upgrade()
                && ui.docked.borrow().is_none()
                && ui.config_valid
            {
                let mut config = ui.config.borrow().clone();
                let horizontal = matches!(config.edge, ScreenEdge::Top | ScreenEdge::Bottom);
                let distance = if horizontal { x } else { y };
                let Some(display) = gtk::gdk::Display::default() else {
                    return;
                };
                let Some(monitor) = display
                    .monitors()
                    .item(0)
                    .and_downcast::<gtk::gdk::Monitor>()
                else {
                    return;
                };
                let geometry = monitor.geometry();
                let available = if horizontal {
                    geometry.width() - ui.strip_bounds.get().2
                } else {
                    geometry.height() - ui.strip_bounds.get().3
                };
                if available <= 0 {
                    return;
                }
                config.marker_offset = (start + distance / f64::from(available)).clamp(0., 1.);
                if config.save(&config::config_path()).is_ok() {
                    ui.config.replace(config);
                    ui.strip_offset.set(None);
                    ui.place_edge();
                }
            }
        });
    }
    strip_root.add_controller(drag);
    for target in [&window, &strip_window] {
        let keys = gtk::EventControllerKey::new();
        keys.set_propagation_phase(gtk::PropagationPhase::Capture);
        {
            let weak = Rc::downgrade(&ui);
            keys.connect_key_pressed(move |_, key, _, _| {
                if key == gtk::gdk::Key::Escape {
                    if let Some(ui) = weak.upgrade() {
                        if ui.docked.borrow().is_some() {
                            if ui.close_docked() && !ui.strip_window.is_active() {
                                ui.strip_window.set_keyboard_mode(KeyboardMode::Exclusive);
                            }
                        } else if ui.searching.get() {
                            ui.close_edge_search();
                            if !ui.strip_window.is_active() {
                                ui.strip_window.set_keyboard_mode(KeyboardMode::Exclusive);
                            }
                        } else {
                            ui.dormant();
                        }
                    }
                    glib::Propagation::Stop
                } else if matches!(key, gtk::gdk::Key::Down | gtk::gdk::Key::Up) {
                    if let Some(ui) = weak.upgrade()
                        && ui.searching.get()
                    {
                        let current = ui
                            .search_results
                            .selected_row()
                            .map_or(0, |row| row.index());
                        let count = ui.search_ids.borrow().len() as i32;
                        let next = (current + if key == gtk::gdk::Key::Down { 1 } else { -1 })
                            .clamp(0, (count - 1).max(0));
                        ui.search_results
                            .select_row(ui.search_results.row_at_index(next).as_ref());
                        return glib::Propagation::Stop;
                    }
                    glib::Propagation::Proceed
                } else {
                    glib::Propagation::Proceed
                }
            });
        }
        target.add_controller(keys);
    }
    {
        let weak = Rc::downgrade(&ui);
        window.connect_map(move |_| {
            if let Some(ui) = weak.upgrade() {
                ui.set_input();
            }
        });
    }
    {
        let weak = Rc::downgrade(&ui);
        window.connect_realize(move |window| {
            if let Some(surface) = window.surface() {
                let weak = weak.clone();
                surface.connect_layout(move |_, _, _| {
                    if let Some(ui) = weak.upgrade() {
                        ui.set_input();
                    }
                });
            }
        });
    }
    // Keep the coordinator alive for the application lifetime, without widget cycles.
    let shutdown_ui = ui.clone();
    application.connect_shutdown(move |_| {
        let ui = &shutdown_ui;
        let manager = ui.manager.borrow().clone();
        if let Some(manager) = manager {
            manager.flush();
        }
        if let Some(editor) = ui.docked.borrow().as_ref() {
            editor.flush();
        }
        for note in ui.separate.borrow().values() {
            note.editor.flush();
        }
    });
    strip_window.set_child(Some(&strip_root));
    window.set_child(Some(&root));
    window.present();
    strip_window.present();
    ui.set_input();
    let weak = Rc::downgrade(&ui);
    let monitor = platform::watch_do_not_disturb(move |dnd| {
        if let Some(ui) = weak.upgrade() {
            ui.dnd.set(dnd);
            ui.update_visibility();
        }
    });
    ui.dnd_monitor.replace(monitor);
    Some(ui)
}

#[cfg(test)]
mod edge_tests {
    use super::*;

    fn capture(ui: &Ui, name: &str) {
        let Ok(directory) = std::env::var("STICKIES_EDGE_CAPTURES") else {
            return;
        };
        let snapshot = gtk::Snapshot::new();
        for widget in [&ui.root, &ui.strip_root] {
            gtk::WidgetPaintable::new(Some(widget)).snapshot(
                &snapshot,
                f64::from(widget.width()),
                f64::from(widget.height()),
            );
        }
        let node = snapshot.to_node().unwrap();
        let (sx, sy, sw, sh) = ui.strip_bounds.get();
        let (x, y, w, h) = if ui.docked.borrow().is_some() || ui.searching.get() {
            let (px, py, pw, ph) = ui.panel_bounds.get();
            let x = sx.min(px).saturating_sub(8).max(0);
            let y = sy.min(py).saturating_sub(8).max(0);
            (
                x,
                y,
                (sx + sw).max(px + pw) - x + 8,
                (sy + sh).max(py + ph) - y + 8,
            )
        } else {
            (sx, sy, sw, sh)
        };
        let viewport = gtk::graphene::Rect::new(x as f32, y as f32, w as f32, h as f32);
        let texture = ui
            .window
            .renderer()
            .unwrap()
            .render_texture(&node, Some(&viewport));
        std::fs::create_dir_all(&directory).unwrap();
        texture
            .save_to_png(std::path::Path::new(&directory).join(format!("{name}.png")))
            .unwrap();
    }

    fn capture_note(ui: &Ui, name: &str) {
        let Ok(directory) = std::env::var("STICKIES_EDGE_CAPTURES") else {
            return;
        };
        let editor = ui.docked.borrow().clone().unwrap();
        assert!(editor.root.is_mapped());
        let snapshot = gtk::Snapshot::new();
        gtk::WidgetPaintable::new(Some(&ui.root)).snapshot(
            &snapshot,
            f64::from(ui.root.width()),
            f64::from(ui.root.height()),
        );
        let bounds = editor.root.compute_bounds(&ui.root).unwrap();
        let node = snapshot.to_node().unwrap();
        ui.window
            .renderer()
            .unwrap()
            .render_texture(&node, Some(&bounds))
            .save_to_png(std::path::Path::new(&directory).join(format!("{name}.png")))
            .unwrap();
    }

    #[test]
    #[ignore = "requires Wayland and disposable XDG_DATA_HOME/XDG_CONFIG_HOME directories"]
    fn edge_state_changes_preserve_bounds_and_tab_orientation() {
        for variable in ["XDG_DATA_HOME", "XDG_CONFIG_HOME", "XDG_CACHE_HOME"] {
            let path = std::path::PathBuf::from(std::env::var(variable).expect(variable));
            assert!(
                path.starts_with(std::env::temp_dir()),
                "use disposable XDG paths"
            );
        }
        ui_assets::register().unwrap();
        adw::init().unwrap();
        let application = gtk::Application::builder()
            .application_id("dev.stickies.EdgeRegression")
            .flags(gtk::gio::ApplicationFlags::NON_UNIQUE)
            .build();
        application
            .register(None::<&gtk::gio::Cancellable>)
            .unwrap();
        let storage = Storage::open(&storage::database_path()).unwrap();
        let mut ids = Vec::new();
        for (index, title) in [
            "Groceries",
            "Weekend Plans",
            "Reading",
            "A long title that must truncate without expanding the strip",
        ]
        .into_iter()
        .enumerate()
        {
            ids.push(
                storage
                    .create_note(
                        title,
                        "- [ ] Milk\n- [ ] Eggs\n- [ ] Bread\n- [ ] Coffee\n- [ ] Fruits & veggies",
                        [
                            NoteColor::Green,
                            NoteColor::Blue,
                            NoteColor::Pink,
                            NoteColor::Yellow,
                        ][index],
                    )
                    .unwrap()
                    .id,
            );
        }
        storage.set_note_pinned(ids[0], true).unwrap();
        Config {
            launch_at_login: false,
            ..Config::default()
        }
        .save(&config::config_path())
        .unwrap();
        let ui = build(&application).unwrap();
        let settle = || {
            let end = std::time::Instant::now() + Duration::from_millis(180);
            while std::time::Instant::now() < end {
                while glib::MainContext::default().pending() {
                    glib::MainContext::default().iteration(false);
                }
                std::thread::sleep(Duration::from_millis(1));
            }
        };
        for (theme, appearance, name) in [
            (config::Theme::Paper, config::Appearance::Light, "paper"),
            (config::Theme::Playful, config::Appearance::Light, "playful"),
            (
                config::Theme::Minimal,
                config::Appearance::Light,
                "minimal-light",
            ),
            (
                config::Theme::Minimal,
                config::Appearance::Dark,
                "minimal-dark",
            ),
        ] {
            for edge in [
                ScreenEdge::Left,
                ScreenEdge::Right,
                ScreenEdge::Top,
                ScreenEdge::Bottom,
            ] {
                ui.dormant();
                let mut config = ui.config.borrow().clone();
                config.edge = edge;
                config.theme = theme;
                config.appearance = appearance;
                config.marker_offset = 0.5;
                config.launch_at_login = false;
                ui.change_settings(config).unwrap();
                settle();
                let prefix = format!("{name}-{edge:?}").to_lowercase();
                capture(&ui, &format!("{prefix}-resting"));
                ui.reveal();
                settle();
                capture(&ui, &format!("{prefix}-expanded"));
                let extent = (ui.root.width(), ui.root.height());
                let tab = ui.tabs.first_child().unwrap().first_child().unwrap();
                let horizontal = matches!(edge, ScreenEdge::Top | ScreenEdge::Bottom);
                let bounds = tab.compute_bounds(&ui.strip_root).unwrap();
                assert_eq!(bounds.height() as i32, if horizontal { 34 } else { 80 });
                if horizontal {
                    assert!((80..=112).contains(&(bounds.width() as i32)));
                    assert!(tab.first_child().unwrap().is::<gtk::Label>());
                } else {
                    assert_eq!(bounds.width() as i32, 34);
                }
                let utilities = ui.tabs.last_child().unwrap();
                let mut control = utilities.first_child();
                while let Some(button) = control {
                    let bounds = button.compute_bounds(&ui.strip_root).unwrap();
                    assert_eq!(bounds.width() as i32, 34);
                    assert_eq!(bounds.height() as i32, if horizontal { 34 } else { 28 });
                    control = button.next_sibling();
                }
                for _ in 0..3 {
                    ui.show_edge_search();
                    settle();
                    capture(&ui, &format!("{prefix}-search"));
                    assert!(ui.tabs.is_mapped());
                    let current = ui
                        .tabs
                        .first_child()
                        .unwrap()
                        .first_child()
                        .unwrap()
                        .compute_bounds(&ui.strip_root)
                        .unwrap();
                    assert_eq!(current, bounds);
                    assert!(!ui.stack.is_transition_running());
                    ui.close_edge_search();
                    ui.open_docked(ids[0]);
                    settle();
                    capture(&ui, &format!("{prefix}-note"));
                    let editor = ui.docked.borrow().clone().unwrap();
                    assert_eq!(
                        (editor.root.width(), editor.root.height()),
                        (editor::WIDTH, editor::HEIGHT)
                    );
                    capture_note(&ui, &format!("{name}-regression-default"));
                    editor.set_docked_size(editor::MIN_WIDTH, editor::MIN_HEIGHT);
                    ui.place_edge();
                    settle();
                    capture(&ui, &format!("{prefix}-minimum"));
                    capture_note(&ui, &format!("{name}-regression-minimum"));
                    editor.set_docked_size(editor::WIDTH, editor::HEIGHT);
                    ui.place_edge();
                    settle();
                    assert!(ui.tabs.is_mapped());
                    let current = ui
                        .tabs
                        .first_child()
                        .unwrap()
                        .first_child()
                        .unwrap()
                        .compute_bounds(&ui.strip_root)
                        .unwrap();
                    assert_eq!(current, bounds);
                    assert!(!ui.stack.is_transition_running());
                    if edge == ScreenEdge::Right {
                        assert_eq!(ui.root.width(), extent.0);
                    } else if edge == ScreenEdge::Bottom {
                        assert_eq!(ui.root.height(), extent.1);
                    }
                    assert!(ui.close_docked());
                    settle();
                    assert!(!ui.stack.is_transition_running());
                    assert_eq!((ui.root.width(), ui.root.height()), extent);
                    let tab = ui.tabs.first_child().unwrap().first_child().unwrap();
                    let bounds = tab.compute_bounds(&ui.strip_root).unwrap();
                    if edge == ScreenEdge::Right {
                        assert_eq!((bounds.x() + bounds.width()) as i32, extent.0);
                    } else if edge == ScreenEdge::Bottom {
                        assert_eq!((bounds.y() + bounds.height()) as i32, extent.1);
                    }
                }
            }
        }
        // Hiding the last panel must submit a transparent buffer, not leave the
        // compositor displaying the previous Search frame.
        ui.close_edge_search();
        settle();
        let snapshot = gtk::Snapshot::new();
        gtk::WidgetPaintable::new(Some(&ui.root)).snapshot(
            &snapshot,
            f64::from(ui.root.width()),
            f64::from(ui.root.height()),
        );
        let node = snapshot.to_node().expect("clear the previous panel buffer");
        let texture = ui.window.renderer().unwrap().render_texture(&node, None);
        let stride = texture.width() as usize * 4;
        let mut pixels = vec![0; stride * texture.height() as usize];
        texture.download(&mut pixels, stride);
        assert!(pixels.as_chunks::<4>().0.iter().all(|pixel| pixel[3] == 0));
        // Replacement and closure must retain the dirty editor on save failure.
        ui.open_docked(ids[0]);
        let editor = ui.docked.borrow().clone().unwrap();
        editor
            .body
            .buffer()
            .set_text("Must survive failed replacement");
        let connection = rusqlite::Connection::open(storage::database_path()).unwrap();
        connection.execute_batch("CREATE TRIGGER reject_edge_edit BEFORE UPDATE ON notes BEGIN SELECT RAISE(FAIL, 'edge save failure'); END;").unwrap();
        ui.show_edge_search();
        assert!(!ui.searching.get());
        ui.open_docked(ids[1]);
        assert_eq!(ui.docked.borrow().as_ref().unwrap().id(), ids[0]);
        let count = ui
            .storage
            .search_notes(NoteCollection::Active, "", NoteSort::Recent)
            .unwrap()
            .len();
        ui.create_note();
        assert_eq!(
            ui.storage
                .search_notes(NoteCollection::Active, "", NoteSort::Recent)
                .unwrap()
                .len(),
            count
        );
        assert!(!ui.close_docked());
        assert_eq!(
            editor.body.buffer().text(
                &editor.body.buffer().start_iter(),
                &editor.body.buffer().end_iter(),
                true
            ),
            "Must survive failed replacement"
        );
        connection
            .execute_batch("DROP TRIGGER reject_edge_edit;")
            .unwrap();
        assert!(ui.close_docked());
        assert_eq!(
            ui.storage.load_note(ids[0]).unwrap().body,
            "Must survive failed replacement"
        );
        for edge in [
            ScreenEdge::Left,
            ScreenEdge::Right,
            ScreenEdge::Top,
            ScreenEdge::Bottom,
        ] {
            for offset in [0., 1.] {
                ui.dormant();
                let mut config = ui.config.borrow().clone();
                config.edge = edge;
                config.marker_offset = offset;
                ui.change_settings(config).unwrap();
                ui.reveal();
                settle();
                let strip = ui.strip_bounds.get();
                ui.open_docked(ids[0]);
                let editor = ui.docked.borrow().clone().unwrap();
                for size in [
                    (editor::WIDTH, editor::HEIGHT),
                    (editor::MIN_WIDTH, editor::MIN_HEIGHT),
                    (700, 600),
                ] {
                    editor.set_docked_size(size.0, size.1);
                    ui.place_edge();
                    settle();
                    assert_eq!(ui.strip_bounds.get(), strip);
                    assert_eq!((editor.root.width(), editor.root.height()), size);
                    let (x, y, w, h) = ui.panel_bounds.get();
                    let (sw, sh) = ui.note_limits();
                    assert!(x >= 0 && y >= 0 && x + w <= sw && y + h <= sh);
                    assert!(!ui.pointer.can_target());
                }
                ui.show_edge_search();
                settle();
                assert!(ui.searching.get());
                assert!(ui.tabs.is_mapped());
                assert_eq!(ui.strip_bounds.get(), strip);
                let (x, y, w, h) = ui.panel_bounds.get();
                let (sw, sh) = ui.note_limits();
                assert!(x >= 0 && y >= 0 && x + w <= sw && y + h <= sh);
                ui.close_edge_search();
            }
        }
        // Reduced motion does not select a different edge animation path.
        for reduced in [false, true] {
            let mut config = ui.config.borrow().clone();
            config.reduced_motion = reduced;
            config.hide_in_dnd = true;
            config.hide_in_fullscreen = true;
            ui.change_settings(config).unwrap();
            ui.open_docked(ids[0]);
            ui.dnd.set(true);
            ui.update_visibility();
            assert!(ui.window.is_visible());
            assert!(!ui.strip_window.is_visible());
            assert_eq!(ui.window.layer(), Layer::Overlay);
            assert_eq!(ui.strip_window.layer(), Layer::Top);
            ui.dnd.set(false);
            ui.update_visibility();
            ui.show_edge_search();
            assert_eq!(ui.window.layer(), Layer::Top);
            settle();
            assert!(
                gtk::prelude::GtkWindowExt::focus(&ui.window)
                    .is_some_and(|focus| focus.is_ancestor(&ui.edge_search))
            );
            assert_eq!(ui.stack.transition_duration(), 0);
            ui.close_edge_search();
        }
        let overflow = ui
            .storage
            .create_note("Overflow target", "unique body needle", NoteColor::Purple)
            .unwrap();
        ui.refresh_tabs();
        ui.show_edge_search();
        ui.edge_search.set_text("unique body needle");
        ui.refresh_edge_search();
        assert_eq!(*ui.search_ids.borrow(), vec![overflow.id]);
        ui.activate_edge_search();
        assert_eq!(ui.docked.borrow().as_ref().unwrap().id(), overflow.id);
        ui.show_edge_search();
        ui.edge_search.set_text("New unmatched title");
        ui.refresh_edge_search();
        assert!(ui.search_create.is_sensitive());
        ui.activate_edge_search();
        let created = ui.docked.borrow().as_ref().unwrap().id();
        assert_eq!(
            ui.storage.load_note(created).unwrap().title,
            "New unmatched title"
        );
        ui.close_docked();
        ui.create_note();
        let blank = ui.docked.borrow().as_ref().unwrap().id();
        assert!(ui.deck.borrow().notes().iter().any(|note| note.id == blank));
        ui.close_docked();
        for note in ui
            .storage
            .search_notes(NoteCollection::Active, "", NoteSort::Recent)
            .unwrap()
        {
            ui.storage.archive_note(note.id).unwrap();
        }
        for count in [0, 2] {
            for index in 0..count {
                ui.storage
                    .create_note(&format!("Short {index}"), "", NoteColor::Blue)
                    .unwrap();
            }
            ui.dormant();
            ui.refresh_tabs();
            for edge in [
                ScreenEdge::Left,
                ScreenEdge::Right,
                ScreenEdge::Top,
                ScreenEdge::Bottom,
            ] {
                for offset in [0., 1.] {
                    ui.dormant();
                    let mut config = ui.config.borrow().clone();
                    config.edge = edge;
                    config.marker_offset = offset;
                    ui.change_settings(config).unwrap();
                    ui.reveal();
                    settle();
                    let (x, y, w, h) = ui.strip_bounds.get();
                    let (sw, sh) = ui.note_limits();
                    assert!(x >= 0 && y >= 0 && x + w <= sw && y + h <= sh);
                    capture(
                        &ui,
                        &format!("{edge:?}-{count}-notes-end-{offset}").to_lowercase(),
                    );
                }
            }
        }
        let short = ui.storage.load_deck_notes(4, NoteSort::Recent).unwrap()[0].id;
        let strip_before_edit = ui.strip_bounds.get();
        ui.open_docked(short);
        let editor = ui.docked.borrow().clone().unwrap();
        editor
            .title
            .set_text("A much longer title must not move the horizontal strip while editing");
        assert!(editor.flush());
        settle();
        assert_eq!(ui.strip_bounds.get(), strip_before_edit);
        assert!(ui.close_docked());
        ui.strip_window.close();
        ui.window.close();
    }
}
