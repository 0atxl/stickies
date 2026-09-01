mod app;
mod storage;

use std::{cell::Cell, cell::RefCell, rc::Rc, time::Duration};

use app::{Action, AppState, DeckState, Event, Note, NoteColor, NoteId};
use gtk::gdk::Display;
use gtk::glib;
use gtk::prelude::*;
use gtk4_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};
use storage::Storage;

const APPLICATION_ID: &str = "dev.stickies.Stickies";
const LAYER_NAMESPACE: &str = "stickies";
const DORMANT_WIDTH: i32 = 20;
const DORMANT_HEIGHT: i32 = 180;
const EXPANDED_WIDTH: i32 = 360;
const EDITOR_HEIGHT: i32 = 420;
const TAB_WIDTH: i32 = 160;
const TAB_HEIGHT: i32 = 42;
const TAB_INPUT_WIDTH: i32 = 180;
const TAB_INPUT_HEIGHT: i32 = 310;
const MAX_VISIBLE_NOTES: usize = 5;
const COLLAPSE_DELAY_MS: u64 = 500;
const OPEN_FOCUS_LOSS_DELAY_SECONDS: u64 = 2 * 60;
const INACTIVITY_TIMEOUT_SECONDS: u64 = 5 * 60;
const AUTOSAVE_DELAY_MS: u64 = 750;
const REVEAL_DURATION_MS: u64 = 180;
const TAB_STAGGER_MS: u64 = 35;

#[derive(Clone)]
struct PrototypeUi {
    window: gtk::ApplicationWindow,
    stack: gtk::Stack,
    tabs: gtk::Box,
    tab_revealers: Rc<RefCell<Vec<gtk::Revealer>>>,
    editor_title: gtk::Entry,
    editor_body: gtk::TextView,
    keep_open_button: gtk::ToggleButton,
    state: Rc<RefCell<AppState>>,
    storage: Option<Rc<Storage>>,
    animation_generation: Rc<Cell<u64>>,
    focus_loss_generation: Rc<Cell<u64>>,
    inactivity_source: Rc<RefCell<Option<glib::SourceId>>>,
    autosave_source: Rc<RefCell<Option<glib::SourceId>>>,
    autosave_pending: Rc<Cell<bool>>,
}

impl PrototypeUi {
    fn bump_generation(&self) -> u64 {
        let generation = self.animation_generation.get().wrapping_add(1);
        self.animation_generation.set(generation);
        generation
    }

    fn show_tabs(&self) {
        let event = self.state.borrow_mut().dispatch(Action::ShowTabs);
        if event != Some(Event::Deck(DeckState::Tabs)) {
            return;
        }

        let generation = self.bump_generation();
        self.set_edge_input_region(TAB_INPUT_WIDTH, TAB_INPUT_HEIGHT);
        self.stack.set_visible_child_name("tabs");
        self.stack.set_visible(true);

        for (index, revealer) in self.tab_revealers.borrow().iter().cloned().enumerate() {
            let animation_generation = self.animation_generation.clone();
            glib::timeout_add_local_once(
                Duration::from_millis(index as u64 * TAB_STAGGER_MS),
                move || {
                    if animation_generation.get() == generation {
                        revealer.set_reveal_child(true);
                    }
                },
            );
        }
    }

    fn schedule_collapse(&self) {
        if matches!(self.state.borrow().deck(), DeckState::Open(_)) {
            return;
        }

        let generation = self.bump_generation();
        let ui = self.clone();
        glib::timeout_add_local_once(Duration::from_millis(COLLAPSE_DELAY_MS), move || {
            if ui.animation_generation.get() == generation
                && !matches!(ui.state.borrow().deck(), DeckState::Open(_))
            {
                ui.start_collapse(generation);
            }
        });
    }

    fn start_collapse(&self, generation: u64) {
        self.flush_autosave();
        self.cancel_inactivity_timeout();
        self.cancel_focus_loss_collapse();
        self.state.borrow_mut().dispatch(Action::CollapseDeck);
        self.keep_open_button.set_active(false);
        self.window.set_keyboard_mode(KeyboardMode::None);
        gtk::prelude::RootExt::set_focus(&self.window, None::<&gtk::Widget>);

        for revealer in self.tab_revealers.borrow().iter() {
            revealer.set_reveal_child(false);
        }
        self.stack.set_visible_child_name("tabs");

        let ui = self.clone();
        glib::timeout_add_local_once(Duration::from_millis(REVEAL_DURATION_MS), move || {
            if ui.animation_generation.get() == generation {
                ui.refresh_tabs();
                ui.stack.set_visible(false);
                ui.set_edge_input_region(DORMANT_WIDTH, DORMANT_HEIGHT);
            }
        });
    }

