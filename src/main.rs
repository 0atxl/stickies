mod app;
mod appearance;
mod autostart;
mod config;
mod edge;
mod editor;
mod manager;
mod motion;
mod note_controls;
mod note_session;
mod platform;
mod settings;
mod storage;
mod ui_assets;

use app::{Action, AppState, NoteColor, NoteId};
use config::Config;
#[cfg(test)]
use config::ScreenEdge;
use edge::{EdgeEvent, EdgeState};
use gtk::{glib, prelude::*};
#[cfg(test)]
use gtk4_layer_shell::{Layer, LayerShell};
#[cfg(test)]
use std::time::Duration;
use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    rc::{Rc, Weak},
};
#[cfg(test)]
use storage::NoteCollection;
use storage::{NoteSort, Storage};

const APPLICATION_ID: &str = "dev.stickies.Stickies";

struct SeparateNote {
    window: gtk::ApplicationWindow,
    editor: Rc<editor::Editor>,
}

struct Ui {
    application: gtk::Application,
    edge: edge::Strip,
    edge_sort: Cell<NoteSort>,
    config: Rc<RefCell<Config>>,
    config_valid: bool,
    system_animations: bool,
    dnd: Cell<bool>,
    dnd_monitor: RefCell<Option<gtk::gio::FileMonitor>>,
    provider: gtk::CssProvider,
    storage: Rc<Storage>,
    sessions: RefCell<HashMap<NoteId, Weak<note_session::NoteSession>>>,
    deck: RefCell<AppState>,
    separate: RefCell<HashMap<NoteId, SeparateNote>>,
    manager: RefCell<Option<manager::ManagerWindow>>,
}

impl Ui {


    fn refresh(self: &Rc<Self>) {
        if let Some(manager) = self.manager.borrow().as_ref() {
            manager.refresh();
        }
        self.refresh_tabs();
    }

