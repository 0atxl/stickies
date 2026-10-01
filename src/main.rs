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
use config::{Config, ScreenEdge, TabLayout};
use gtk::{glib, prelude::*};
use gtk4_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};
use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    rc::{Rc, Weak},
    time::Duration,
};
use storage::Storage;

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
    stack: gtk::Stack,
    marker: gtk::Box,
    tabs: gtk::Box,
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
        let notes = match self.storage.load_deck_notes(config.visible_notes) {
            Ok(notes) => notes,
            Err(error) => {
                eprintln!("Could not load edge notes: {error}");
                return;
            }
        };
        self.deck.borrow_mut().replace_notes(notes);
        let notes = self.deck.borrow().notes().to_vec();
        let all = appearance::icon("stickies-notes", "All Notes");
        all.add_css_class("strip-control");
        all.set_halign(gtk::Align::Center);
        {
            let weak = Rc::downgrade(self);
            all.connect_clicked(move |_| {
                if let Some(ui) = weak.upgrade() {
                    ui.show_manager();
                }
            });
        }
        self.tabs.append(&all);
        let horizontal = matches!(config.edge, ScreenEdge::Top | ScreenEdge::Bottom);
        self.tabs.set_orientation(if horizontal {
            gtk::Orientation::Horizontal
        } else {
            gtk::Orientation::Vertical
        });
        self.marker.set_orientation(if horizontal {
            gtk::Orientation::Horizontal
        } else {
            gtk::Orientation::Vertical
        });
        let fixed = gtk::Fixed::new();
        for (index, note) in notes.iter().enumerate() {
            let pill = gtk::Box::new(gtk::Orientation::Vertical, 0);
            pill.add_css_class("edge-pill");
            appearance::set_color(&pill, note.color);
            self.marker.append(&pill);
            let tab = gtk::Button::new();
            tab.add_css_class("note-tab");
            appearance::set_color(&tab, note.color);
            let title = if note.title.trim().is_empty() {
                "Untitled note"
            } else {
                &note.title
            };
            tab.set_tooltip_text(Some(title));
            let label = gtk::Label::new(Some(title));
            label.add_css_class("tab-title");
            label.set_ellipsize(gtk::pango::EllipsizeMode::End);
            label.set_max_width_chars(18);
            label.set_xalign(0.);
            let (x, y, w, h) = if config.tab_layout == TabLayout::Overlapping {
                let text = gtk::Fixed::new();
                text.set_size_request(24, 126);
                label.set_size_request(124, 22);
                text.put(&label, 0., 0.);
                text.set_child_transform(
                    &label,
                    Some(
                        &gtk::gsk::Transform::new()
                            .translate(&gtk::graphene::Point::new(0., 126.))
                            .rotate(-90.),
                    ),
                );
                tab.set_child(Some(&text));
                if horizontal {
                    (index as f64 * 36., 0., 48, 148)
                } else {
                    (index as f64 * 36., index as f64 * 24., 48, 148)
                }
            } else {
                tab.set_child(Some(&label));
                if horizontal {
                    (index as f64 * 116., 0., 110, 44)
                } else {
                    (0., index as f64 * 50., 174, 44)
                }
            };
            tab.set_size_request(w, h);
            fixed.put(&tab, x, y);
            if !config.reduced_motion {
                appearance::animate_tab_on_hover(
                    &fixed,
                    &tab,
                    x,
                    y,
                    config.edge,
                    config.tab_layout == TabLayout::Overlapping,
                );
            }
            let id = note.id;
            let weak = Rc::downgrade(self);
            tab.connect_clicked(move |_| {
                if let Some(ui) = weak.upgrade() {
                    ui.open_docked(id);
                }
            });
        }
        if notes.is_empty() {
            let pill = gtk::Box::new(gtk::Orientation::Vertical, 0);
            pill.add_css_class("edge-pill");
            pill.add_css_class("note-yellow");
            self.marker.append(&pill);
        }
        self.tabs.append(&fixed);
        let create = appearance::icon("list-add-symbolic", "New note");
        create.add_css_class("strip-control");
        create.set_halign(gtk::Align::Center);
        {
            let weak = Rc::downgrade(self);
            create.connect_clicked(move |_| {
                if let Some(ui) = weak.upgrade() {
                    ui.create_note();
                }
            });
        }
        self.tabs.append(&create);
    }
    fn reveal(&self) {
        if self.docked.borrow().is_some() {
            return;
        }
        self.generation.set(self.generation.get().wrapping_add(1));
        self.deck.borrow_mut().dispatch(Action::ShowTabs);
        self.revealed.set(true);
        self.marker.set_visible(false);
        self.stack.set_visible_child_name("tabs");
        self.stack.set_visible(true);
        self.set_input(false);
        self.window.set_keyboard_mode(KeyboardMode::OnDemand);
    }
    fn dormant(&self) {
        if self.docked.borrow().is_some() {
            return;
        }
        self.deck.borrow_mut().dispatch(Action::CollapseDeck);
        self.revealed.set(false);
        self.marker.set_visible(true);
        self.stack.set_visible(false);
        self.window.set_keyboard_mode(KeyboardMode::None);
        self.set_input(true);
    }
    fn schedule_hide(self: &Rc<Self>) {
        let generation = self.generation.get().wrapping_add(1);
        self.generation.set(generation);
        let weak = Rc::downgrade(self);
        glib::timeout_add_local_once(Duration::from_millis(500), move || {
            if let Some(ui) = weak.upgrade()
                && ui.generation.get() == generation
            {
                ui.dormant();
            }
        });
    }
    fn create_note(self: &Rc<Self>) {
        if !self.close_docked() {
            return;
        }
        match self.storage.create_note("", "", NoteColor::Yellow) {
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
                    ui.close_docked();
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
                    ui.set_input(false);
                }
            });
        }
        Some(editor)
    }
    fn open_docked(self: &Rc<Self>, id: NoteId) {
        if let Some(note) = self.separate.borrow().get(&id) {
            note.window.present();
            return;
        }
        if !self.close_docked() {
            return;
        }
        let Some(editor) = self.new_editor(id, false) else {
            return;
        };
        self.deck.borrow_mut().dispatch(Action::OpenNote(id));
        self.stack.add_named(&editor.root, Some("editor"));
        self.stack.set_visible_child_name("editor");
        self.stack.set_visible(true);
        self.marker.set_visible(false);
        self.window.set_keyboard_mode(KeyboardMode::OnDemand);
        self.docked.replace(Some(editor.clone()));
        self.docked_offset.set(None);
        self.window.set_layer(Layer::Overlay);
        self.place_edge();
        self.update_visibility();
        self.set_input(false);
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
            self.docked_offset.set(None);
            self.deck.borrow_mut().dispatch(Action::CollapseDeck);
            self.stack.remove(&editor.root);
            self.marker.set_visible(true);
            self.refresh_tabs();
            self.reveal();
            self.place_edge();
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
        let manager = manager::build_window(
            &self.application,
            self.storage.clone(),
            self.config.clone(),
            manager::Callbacks {
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
        self.window.set_visible(
            self.docked.borrow().is_some() || !(self.config.borrow().hide_in_dnd && self.dnd.get()),
        );
    }

    fn place_edge(&self) {
        let config = self.config.borrow();
        for edge in [Edge::Left, Edge::Right, Edge::Top, Edge::Bottom] {
            self.window.set_anchor(edge, false);
            self.window.set_margin(edge, 0);
        }
        let (edge, halign, valign) = match config.edge {
            ScreenEdge::Left => (Edge::Left, gtk::Align::Start, gtk::Align::Center),
            ScreenEdge::Right => (Edge::Right, gtk::Align::End, gtk::Align::Center),
            ScreenEdge::Top => (Edge::Top, gtk::Align::Center, gtk::Align::Start),
            ScreenEdge::Bottom => (Edge::Bottom, gtk::Align::Center, gtk::Align::End),
        };
        self.window.set_anchor(edge, true);
        self.marker.set_halign(halign);
        self.marker.set_valign(valign);
        self.stack.set_halign(halign);
        self.stack.set_valign(valign);
        let horizontal = matches!(config.edge, ScreenEdge::Top | ScreenEdge::Bottom);
        let (width, height) = if let Some(editor) = self.docked.borrow().as_ref() {
            let (max_width, max_height) = self.note_limits();
            let (width, height) = editor.docked_size();
            editor.set_docked_size(width.min(max_width), height.min(max_height));
            editor.set_resize_edge(match config.edge {
                ScreenEdge::Right => gtk::gdk::SurfaceEdge::SouthWest,
                ScreenEdge::Bottom => gtk::gdk::SurfaceEdge::NorthEast,
                _ => gtk::gdk::SurfaceEdge::SouthEast,
            });
            let (width, height) = editor.docked_size();
            (width, height)
        } else {
            (if horizontal { 720 } else { WIDTH }, HEIGHT)
        };
        // Keep the layer's origin fixed on the axes that would otherwise move
        // during resizing. The input region still contains only the note.
        let (surface_width, surface_height) = if self.docked.borrow().is_some() {
            let (max_width, max_height) = self.note_limits();
            match config.edge {
                ScreenEdge::Right => (max_width, height),
                ScreenEdge::Bottom => (width, max_height),
                _ => (width, height),
            }
        } else {
            (width, height)
        };
        self.root.set_size_request(surface_width, surface_height);
        self.window.set_default_size(surface_width, surface_height);
        if let Some(display) = gtk::gdk::Display::default()
            && let Some(monitor) = display
                .monitors()
                .item(0)
                .and_downcast::<gtk::gdk::Monitor>()
        {
            let geometry = monitor.geometry();
            let available = if horizontal {
                geometry.width() - width
            } else {
                geometry.height() - height
            };
            let perpendicular = if horizontal { Edge::Left } else { Edge::Top };
            self.window.set_anchor(perpendicular, true);
            let available = available.max(0);
            let default_offset = (f64::from(available) * config.marker_offset) as i32;
            let offset = if self.docked.borrow().is_some() {
                let offset = self
                    .docked_offset
                    .get()
                    .filter(|(edge, _)| *edge == config.edge)
                    .map_or(default_offset, |(_, offset)| offset)
                    .clamp(0, available);
                self.docked_offset.set(Some((config.edge, offset)));
                offset
            } else {
                default_offset
            };
            self.window.set_margin(perpendicular, offset);
        }
        self.window.set_layer(
            if config.hide_in_fullscreen && self.docked.borrow().is_none() {
                Layer::Top
            } else {
                Layer::Overlay
            },
        );
        self.stack
            .set_transition_duration(if config.reduced_motion { 0 } else { 140 });
        drop(config);
        self.set_input(!self.revealed.get() && self.docked.borrow().is_none());
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
        let (x, y) = gesture.current_event()?.position()?;
        let surface = self.window.surface()?;
        let (width, height) = self.note_limits();
        // Gesture offsets are relative to the moving grip. Use surface coordinates
        // and our layer anchors so the corner follows the pointer while resizing.
        Some(match self.config.borrow().edge {
            ScreenEdge::Left => (x, y + f64::from(self.window.margin(Edge::Top))),
            ScreenEdge::Right => (
                x + f64::from(width - surface.width()),
                y + f64::from(self.window.margin(Edge::Top)),
            ),
            ScreenEdge::Top => (x + f64::from(self.window.margin(Edge::Left)), y),
            ScreenEdge::Bottom => (
                x + f64::from(self.window.margin(Edge::Left)),
                y + f64::from(height - surface.height()),
            ),
        })
    }
    fn set_input(&self, dormant: bool) {
        let Some(surface) = self.window.surface() else {
            return;
        };
        let width = surface.width();
        let height = surface.height();
        let edge = self.config.borrow().edge;
        if let Some(editor) = self.docked.borrow().as_ref() {
            let bounds = editor.root.compute_bounds(&self.root);
            let (x, y, w, h) = bounds.map_or_else(
                || {
                    let (w, h) = editor.docked_size();
                    (0, 0, w, h)
                },
                |bounds| {
                    (
                        bounds.x() as i32,
                        bounds.y() as i32,
                        bounds.width().ceil() as i32,
                        bounds.height().ceil() as i32,
                    )
                },
            );
            surface.set_input_region(Some(&gtk::cairo::Region::create_rectangle(
                &gtk::cairo::RectangleInt::new(x, y, w, h),
            )));
            return;
        }
        let (w, h) = if dormant {
            if matches!(edge, ScreenEdge::Top | ScreenEdge::Bottom) {
                (150, 20)
            } else {
                (20, 180)
            }
        } else if matches!(edge, ScreenEdge::Top | ScreenEdge::Bottom) {
            (700, 190)
        } else {
            (230, 390)
        };
        let x = match edge {
            ScreenEdge::Left => 0,
            ScreenEdge::Right => width - w,
            _ => (width - w) / 2,
        };
        let y = match edge {
            ScreenEdge::Top => 0,
            ScreenEdge::Bottom => height - h,
            _ => (height - h) / 2,
        };
        surface.set_input_region(Some(&gtk::cairo::Region::create_rectangle(
            &gtk::cairo::RectangleInt::new(x, y, w, h),
        )));
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
    window.set_exclusive_zone(0);
    window.set_keyboard_mode(KeyboardMode::None);
    let root = gtk::Overlay::new();
    let hit_area = gtk::Box::new(gtk::Orientation::Vertical, 0);
    hit_area.add_css_class("hit-area");
    root.set_child(Some(&hit_area));
    let stack = gtk::Stack::new();
    stack.set_hhomogeneous(false);
    stack.set_vhomogeneous(false);
    stack.set_transition_type(gtk::StackTransitionType::Crossfade);
    stack.set_visible(false);
    let tabs = gtk::Box::new(gtk::Orientation::Vertical, 8);
    tabs.set_margin_start(12);
    tabs.set_margin_end(12);
    tabs.set_margin_top(12);
    tabs.set_margin_bottom(12);
    stack.add_named(&tabs, Some("tabs"));
    root.add_overlay(&stack);
    let marker = gtk::Box::new(gtk::Orientation::Vertical, 0);
    marker.set_can_target(false);
    root.add_overlay(&marker);
    let ui = Rc::new(Ui {
        system_animations: gtk::Settings::default().is_some_and(|s| s.is_gtk_enable_animations()),
        sessions: RefCell::new(HashMap::new()),
        application: application.clone(),
        window: window.clone(),
        root: root.clone(),
        stack,
        marker,
        tabs,
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
    ui.place_edge();
    ui.refresh_tabs();
    let motion = gtk::EventControllerMotion::new();
    {
        let weak = Rc::downgrade(&ui);
        motion.connect_enter(move |_, _, _| {
            if let Some(ui) = weak.upgrade()
                && ui.config.borrow().edge_hover_enabled
            {
                ui.reveal();
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
    root.add_controller(motion);
    let click = gtk::GestureClick::new();
    {
        let weak = Rc::downgrade(&ui);
        click.connect_released(move |_, _, _, _| {
            if let Some(ui) = weak.upgrade() {
                ui.reveal();
            }
        });
    }
    root.add_controller(click);
    let drag = gtk::GestureDrag::new();
    let initial = Rc::new(Cell::new(0.));
    {
        let weak = Rc::downgrade(&ui);
        let initial = initial.clone();
        drag.connect_drag_begin(move |gesture, _, _| {
            if let Some(ui) = weak.upgrade() {
                if ui.docked.borrow().is_some() || ui.revealed.get() {
                    gesture.set_state(gtk::EventSequenceState::Denied);
                    return;
                }
                initial.set(ui.config.borrow().marker_offset);
            }
        });
    }
    {
        let weak = Rc::downgrade(&ui);
        drag.connect_drag_end(move |_, x, y| {
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
                    geometry.width() - 720
                } else {
                    geometry.height() - HEIGHT
                };
                if available <= 0 {
                    return;
                }
                config.marker_offset =
                    (initial.get() + distance / f64::from(available)).clamp(0., 1.);
                if config.save(&config::config_path()).is_ok() {
                    ui.config.replace(config);
                    ui.place_edge();
                }
            }
        });
    }
    root.add_controller(drag);
    let keys = gtk::EventControllerKey::new();
    keys.set_propagation_phase(gtk::PropagationPhase::Capture);
    {
        let weak = Rc::downgrade(&ui);
        keys.connect_key_pressed(move |_, key, _, _| {
            if key == gtk::gdk::Key::Escape {
                if let Some(ui) = weak.upgrade() {
                    if ui.docked.borrow().is_some() {
                        ui.close_docked();
                    } else {
                        ui.dormant();
                    }
                }
                glib::Propagation::Stop
            } else {
                glib::Propagation::Proceed
            }
        });
    }
    window.add_controller(keys);
    {
        let weak = Rc::downgrade(&ui);
        window.connect_map(move |_| {
            if let Some(ui) = weak.upgrade() {
                ui.set_input(!ui.revealed.get() && ui.docked.borrow().is_none());
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
                        ui.set_input(!ui.revealed.get() && ui.docked.borrow().is_none());
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
    window.set_child(Some(&root));
    window.present();
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