    fn collapse_now(&self) {
        let generation = self.bump_generation();
        self.start_collapse(generation);
    }

    fn open_editor(&self, note_id: NoteId, focus_title: bool) {
        let note = {
            let mut state = self.state.borrow_mut();
            let event = state.dispatch(Action::OpenNote(note_id));
            if event != Some(Event::Deck(DeckState::Open(note_id))) {
                return;
            }
            state.note(note_id).cloned()
        };
        let Some(note) = note else {
            return;
        };

        self.bump_generation();
        self.set_full_input_region();
        self.keep_open_button.set_active(false);
        self.editor_title.set_text(&note.title);
        self.editor_body.buffer().set_text(&note.body);
        self.stack.set_visible_child_name("editor");
        self.stack.set_visible(true);
        self.window.set_keyboard_mode(KeyboardMode::OnDemand);
        self.reset_inactivity_timeout();

        if focus_title {
            let editor_title = self.editor_title.clone();
            glib::idle_add_local_once(move || {
                let _ = editor_title.grab_focus();
                editor_title.select_region(0, -1);
            });
        } else {
            let editor_body = self.editor_body.clone();
            glib::idle_add_local_once(move || {
                let _ = editor_body.grab_focus();
            });
        }
    }

    fn set_keep_open(&self, keep_open: bool) {
        let event = self
            .state
            .borrow_mut()
            .dispatch(Action::SetKeepOpen(keep_open));
        if event != Some(Event::KeepOpen(keep_open)) {
            return;
        }

        if keep_open {
            self.cancel_inactivity_timeout();
            self.cancel_focus_loss_collapse();
        } else {
            self.reset_inactivity_timeout();
            if !self.window.is_active() {
                self.schedule_focus_loss_collapse();
            }
        }
    }

    fn update_open_note_body(&self, body: String) {
        let event = self
            .state
            .borrow_mut()
            .dispatch(Action::UpdateOpenNoteBody(body));
        if matches!(event, Some(Event::NoteEdited(_))) {
            self.schedule_autosave();
        }
    }

    fn update_open_note_title(&self, title: String) {
        let event = self
            .state
            .borrow_mut()
            .dispatch(Action::UpdateOpenNoteTitle(title));
        if matches!(event, Some(Event::NoteEdited(_))) {
            self.schedule_autosave();
        }
    }

    fn create_note(&self) {
        let note = if let Some(storage) = self.storage.as_ref() {
            match storage.create_note("Untitled note", "", NoteColor::Yellow) {
                Ok(note) => note,
                Err(error) => {
                    eprintln!("Stickies could not create a note: {error}");
                    return;
                }
            }
        } else {
            Note::new(
                self.state.borrow().next_note_id(),
                "Untitled note",
                "",
                NoteColor::Yellow,
            )
        };

        let note_id = note.id;
        if self.state.borrow_mut().dispatch(Action::AddNote(note))
            != Some(Event::NoteAdded(note_id))
        {
            return;
        }
        self.open_editor(note_id, true);
    }

    fn archive_open_note(&self) {
        self.remove_open_note(false);
    }

    fn confirm_delete_open_note(&self) {
        let title = {
            let state = self.state.borrow();
            let DeckState::Open(note_id) = state.deck() else {
                return;
            };
            state
                .note(note_id)
                .map_or_else(|| "this note".to_owned(), |note| note.title.clone())
        };
        let dialog = gtk::MessageDialog::builder()
            .transient_for(&self.window)
            .modal(true)
            .message_type(gtk::MessageType::Question)
            .buttons(gtk::ButtonsType::Cancel)
            .text("Delete note?")
            .secondary_text(format!("Delete \"{title}\"? This cannot be undone yet."))
            .build();
        dialog.add_button("Delete", gtk::ResponseType::Accept);

        let ui = self.clone();
        dialog.run_async(move |dialog, response| {
            if response == gtk::ResponseType::Accept {
                ui.remove_open_note(true);
            }
            dialog.close();
        });
    }