    fn create_note(self: &Rc<Self>) {
        self.create_note_with_title("");
    }
    fn create_note_with_title(self: &Rc<Self>, title: &str) {
        self.finish_reposition(false).ok();
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
                if let Some(editor) = self.edge.docked.borrow().as_ref() {
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
        let editor = editor::Editor::new(session, self.config.clone(), !detached, move || {
            if let Some(ui) = timeout.upgrade() {
                ui.close_docked();
            }
        });
        let weak = Rc::downgrade(self);
        editor.close.connect_clicked(move |_| {
            if let Some(ui) = weak.upgrade() {
                if detached {
                    ui.close_separate(id);
                } else {
                    if ui.close_docked() {
                        ui.transition_edge(EdgeEvent::FocusStrip);
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
            self.configure_docked_editor(&editor);
        }
        Some(editor)
    }
    fn open_docked(self: &Rc<Self>, id: NoteId) {
        self.finish_reposition(false).ok();
        if self.edge.state() == EdgeState::Docked(id) {
            return;
        }
        let origin = self.edge.pointer_origin.get();
        if !self.close_docked() {
            return;
        }
        self.edge.pointer_origin.set(origin);
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
        self.finish_panel_motion();
        self.edge.stack.add_named(&editor.root, Some("editor"));
        self.edge.docked.replace(Some(editor.clone()));
        self.transition_edge(EdgeEvent::Dock(id));
        self.edge.pointer.queue_draw();
        self.edge.docked_offset.set(None);
        self.place_edge();
        self.update_visibility();
        self.set_input();
        self.transition_edge(EdgeEvent::FocusWindow);
        editor.body.grab_focus();
    }
    fn close_docked(self: &Rc<Self>) -> bool {
        let editor = self.edge.docked.borrow().clone();
        if let Some(editor) = editor {
            if !editor.flush() {
                return false;
            }
            editor.stop();
            self.edge.docked.borrow_mut().take();
            self.edge.docked_offset.set(None);
            self.deck.borrow_mut().dispatch(Action::CollapseDeck);
            self.edge.pointer_origin.set(None);
            self.transition_edge(EdgeEvent::CloseDocked);
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
        let is_docked = self.edge.state() == EdgeState::Docked(id);
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
            let weak = Rc::downgrade(&editor);
            keys.connect_key_pressed(move |_, key, _, _| {
                if key == gtk::gdk::Key::Escape {
                    if let Some(editor) = weak.upgrade() {
                        editor.escape();
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
            if self.edge.state() == EdgeState::Docked(id) {
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
                ui.finish_reposition(false).ok();
                ui.manager.borrow_mut().take();
            }
            glib::Propagation::Proceed
        });
        let keys = gtk::EventControllerKey::new();
        keys.set_propagation_phase(gtk::PropagationPhase::Capture);
        let weak = Rc::downgrade(self);
        keys.connect_key_pressed(move |_, key, _, _| {
            if key == gtk::gdk::Key::Escape
                && let Some(ui) = weak.upgrade()
                && ui.edge.state().is_repositioning()
            {
                ui.finish_reposition(false).ok();
                return glib::Propagation::Stop;
            }
            glib::Propagation::Proceed
        });
        manager.window().add_controller(keys);
        self.manager.replace(Some(manager.clone()));
        manager.present();
    }

    fn change_settings(self: &Rc<Self>, config: Config) -> Result<(), String> {
        if self.edge.state().is_repositioning() {
            return Err("Choose Done or Cancel before changing Settings.".into());
        }
        if !self.config_valid {
            return Err("The existing configuration is invalid. Correct it before saving settings; it has not been overwritten.".into());
        }
        config
            .save(&config::config_path())
            .map_err(|e| e.to_string())?;
        let login_changed = self.config.borrow().launch_at_login != config.launch_at_login;
        let timeout_changed = self.config.borrow().inactivity_seconds != config.inactivity_seconds;
        if self.config.borrow().edge != config.edge
            || self.config.borrow().marker_offset != config.marker_offset
        {
            self.edge.docked_offset.set(None);
            self.edge.strip_offset.set(None);
        }
        self.finish_edge_motion();
        self.config.replace(config.clone());
        if let Some(settings) = gtk::Settings::default() {
            settings.set_gtk_enable_animations(self.system_animations && !config.reduced_motion);
        }
        appearance::apply(&self.provider, &config);
        self.place_edge();
        self.update_visibility();
        self.refresh_tabs();
        if let Some(editor) = self.edge.docked.borrow().as_ref() {
            editor.refresh_appearance();
            if timeout_changed {
                editor.activity();
            }
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
    let edge_provider = gtk::CssProvider::new();
    edge_provider.load_from_data(include_str!("../assets/edge-strip.css"));
    gtk::style_context_add_provider_for_display(
        &gtk::gdk::Display::default().expect("GTK display"),
        &edge_provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION + 1,
    );
    let edge = edge::Strip::new(application);
    let window = edge.window.clone();
    let strip_window = edge.strip_window.clone();
    let ui = Rc::new(Ui {
        system_animations: gtk::Settings::default().is_some_and(|s| s.is_gtk_enable_animations()),
        sessions: RefCell::new(HashMap::new()),
        application: application.clone(),
        edge,
        edge_sort: Cell::new(NoteSort::Recent),
        config: Rc::new(RefCell::new(config)),
        config_valid: valid,
        dnd: Cell::new(false),
        dnd_monitor: RefCell::new(None),
        provider,
        storage,
        deck: RefCell::new(AppState::new(Vec::new())),
        separate: RefCell::new(HashMap::new()),
        manager: RefCell::new(None),
    });
    if let Some(settings) = gtk::Settings::default() {
        settings
            .set_gtk_enable_animations(ui.system_animations && !ui.config.borrow().reduced_motion);
    }
    ui.connect_edge();
    // Keep the coordinator alive for the application lifetime, without widget cycles.
    let shutdown_ui = ui.clone();
    application.connect_shutdown(move |_| {
        let ui = &shutdown_ui;
        let manager = ui.manager.borrow().clone();
        if let Some(manager) = manager {
            manager.flush();
        }
        if let Some(editor) = ui.edge.docked.borrow().as_ref() {
            editor.stop();
        }
        for note in ui.separate.borrow().values() {
            note.editor.stop();
        }
    });
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
        for widget in [&ui.edge.root, &ui.edge.strip_root] {
            gtk::WidgetPaintable::new(Some(widget)).snapshot(
                &snapshot,
                f64::from(widget.width()),
                f64::from(widget.height()),
            );
        }
        let node = snapshot.to_node().unwrap();
        let (sx, sy, sw, sh) = ui.edge.strip_bounds.get();
        let (x, y, w, h) = if ui.edge.docked.borrow().is_some()
            || ui.edge.state().is_searching()
            || ui.edge.reposition_offset().is_some()
        {
            let (px, py, pw, ph) = ui.edge.panel_bounds.get();
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
            .edge
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
        let editor = ui.edge.docked.borrow().clone().unwrap();
        assert!(editor.root.is_mapped());
        let snapshot = gtk::Snapshot::new();
        gtk::WidgetPaintable::new(Some(&ui.edge.root)).snapshot(
            &snapshot,
            f64::from(ui.edge.root.width()),
            f64::from(ui.edge.root.height()),
        );
        let bounds = editor.root.compute_bounds(&ui.edge.root).unwrap();
        let node = snapshot.to_node().unwrap();
        ui.edge
            .window
            .renderer()
            .unwrap()
            .render_texture(&node, Some(&bounds))
            .save_to_png(std::path::Path::new(&directory).join(format!("{name}.png")))
            .unwrap();
    }

    fn find_widget(
        root: &gtk::Widget,
        predicate: &impl Fn(&gtk::Widget) -> bool,
    ) -> Option<gtk::Widget> {
        if predicate(root) {
            return Some(root.clone());
        }
        let mut child = root.first_child();
        while let Some(widget) = child {
            if let Some(found) = find_widget(&widget, predicate) {
                return Some(found);
            }
            child = widget.next_sibling();
        }
        None
    }
    fn position_group(root: &gtk::Widget, name: &str) -> Vec<gtk::ToggleButton> {
        let label = find_widget(root, &|widget| {
            widget
                .downcast_ref::<gtk::Label>()
                .is_some_and(|label| label.text() == name)
        })
        .unwrap();
        let row = label.parent().unwrap().last_child().unwrap();
        let mut buttons = Vec::new();
        let mut child = row.first_child();
        while let Some(widget) = child {
            buttons.push(widget.clone().downcast::<gtk::ToggleButton>().unwrap());
            child = widget.next_sibling();
        }
        buttons
    }
    fn button(root: &gtk::Widget, name: &str) -> gtk::Button {
        find_widget(root, &|widget| {
            widget.downcast_ref::<gtk::Button>().is_some_and(|button| {
                button.label().as_deref() == Some(name)
                    || button.tooltip_text().as_deref() == Some(name)
            })
        })
        .unwrap()
        .downcast()
        .unwrap()
    }



    #[test]
    #[ignore = "requires Wayland and disposable XDG_DATA_HOME/XDG_CONFIG_HOME directories"]
    fn escape_dismisses_individual_popovers_before_closing_and_retains_failed_edits() {
        for variable in ["XDG_DATA_HOME", "XDG_CONFIG_HOME", "XDG_CACHE_HOME"] {
            assert!(
                std::path::PathBuf::from(std::env::var(variable).expect(variable))
                    .starts_with(std::env::temp_dir())
            );
        }
        ui_assets::register().unwrap();
        adw::init().unwrap();
        Config::default().save(&config::config_path()).unwrap();
        let app = gtk::Application::builder()
            .application_id(APPLICATION_ID)
            .flags(gtk::gio::ApplicationFlags::NON_UNIQUE)
            .build();
        app.register(None::<&gtk::gio::Cancellable>).unwrap();
        let storage = Storage::open(&storage::database_path()).unwrap();
        let id = storage
            .create_note("Escape", "Keep", NoteColor::Blue)
            .unwrap()
            .id;
        let ui = build(&app).unwrap();
        let settle = || {
            let end = std::time::Instant::now() + Duration::from_millis(180);
            while std::time::Instant::now() < end {
                while glib::MainContext::default().pending() {
                    glib::MainContext::default().iteration(false);
                }
                std::thread::sleep(Duration::from_millis(1));
            }
        };
        let escape = |target: &gtk::Widget| {
            let controllers = target.observe_controllers();
            let handled = (0..controllers.n_items()).any(|index| {
                controllers
                    .item(index)
                    .and_downcast::<gtk::EventControllerKey>()
                    .is_some_and(|keys| {
                        keys.emit_by_name::<bool>(
                            "key-pressed",
                            &[
                                &gtk::gdk::Key::Escape,
                                &0u32,
                                &gtk::gdk::ModifierType::empty(),
                            ],
                        )
                    })
            });
            assert!(handled);
        };
        let popover = |editor: &editor::Editor, name: &str| {
            find_widget(editor.root.upcast_ref(), &|widget| {
                widget.is::<gtk::MenuButton>() && widget.tooltip_text().as_deref() == Some(name)
            })
            .unwrap()
            .downcast::<gtk::MenuButton>()
            .unwrap()
            .popover()
            .unwrap()
        };
        for detached in [false, true] {
            let open = || {
                let (editor, window) = if detached {
                    ui.open_separate(id);
                    let separate = ui.separate.borrow();
                    let note = separate.get(&id).unwrap();
                    (
                        note.editor.clone(),
                        note.window.clone().upcast::<gtk::Widget>(),
                    )
                } else {
                    ui.open_docked(id);
                    (
                        ui.edge.docked.borrow().clone().unwrap(),
                        ui.edge.window.clone().upcast(),
                    )
                };
                editor.keep.set_active(true);
                settle();
                (editor, window)
            };
            let is_open = || {
                if detached {
                    ui.separate.borrow().contains_key(&id)
                } else {
                    ui.edge.docked.borrow().is_some()
                }
            };
            for on_popover in [false, true] {
                for nested in [false, true] {
                    let (editor, window) = open();
                    let tools = popover(&editor, "Open note tools");
                    let more = popover(&editor, "More note tools");
                    tools.popup();
                    settle();
                    if nested {
                        more.popup();
                        settle();
                    }
                    let target = if on_popover {
                        if nested {
                            more.upcast_ref()
                        } else {
                            tools.upcast_ref()
                        }
                    } else {
                        &window
                    };
                    escape(target);
                    settle();
                    assert!(is_open(), "first Escape must preserve the note");
                    assert!(!tools.is_visible());
                    assert!(!more.is_visible());
                    escape(&window);
                    assert!(!is_open(), "second Escape must close an Always open note");
                }
            }
            let (editor, window) = open();
            escape(&window);
            assert!(!is_open(), "Escape without a popover must close the note");
            drop(editor);
            let (editor, window) = open();
            let connection = rusqlite::Connection::open(storage::database_path()).unwrap();
            connection.execute_batch("CREATE TRIGGER reject_escape_save BEFORE UPDATE ON notes BEGIN SELECT RAISE(FAIL, 'save failure'); END;").unwrap();
            let unsaved_title = format!("Unsaved title {detached}");
            editor.title.set_text(&unsaved_title);
            let tools = popover(&editor, "Open note tools");
            tools.popup();
            settle();
            escape(&window);
            settle();
            assert!(!tools.is_visible());
            escape(&window);
            assert!(is_open(), "failed saves must retain the editor");
            assert_eq!(editor.title.text(), unsaved_title);
            connection
                .execute_batch("DROP TRIGGER reject_escape_save;")
                .unwrap();
            escape(&window);
            assert!(!is_open());
            assert_eq!(storage.load_note(id).unwrap().title, unsaved_title);
        }
        ui.edge.window.close();
        ui.edge.strip_window.close();
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
            let deadline = end + Duration::from_secs(2);
            while std::time::Instant::now() < end || ui.edge_motion_running() {
                assert!(
                    std::time::Instant::now() < deadline,
                    "edge motion did not settle"
                );
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
                let extent = (ui.edge.root.width(), ui.edge.root.height());
                let tab = ui.edge.tabs.first_child().unwrap().first_child().unwrap();
                let horizontal = matches!(edge, ScreenEdge::Top | ScreenEdge::Bottom);
                let bounds = tab.compute_bounds(&ui.edge.strip_root).unwrap();
                assert_eq!(bounds.height() as i32, if horizontal { 24 } else { 80 });
                if horizontal {
                    assert!((80..=112).contains(&(bounds.width() as i32)));
                    assert!(tab.first_child().unwrap().is::<gtk::Label>());
                } else {
                    assert_eq!(bounds.width() as i32, 24);
                }
                let utilities = ui.edge.tabs.last_child().unwrap();
                let mut control = utilities.first_child();
                while let Some(button) = control {
                    let bounds = button.compute_bounds(&ui.edge.strip_root).unwrap();
                    assert_eq!(bounds.width() as i32, 24);
                    assert_eq!(bounds.height() as i32, 24);
                    control = button.next_sibling();
                }
                for _ in 0..3 {
                    ui.show_edge_search();
                    settle();
                    capture(&ui, &format!("{prefix}-search"));
                    assert!(ui.edge.tabs.is_mapped());
                    let current = ui
                        .edge
                        .tabs
                        .first_child()
                        .unwrap()
                        .first_child()
                        .unwrap()
                        .compute_bounds(&ui.edge.strip_root)
                        .unwrap();
                    assert_eq!(current, bounds);
                    assert!(!ui.edge.stack.is_transition_running());
                    ui.close_edge_search();
                    ui.open_docked(ids[0]);
                    settle();
                    capture(&ui, &format!("{prefix}-note"));
                    let editor = ui.edge.docked.borrow().clone().unwrap();
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
                    assert!(ui.edge.tabs.is_mapped());
                    let current = ui
                        .edge
                        .tabs
                        .first_child()
                        .unwrap()
                        .first_child()
                        .unwrap()
                        .compute_bounds(&ui.edge.strip_root)
                        .unwrap();
                    assert_eq!(current, bounds);
                    assert!(!ui.edge.stack.is_transition_running());
                    if edge == ScreenEdge::Right {
                        assert_eq!(ui.edge.root.width(), extent.0);
                    } else if edge == ScreenEdge::Bottom {
                        assert_eq!(ui.edge.root.height(), extent.1);
                    }
                    assert!(ui.close_docked());
                    settle();
                    assert!(!ui.edge.stack.is_transition_running());
                    assert_eq!((ui.edge.root.width(), ui.edge.root.height()), extent);
                    let tab = ui.edge.tabs.first_child().unwrap().first_child().unwrap();
                    let bounds = tab.compute_bounds(&ui.edge.strip_root).unwrap();
                    if edge == ScreenEdge::Right {
                        assert_eq!((bounds.x() + bounds.width()) as i32, extent.0);
                    } else if edge == ScreenEdge::Bottom {
                        assert_eq!((bounds.y() + bounds.height()) as i32, extent.1);
                    }
                }
            }
        }
        // Replacement and closure must retain the dirty editor on save failure.
        ui.open_docked(ids[0]);
        let editor = ui.edge.docked.borrow().clone().unwrap();
        editor
            .body
            .buffer()
            .set_text("Must survive failed replacement");
        let connection = rusqlite::Connection::open(storage::database_path()).unwrap();
        connection.execute_batch("CREATE TRIGGER reject_edge_edit BEFORE UPDATE ON notes BEGIN SELECT RAISE(FAIL, 'edge save failure'); END;").unwrap();
        ui.show_edge_search();
        assert!(!ui.edge.state().is_searching());
        ui.open_docked(ids[1]);
        assert_eq!(ui.edge.docked.borrow().as_ref().unwrap().id(), ids[0]);
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
        assert!(ui.begin_reposition().is_err());
        assert!(ui.edge.reposition_offset().is_none());
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
                let strip = ui.edge.strip_bounds.get();
                ui.open_docked(ids[0]);
                let editor = ui.edge.docked.borrow().clone().unwrap();
                for size in [
                    (editor::WIDTH, editor::HEIGHT),
                    (editor::MIN_WIDTH, editor::MIN_HEIGHT),
                    (700, 600),
                ] {
                    editor.set_docked_size(size.0, size.1);
                    ui.place_edge();
                    settle();
                    assert_eq!(ui.edge.strip_bounds.get(), strip);
                    assert_eq!((editor.root.width(), editor.root.height()), size);
                    let (x, y, w, h) = ui.edge.panel_bounds.get();
                    let (sw, sh) = ui.note_limits();
                    assert!(x >= 0 && y >= 0 && x + w <= sw && y + h <= sh);
                    assert!(!ui.edge.pointer.can_target());
                }
                ui.show_edge_search();
                settle();
                assert!(ui.edge.state().is_searching());
                assert!(ui.edge.tabs.is_mapped());
                assert_eq!(ui.edge.strip_bounds.get(), strip);
                let (x, y, w, h) = ui.edge.panel_bounds.get();
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
            assert!(ui.edge.window.is_visible());
            assert!(!ui.edge.strip_window.is_visible());
            assert_eq!(ui.edge.window.layer(), Layer::Overlay);
            assert_eq!(ui.edge.strip_window.layer(), Layer::Top);
            ui.dnd.set(false);
            ui.update_visibility();
            ui.show_edge_search();
            assert_eq!(ui.edge.window.layer(), Layer::Top);
            settle();
            assert!(
                gtk::prelude::GtkWindowExt::focus(&ui.edge.window)
                    .is_some_and(|focus| focus.is_ancestor(&ui.edge.edge_search))
            );
            assert_eq!(ui.edge.stack.transition_duration(), 0);
            ui.close_edge_search();
        }
        let overflow = ui
            .storage
            .create_note("Overflow target", "unique body needle", NoteColor::Purple)
            .unwrap();
        ui.refresh_tabs();
        ui.show_edge_search();
        ui.edge.edge_search.set_text("unique body needle");
        ui.refresh_edge_search();
        assert_eq!(*ui.edge.search_ids.borrow(), vec![overflow.id]);
        ui.activate_edge_search();
        assert_eq!(ui.edge.docked.borrow().as_ref().unwrap().id(), overflow.id);
        ui.show_edge_search();
        ui.edge.edge_search.set_text("New unmatched title");
        ui.refresh_edge_search();
        assert!(ui.edge.search_create.is_sensitive());
        ui.activate_edge_search();
        let created = ui.edge.docked.borrow().as_ref().unwrap().id();
        assert_eq!(
            ui.storage.load_note(created).unwrap().title,
            "New unmatched title"
        );
        ui.close_docked();
        ui.create_note();
        let blank = ui.edge.docked.borrow().as_ref().unwrap().id();
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
                    let (x, y, w, h) = ui.edge.strip_bounds.get();
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
        let strip_before_edit = ui.edge.strip_bounds.get();
        ui.open_docked(short);
        let editor = ui.edge.docked.borrow().clone().unwrap();
        editor
            .title
            .set_text("A much longer title must not move the horizontal strip while editing");
        assert!(editor.flush());
        settle();
        assert_eq!(ui.edge.strip_bounds.get(), strip_before_edit);
        assert!(ui.close_docked());
        ui.edge.strip_window.close();
        ui.edge.window.close();
    }
}