    fn remove_open_note(&self, delete: bool) {
        self.flush_autosave();
        if self.autosave_pending.get() {
            return;
        }

        let note_id = match self.state.borrow().deck() {
            DeckState::Open(note_id) => note_id,
            _ => return,
        };
        if let Some(storage) = self.storage.as_ref() {
            let result = if delete {
                storage.delete_note(note_id)
            } else {
                storage.archive_note(note_id)
            };
            if let Err(error) = result {
                eprintln!("Stickies could not remove the note: {error}");
                return;
            }
        }

        let action = if delete {
            Action::DeleteOpenNote
        } else {
            Action::ArchiveOpenNote
        };
        self.state.borrow_mut().dispatch(action);
        self.reload_deck_notes();
        self.collapse_now();
    }

    fn reload_deck_notes(&self) {
        let Some(storage) = self.storage.as_ref() else {
            return;
        };
        match storage.load_deck_notes(MAX_VISIBLE_NOTES) {
            Ok(notes) => self.state.borrow_mut().replace_notes(notes),
            Err(error) => eprintln!("Stickies could not refresh the edge notes: {error}"),
        }
    }

    fn refresh_tabs(&self) {
        while let Some(child) = self.tabs.first_child() {
            self.tabs.remove(&child);
        }
        self.tab_revealers.borrow_mut().clear();

        let notes = self.state.borrow().notes().to_vec();
        for note in notes.into_iter().take(MAX_VISIBLE_NOTES) {
            let tab = gtk::Button::new();
            tab.add_css_class("note-tab");
            tab.add_css_class(match note.color {
                NoteColor::Yellow => "note-yellow",
                NoteColor::Blue => "note-blue",
                NoteColor::Purple => "note-purple",
                NoteColor::Green => "note-green",
            });
            tab.set_size_request(TAB_WIDTH, TAB_HEIGHT);
            tab.set_focus_on_click(false);
            tab.set_tooltip_text(Some(&format!("Open {}", note.title)));

            let visible_title = if note.title.trim().is_empty() {
                "Untitled note"
            } else {
                &note.title
            };
            let title_label = gtk::Label::new(Some(visible_title));
            title_label.add_css_class("tab-title");
            title_label.set_xalign(0.0);
            tab.set_child(Some(&title_label));

            let ui = self.clone();
            let note_id = note.id;
            tab.connect_clicked(move |_| ui.open_editor(note_id, false));
            self.append_tab_widget(&tab);
        }

        let create_button = gtk::Button::with_label("+");
        create_button.add_css_class("create-note");
        create_button.set_tooltip_text(Some("Create note"));
        let ui = self.clone();
        create_button.connect_clicked(move |_| ui.create_note());
        self.append_tab_widget(&create_button);
    }

    fn append_tab_widget(&self, widget: &impl IsA<gtk::Widget>) {
        let revealer = gtk::Revealer::new();
        revealer.set_transition_type(gtk::RevealerTransitionType::SlideLeft);
        revealer.set_transition_duration(REVEAL_DURATION_MS as u32);
        revealer.set_child(Some(widget));
        self.tabs.append(&revealer);
        self.tab_revealers.borrow_mut().push(revealer);
    }

    fn schedule_autosave(&self) {
        if self.storage.is_none() {
            return;
        }

        if let Some(source) = self.autosave_source.borrow_mut().take() {
            source.remove();
        }
        self.autosave_pending.set(true);

        let ui = self.clone();
        let source =
            glib::timeout_add_local_once(Duration::from_millis(AUTOSAVE_DELAY_MS), move || {
                ui.autosave_source.borrow_mut().take();
                ui.save_pending_note();
            });
        self.autosave_source.replace(Some(source));
    }

    fn flush_autosave(&self) {
        if let Some(source) = self.autosave_source.borrow_mut().take() {
            source.remove();
        }
        self.save_pending_note();
    }

    fn save_pending_note(&self) {
        if !self.autosave_pending.get() {
            return;
        }

        let note = {
            let state = self.state.borrow();
            let DeckState::Open(note_id) = state.deck() else {
                return;
            };
            state.note(note_id).cloned()
        };
        let (Some(storage), Some(note)) = (self.storage.as_ref(), note) else {
            return;
        };

        match storage.update_note(&note) {
            Ok(()) => self.autosave_pending.set(false),
            Err(error) => eprintln!("Stickies could not save the open note: {error}"),
        }
    }

    fn reset_inactivity_timeout(&self) {
        self.cancel_inactivity_timeout();

        let state = self.state.borrow();
        if !matches!(state.deck(), DeckState::Open(_))
            || state.keep_open()
            || !self.window.is_active()
        {
            return;
        }
        drop(state);

        let ui = self.clone();
        let source = glib::timeout_add_local_once(
            Duration::from_secs(INACTIVITY_TIMEOUT_SECONDS),
            move || {
                ui.inactivity_source.borrow_mut().take();
                let state = ui.state.borrow();
                let should_collapse =
                    matches!(state.deck(), DeckState::Open(_)) && !state.keep_open();
                drop(state);

                if should_collapse {
                    ui.collapse_now();
                }
            },
        );
        self.inactivity_source.replace(Some(source));
    }

    fn cancel_inactivity_timeout(&self) {
        if let Some(source) = self.inactivity_source.borrow_mut().take() {
            source.remove();
        }
    }

    fn schedule_focus_loss_collapse(&self) {
        self.cancel_inactivity_timeout();

        let state = self.state.borrow();
        if !matches!(state.deck(), DeckState::Open(_)) || state.keep_open() {
            return;
        }
        drop(state);

        let generation = self.focus_loss_generation.get().wrapping_add(1);
        self.focus_loss_generation.set(generation);
        let ui = self.clone();
        glib::timeout_add_local_once(
            Duration::from_secs(OPEN_FOCUS_LOSS_DELAY_SECONDS),
            move || {
                let state = ui.state.borrow();
                let should_collapse = ui.focus_loss_generation.get() == generation
                    && matches!(state.deck(), DeckState::Open(_))
                    && !state.keep_open();
                drop(state);

                if should_collapse {
                    ui.collapse_now();
                }
            },
        );
    }

    fn cancel_focus_loss_collapse(&self) {
        let generation = self.focus_loss_generation.get().wrapping_add(1);
        self.focus_loss_generation.set(generation);
    }

    fn set_edge_input_region(&self, width: i32, height: i32) {
        let Some(surface) = self.window.surface() else {
            return;
        };

        let rectangle = gtk::cairo::RectangleInt::new(
            EXPANDED_WIDTH - width,
            (EDITOR_HEIGHT - height) / 2,
            width,
            height,
        );
        let region = gtk::cairo::Region::create_rectangle(&rectangle);
        surface.set_input_region(Some(&region));
    }

    fn set_full_input_region(&self) {
        if let Some(surface) = self.window.surface() {
            surface.set_input_region(None);
        }
    }
}

fn initial_notes() -> Vec<Note> {
    vec![
        Note::new(
            NoteId::new(1),
            "Work",
            "Review the release checklist.\n\n- Confirm the package build\n- Write the release notes\n- Test the upgrade path",
            NoteColor::Yellow,
        ),
        Note::new(
            NoteId::new(2),
            "Homelab",
            "Move the weekly backups to the new drive, then run a restore test before removing the old copy.",
            NoteColor::Blue,
        ),
        Note::new(
            NoteId::new(3),
            "Projects",
            "The edge stack should open on hover, keep pointer transitions calm, and get out of the way as soon as Escape is pressed.",
            NoteColor::Purple,
        ),
        Note::new(
            NoteId::new(4),
            "Shopping",
            "- Coffee beans\n- AA batteries\n- Two-metre USB-C cable",
            NoteColor::Green,
        ),
    ]
}

fn main() -> glib::ExitCode {
    let application = gtk::Application::builder()
        .application_id(APPLICATION_ID)
        .build();

    application.connect_activate(build_edge_surface);
    application.run()
}

fn build_edge_surface(application: &gtk::Application) {
    let (notes, storage) = load_notes();
    let state = Rc::new(RefCell::new(AppState::new(notes)));

    let window = gtk::ApplicationWindow::builder()
        .application(application)
        .default_width(EXPANDED_WIDTH)
        .default_height(EDITOR_HEIGHT)
        .decorated(false)
        .resizable(false)
        .build();

    window.init_layer_shell();
    window.set_namespace(Some(LAYER_NAMESPACE));
    window.set_layer(Layer::Overlay);
    window.set_anchor(Edge::Right, true);
    window.set_exclusive_zone(0);
    window.set_keyboard_mode(KeyboardMode::None);

    let root = gtk::Overlay::new();
    root.set_size_request(EXPANDED_WIDTH, EDITOR_HEIGHT);

    let hit_area = gtk::Box::new(gtk::Orientation::Vertical, 0);
    hit_area.add_css_class("hit-area");
    root.set_child(Some(&hit_area));

    let stack = gtk::Stack::new();
    stack.set_transition_type(gtk::StackTransitionType::Crossfade);
    stack.set_transition_duration(140);
    stack.set_hhomogeneous(false);
    stack.set_vhomogeneous(false);
    stack.set_size_request(320, 350);
    stack.set_halign(gtk::Align::End);
    stack.set_valign(gtk::Align::Center);
    stack.set_margin_end(16);
    stack.set_visible(false);

    let tabs = gtk::Box::new(gtk::Orientation::Vertical, 8);
    tabs.set_width_request(TAB_WIDTH);
    tabs.set_halign(gtk::Align::End);
    tabs.set_valign(gtk::Align::Center);

    let editor = gtk::Box::new(gtk::Orientation::Vertical, 12);
    editor.add_css_class("editor-card");

    let editor_header = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let editor_title = gtk::Entry::new();
    editor_title.add_css_class("editor-title");
    editor_title.set_hexpand(true);
    editor_title.set_placeholder_text(Some("Note title"));

    let close_hint = gtk::Label::new(Some("Esc to close"));
    close_hint.add_css_class("close-hint");
    editor_header.append(&editor_title);
    editor_header.append(&close_hint);

    let editor_actions = gtk::Box::new(gtk::Orientation::Horizontal, 8);

    let keep_open_button = gtk::ToggleButton::with_label("Keep open");
    keep_open_button.add_css_class("keep-open");
    keep_open_button.set_tooltip_text(Some("Keep this note open while using other windows"));

    let archive_button = gtk::Button::with_label("Archive");
    archive_button.add_css_class("archive-note");
    let delete_button = gtk::Button::with_label("Delete");
    delete_button.add_css_class("delete-note");
    editor_actions.append(&keep_open_button);
    editor_actions.append(&archive_button);
    editor_actions.append(&delete_button);

    let editor_body = gtk::TextView::new();
    editor_body.add_css_class("editor-body");
    editor_body.set_wrap_mode(gtk::WrapMode::WordChar);
    editor_body.set_left_margin(12);
    editor_body.set_right_margin(12);
    editor_body.set_top_margin(12);
    editor_body.set_bottom_margin(12);

    let editor_scroll = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .vexpand(true)
        .child(&editor_body)
        .build();
    editor_scroll.add_css_class("editor-scroll");

    editor.append(&editor_header);
    editor.append(&editor_scroll);
    editor.append(&editor_actions);

    stack.add_named(&tabs, Some("tabs"));
    stack.add_named(&editor, Some("editor"));
    stack.set_visible_child_name("tabs");
    root.add_overlay(&stack);

    let marker = gtk::Box::new(gtk::Orientation::Vertical, 0);
    marker.add_css_class("edge-marker");
    marker.set_size_request(8, DORMANT_HEIGHT);
    marker.set_halign(gtk::Align::End);
    marker.set_valign(gtk::Align::Center);
    marker.set_can_target(false);
    root.add_overlay(&marker);

    let ui = PrototypeUi {
        window: window.clone(),
        stack,
        tabs,
        tab_revealers: Rc::new(RefCell::new(Vec::new())),
        editor_title,
        editor_body,
        keep_open_button: keep_open_button.clone(),
        state,
        storage,
        animation_generation: Rc::new(Cell::new(0)),
        focus_loss_generation: Rc::new(Cell::new(0)),
        inactivity_source: Rc::new(RefCell::new(None)),
        autosave_source: Rc::new(RefCell::new(None)),
        autosave_pending: Rc::new(Cell::new(false)),
    };

    ui.refresh_tabs();

    {
        let ui = ui.clone();
        ui.editor_body.buffer().connect_changed(move |buffer| {
            let (start, end) = buffer.bounds();
            let body = buffer.text(&start, &end, true).to_string();
            ui.update_open_note_body(body);
        });
    }

    {
        let ui = ui.clone();
        let editor_title = ui.editor_title.clone();
        editor_title
            .connect_changed(move |entry| ui.update_open_note_title(entry.text().to_string()));
    }

    {
        let ui = ui.clone();
        keep_open_button.connect_toggled(move |button| ui.set_keep_open(button.is_active()));
    }

    {
        let ui = ui.clone();
        archive_button.connect_clicked(move |_| ui.archive_open_note());
    }

    {
        let ui = ui.clone();
        delete_button.connect_clicked(move |_| ui.confirm_delete_open_note());
    }

    let editor_click = gtk::GestureClick::new();
    {
        let ui = ui.clone();
        editor_click.connect_pressed(move |_, _, _, _| ui.reset_inactivity_timeout());
    }
    editor.add_controller(editor_click);

    let editor_scroll_activity =
        gtk::EventControllerScroll::new(gtk::EventControllerScrollFlags::VERTICAL);
    {
        let ui = ui.clone();
        editor_scroll_activity.connect_scroll(move |_, _, _| {
            ui.reset_inactivity_timeout();
            glib::Propagation::Proceed
        });
    }
    editor.add_controller(editor_scroll_activity);

    let motion = gtk::EventControllerMotion::new();
    {
        let ui = ui.clone();
        motion.connect_enter(move |_, _, _| ui.show_tabs());
    }
    {
        let ui = ui.clone();
        motion.connect_leave(move |_| ui.schedule_collapse());
    }
    root.add_controller(motion);

    let keys = gtk::EventControllerKey::new();
    let key_ui = ui.clone();
    keys.connect_key_pressed(move |_, key, _, _| {
        let note_is_open = matches!(key_ui.state.borrow().deck(), DeckState::Open(_));
        if note_is_open {
            key_ui.reset_inactivity_timeout();
        }

        if key == gtk::gdk::Key::Escape && note_is_open {
            key_ui.collapse_now();
            glib::Propagation::Stop
        } else {
            glib::Propagation::Proceed
        }
    });
    window.add_controller(keys);

    {
        let ui = ui.clone();
        window.connect_is_active_notify(move |window| {
            if window.is_active() {
                ui.cancel_focus_loss_collapse();
                ui.reset_inactivity_timeout();
            } else {
                ui.schedule_focus_loss_collapse();
            }
        });
    }

    {
        let ui = ui.clone();
        window.connect_map(move |_| ui.set_edge_input_region(DORMANT_WIDTH, DORMANT_HEIGHT));
    }

    {
        let ui = ui.clone();
        application.connect_shutdown(move |_| ui.flush_autosave());
    }

    install_styles();
    window.set_child(Some(&root));
    window.present();
}

fn load_notes() -> (Vec<Note>, Option<Rc<Storage>>) {
    let initial_notes = initial_notes();
    let result = Storage::open(&storage::database_path()).and_then(|mut storage| {
        let notes = storage.load_or_seed(&initial_notes, MAX_VISIBLE_NOTES)?;
        Ok((notes, storage))
    });

    match result {
        Ok((notes, storage)) => (notes, Some(Rc::new(storage))),
        Err(error) => {
            eprintln!("Stickies could not open local storage: {error}");
            (initial_notes, None)
        }
    }
}

fn install_styles() {
    let styles = gtk::CssProvider::new();
    styles.load_from_data(
        r#"
        window,
        .hit-area {
            background: transparent;
        }

        .edge-marker {
            background: #f2c14e;
            border-radius: 4px 0 0 4px;
            box-shadow: 0 2px 10px rgba(0, 0, 0, 0.35);
        }

        .note-tab {
            background: rgba(30, 32, 38, 0.97);
            color: #f5f5f5;
            border: 1px solid rgba(255, 255, 255, 0.12);
            border-radius: 10px 0 0 10px;
            padding: 10px 14px;
            box-shadow: 0 8px 24px rgba(0, 0, 0, 0.32);
        }

        .note-tab:hover {
            background: rgba(43, 46, 54, 0.99);
            border-color: rgba(255, 255, 255, 0.24);
        }

        .note-tab.note-yellow { border-left: 4px solid #f2c14e; }
        .note-tab.note-blue { border-left: 4px solid #69a7ff; }
        .note-tab.note-purple { border-left: 4px solid #b08cff; }
        .note-tab.note-green { border-left: 4px solid #70c98b; }

        .tab-title {
            color: #ffffff;
            font-size: 14px;
            font-weight: 700;
        }

        .close-hint {
            color: #8f949e;
            font-size: 10px;
        }

        .keep-open {
            font-size: 11px;
        }

        .editor-card {
            background: rgba(30, 32, 38, 0.98);
            color: #f5f5f5;
            border: 1px solid rgba(255, 255, 255, 0.16);
            border-left: 4px solid #f2c14e;
            border-radius: 14px;
            padding: 16px;
            box-shadow: 0 12px 32px rgba(0, 0, 0, 0.38);
        }

        .editor-title {
            color: #ffffff;
            font-size: 18px;
            font-weight: 700;
        }

        .editor-scroll,
        .editor-body,
        .editor-body text {
            background: rgba(18, 20, 24, 0.82);
            color: #f2f2f2;
            border-radius: 9px;
        }

        .editor-body {
            font-family: monospace;
            font-size: 13px;
        }
        "#,
    );

    gtk::style_context_add_provider_for_display(
        &Display::default().expect("GTK display must be available"),
        &styles,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
}
