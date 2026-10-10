use crate::{
    app::{NoteColor, NoteId},
    appearance,
    config::{Config, Paper, Theme},
    note_controls::NoteControls,
    note_session::NoteSession,
    ui_assets,
};
use gtk::{glib, prelude::*};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::Duration,
};

pub const WIDTH: i32 = 352;
pub const HEIGHT: i32 = 376;
pub const MIN_WIDTH: i32 = 300;
pub const MIN_HEIGHT: i32 = 280;

pub struct Editor {
    pub root: gtk::Overlay,
    pub title: gtk::Entry,
    pub body: gtk::TextView,
    pub close: gtk::Button,
    pub separate: gtk::Button,
    pub keep: gtk::ToggleButton,
    pub resize: gtk::Button,
    toolbar: gtk::Popover,
    more_popover: gtk::Popover,
    docked: bool,
    docked_size: Cell<(i32, i32)>,
    resize_edge: Rc<Cell<gtk::gdk::SurfaceEdge>>,
    syncing_style: Cell<bool>,
    session: Rc<NoteSession>,
    subscription: RefCell<Option<Rc<dyn Fn()>>>,
    config: Rc<RefCell<Config>>,
    resolved: Rc<RefCell<Config>>,
    font_provider: gtk::CssProvider,
    background: gtk::DrawingArea,
    color: Rc<Cell<NoteColor>>,
    toast: adw::ToastOverlay,
    last_error: RefCell<Option<String>>,
    error_toast: RefCell<Option<adw::Toast>>,
    pattern_choice: gtk::DropDown,
    controls: NoteControls,
    timeout: RefCell<Option<glib::SourceId>>,
    on_timeout: Box<dyn Fn()>,
    stopped: Cell<bool>,
}

impl Editor {
    pub fn new(
        session: Rc<NoteSession>,
        config: Rc<RefCell<Config>>,
        docked: bool,
        on_timeout: impl Fn() + 'static,
    ) -> Rc<Self> {
        let root = gtk::Overlay::new();
        root.set_size_request(MIN_WIDTH, MIN_HEIGHT);
        root.set_hexpand(true);
        root.set_vexpand(true);
        root.add_css_class("individual");
        let color = Rc::new(Cell::new(session.color()));
        let style = session.style();
        let resolved = Rc::new(RefCell::new(style.resolve(&config.borrow())));
        let background = appearance::paper_background(
            resolved.clone(),
            color.clone(),
            Rc::new(Cell::new(session.id().value())),
        );
        let sheet = gtk::Overlay::new();
        sheet.set_hexpand(true);
        sheet.set_vexpand(true);
        sheet.set_child(Some(&background));
        let content = gtk::Box::new(gtk::Orientation::Vertical, 8);
        content.add_css_class("editor-card");
        let header = gtk::Box::new(gtk::Orientation::Horizontal, 4);
        let title = gtk::Entry::new();
        title.add_css_class("editor-title");
        title.set_hexpand(true);
        title.set_width_chars(1);
        title.set_placeholder_text(Some("Untitled note"));
        title.set_buffer(&session.title);
        let keep = appearance::toggle(
            "view-pin-symbolic",
            "Pin · Always open, ignore inactivity timeout",
        );
        keep.set_child(Some(&ui_assets::image("pin")));
        keep.update_property(&[gtk::accessible::Property::Label("Always open")]);
        let close = ui_assets::icon("close", "Close note · Escape");
        header.append(&title);
        header.append(&close);
        let body = gtk::TextView::with_buffer(&session.body);
        body.add_css_class("editor-body");
        connect_checklist_backspace(&body);
        body.set_wrap_mode(gtk::WrapMode::WordChar);
        body.set_top_margin(6);
        body.set_bottom_margin(8);
        body.set_pixels_above_lines(4);
        body.set_pixels_below_lines(4);
        body.set_left_margin(0);
        body.set_right_margin(4);
        session.attach_view(&body);
        let scroll = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .vscrollbar_policy(gtk::PolicyType::Automatic)
            .vexpand(true)
            .child(&body)
            .build();
        let checklist = appearance::icon(
            "checkbox-checked-symbolic",
            "Checklist · toggle current item",
        );
        checklist.set_child(Some(&ui_assets::image("checklist")));
        let bullet = appearance::icon("view-list-bullet-symbolic", "Bullet list");
        let numbered = appearance::icon("view-list-ordered-symbolic", "Numbered list");
        let separate = ui_assets::icon("popout", "Open away from edge");
        let controls = NoteControls::new();
        let paper_choice = gtk::DropDown::from_strings(&[
            "Default pattern",
            "Plain",
            "Dots",
            "Square grid",
            "Ruled",
        ]);
        paper_choice.set_tooltip_text(Some(
            "Note canvas pattern; Default pattern follows Appearance",
        ));
        paper_choice.set_selected(style.paper.map_or(0, |paper| paper as u32 + 1));
        let rail = gtk::Box::new(gtk::Orientation::Vertical, 8);
        rail.add_css_class("note-rail");
        rail.set_width_request(120);
        let collapse = ui_assets::icon("chevron", "Collapse note menu");
        let more = gtk::MenuButton::builder()
            .icon_name("view-more-symbolic")
            .tooltip_text("More note tools")
            .build();
        more.add_css_class("icon-control");
        more.set_direction(gtk::ArrowType::Right);
        more.set_child(Some(&ui_assets::image("more")));
        let more_popover = gtk::Popover::new();
        more_popover.set_position(gtk::PositionType::Right);
        let more_tools = gtk::Box::new(gtk::Orientation::Vertical, 6);
        for (button, label) in [
            (&bullet, "Bullet list"),
            (&numbered, "Numbered list"),
            (&separate, "Open away from edge"),
        ] {
            button.set_label(label);
            more_tools.append(button);
            let menu = more_popover.downgrade();
            button.connect_clicked(move |_| {
                if let Some(menu) = menu.upgrade() {
                    menu.popdown();
                }
            });
        }
        more_tools.append(&paper_choice);
        more_popover.set_child(Some(&more_tools));
        more.set_popover(Some(&more_popover));
        let rail_header = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        rail_header.set_halign(gtk::Align::Center);
        for button in [&collapse, keep.upcast_ref(), &checklist] {
            rail_header.append(button);
        }
        rail.append(&rail_header);
        rail.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
        rail.append(&controls.root);
        rail.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
        more.set_halign(gtk::Align::Center);
        rail.append(&more);
        let toolbar = gtk::Popover::new();
        toolbar.set_autohide(false);
        toolbar.set_has_arrow(false);
        toolbar.set_position(gtk::PositionType::Left);
        toolbar.add_css_class("note-tools");
        toolbar.set_child(Some(&rail));
        let toast = adw::ToastOverlay::new();
        toast.set_child(Some(&sheet));
        root.set_child(Some(&toast));
        let grip = gtk::MenuButton::new();
        grip.add_css_class("icon-control");
        grip.add_css_class("rail-grip");
        grip.set_tooltip_text(Some("Open note tools"));
        grip.set_child(Some(&ui_assets::image("more")));
        grip.set_popover(Some(&toolbar));
        header.prepend(&grip);
        let panel = toolbar.downgrade();
        collapse.connect_clicked(move |_| {
            if let Some(panel) = panel.upgrade() {
                panel.popdown();
            }
        });
        let window_handle = gtk::WindowHandle::new();
        window_handle.set_child(Some(&header));
        content.append(&window_handle);
        content.append(&scroll);
        sheet.add_overlay(&content);
        sheet.set_measure_overlay(&content, true);
        let resize = gtk::Button::new();
        resize.add_css_class("note-resize");
        resize.set_tooltip_text(Some("Resize note"));
        resize.set_margin_end(12);
        resize.set_margin_bottom(12);
        resize.set_halign(gtk::Align::End);
        resize.set_valign(gtk::Align::End);
        resize.set_cursor_from_name(Some("se-resize"));
        let resize_edge = Rc::new(Cell::new(gtk::gdk::SurfaceEdge::SouthEast));
        let mark = gtk::DrawingArea::new();
        mark.set_size_request(16, 16);
        let corner = resize_edge.clone();
        mark.set_draw_func(move |area, cr, w, h| {
            let color = area.style_context().color();
            cr.set_source_rgba(
                color.red().into(),
                color.green().into(),
                color.blue().into(),
                1.,
            );
            if corner.get() == gtk::gdk::SurfaceEdge::SouthWest {
                cr.translate(f64::from(w), 0.);
                cr.scale(-1., 1.);
            } else if corner.get() == gtk::gdk::SurfaceEdge::NorthEast {
                cr.translate(0., f64::from(h));
                cr.scale(1., -1.);
            }
            cr.set_line_width(1.4);
            cr.set_line_cap(gtk::cairo::LineCap::Round);
            for inset in [3., 8.] {
                cr.move_to(inset, 13.);
                cr.line_to(13., inset);
            }
            let _ = cr.stroke();
        });
        resize.set_child(Some(&mark));
        sheet.add_overlay(&resize);
        let font_provider = gtk::CssProvider::new();
        appearance::apply_note_font(&font_provider, &resolved.borrow(), &style);
        title
            .style_context()
            .add_provider(&font_provider, gtk::STYLE_PROVIDER_PRIORITY_APPLICATION + 1);
        body.style_context()
            .add_provider(&font_provider, gtk::STYLE_PROVIDER_PRIORITY_APPLICATION + 1);
        let editor = Rc::new(Self {
            root,
            title,
            body,
            close,
            separate,
            keep,
            resize,
            docked_size: Cell::new((WIDTH, HEIGHT)),
            resize_edge,
            syncing_style: Cell::new(false),
            session: session.clone(),
            subscription: RefCell::new(None),
            config,
            resolved,
            font_provider,
            background,
            color,
            toast,
            last_error: RefCell::new(None),
            error_toast: RefCell::new(None),
            toolbar,
            more_popover,
            docked,
            pattern_choice: paper_choice.clone(),
            controls,
            timeout: RefCell::new(None),
            on_timeout: Box::new(on_timeout),
            stopped: Cell::new(false),
        });
        // Popovers handle Escape before the containing window receives it.
        for panel in [&editor.toolbar, &editor.more_popover] {
            let keys = gtk::EventControllerKey::new();
            keys.set_propagation_phase(gtk::PropagationPhase::Capture);
            let weak = Rc::downgrade(&editor);
            keys.connect_key_pressed(move |_, key, _, _| {
                if let Some(editor) = weak.upgrade() {
                    if key == gtk::gdk::Key::Escape {
                        editor.escape();
                        return glib::Propagation::Stop;
                    }
                    editor.activity();
                }
                glib::Propagation::Proceed
            });
            panel.add_controller(keys);
        }
        for (button, size) in &editor.controls.sizes {
            let size = *size;
            let weak = Rc::downgrade(&editor);
            button.connect_toggled(move |button| {
                if let Some(editor) = weak.upgrade()
                    && button.is_active()
                    && !editor.syncing_style.get()
                {
                    let mut style = editor.session.style();
                    style.text_size = size;
                    editor.session.set_style(style);
                    editor.activity();
                }
            });
        }
        for (button, weight) in &editor.controls.weights {
            let weight = *weight;
            let weak = Rc::downgrade(&editor);
            button.connect_toggled(move |button| {
                if let Some(editor) = weak.upgrade()
                    && button.is_active()
                    && !editor.syncing_style.get()
                {
                    let mut style = editor.session.style();
                    style.text_weight = weight;
                    editor.session.set_style(style);
                    editor.activity();
                }
            });
        }
        let weak = Rc::downgrade(&editor);
        paper_choice.connect_selected_notify(move |choice| {
            if let Some(editor) = weak.upgrade()
                && !editor.syncing_style.get()
            {
                let mut style = editor.session.style();
                style.paper = [
                    None,
                    Some(Paper::Plain),
                    Some(Paper::Dots),
                    Some(Paper::Grid),
                    Some(Paper::Ruled),
                ][choice.selected() as usize];
                editor.session.set_style(style);
            }
        });
        let weak = Rc::downgrade(&editor);
        editor.subscription.replace(Some(session.subscribe(move || {
            if let Some(editor) = weak.upgrade() {
                editor.refresh_appearance();
            }
        })));
        {
            let weak = Rc::downgrade(&editor);
            editor.keep.connect_toggled(move |_| {
                if let Some(e) = weak.upgrade() {
                    e.refresh_appearance();
                    e.activity();
                }
            });
        }
        for (button, color) in &editor.controls.colors {
            let color = *color;
            let weak = Rc::downgrade(&editor);
            button.connect_clicked(move |_| {
                if let Some(e) = weak.upgrade() {
                    e.session.set_color(color);
                    e.activity();
                }
            });
        }
        for (button, prefix) in [(checklist, "- [ ] "), (bullet, "- "), (numbered, "1. ")] {
            let weak = Rc::downgrade(&editor);
            button.connect_clicked(move |_| {
                if let Some(e) = weak.upgrade() {
                    e.toggle_line(prefix);
                    e.body.grab_focus();
                }
            });
        }
        let keys = gtk::EventControllerKey::new();
        keys.set_propagation_phase(gtk::PropagationPhase::Capture);
        {
            let weak = Rc::downgrade(&editor);
            keys.connect_key_pressed(move |_, key, _, modifiers| {
                if let Some(e) = weak.upgrade() {
                    e.activity();
                    if key == gtk::gdk::Key::Return
                        && !modifiers.contains(gtk::gdk::ModifierType::SHIFT_MASK)
                        && e.body.has_focus()
                        && e.continue_list()
                    {
                        return glib::Propagation::Stop;
                    }
                }
                glib::Propagation::Proceed
            });
        }
        editor.root.add_controller(keys);
        let click = gtk::GestureClick::new();
        click.set_propagation_phase(gtk::PropagationPhase::Capture);
        {
            let weak = Rc::downgrade(&editor);
            click.connect_pressed(move |_, _, _, _| {
                if let Some(e) = weak.upgrade() {
                    e.activity();
                }
            });
        }
        editor.root.add_controller(click);
        let scrolling = gtk::EventControllerScroll::new(gtk::EventControllerScrollFlags::BOTH_AXES);
        {
            let weak = Rc::downgrade(&editor);
            scrolling.connect_scroll(move |_, _, _| {
                if let Some(e) = weak.upgrade() {
                    e.activity();
                }
                glib::Propagation::Proceed
            });
        }
        editor.root.add_controller(scrolling);
        for target in [
            editor.root.upcast_ref::<gtk::Widget>(),
            editor.toolbar.upcast_ref(),
            editor.more_popover.upcast_ref(),
        ] {
            let motion = gtk::EventControllerMotion::new();
            motion.set_propagation_phase(gtk::PropagationPhase::Capture);
            let weak = Rc::downgrade(&editor);
            motion.connect_enter(move |_, _, _| {
                if let Some(editor) = weak.upgrade() {
                    editor.activity();
                }
            });
            let weak = Rc::downgrade(&editor);
            motion.connect_motion(move |_, _, _| {
                if let Some(editor) = weak.upgrade() {
                    editor.activity();
                }
            });
            target.add_controller(motion);
        }
        editor.refresh_appearance();
        editor.activity();
        editor
    }

    pub fn id(&self) -> NoteId {
        self.session.id()
    }
    pub fn connect_size_changed(&self, callback: impl Fn() + 'static) {
        self.background.connect_resize(move |_, _, _| callback());
    }
    pub fn docked_size(&self) -> (i32, i32) {
        self.docked_size.get()
    }
    pub fn set_docked_size(&self, width: i32, height: i32) {
        let size = (width.max(MIN_WIDTH), height.max(MIN_HEIGHT));
        self.docked_size.set(size);
        self.root.set_size_request(size.0, size.1);
    }
    pub fn set_resize_edge(&self, edge: gtk::gdk::SurfaceEdge) {
        self.toolbar
            .set_position(if edge == gtk::gdk::SurfaceEdge::SouthEast {
                gtk::PositionType::Right
            } else {
                gtk::PositionType::Left
            });
        self.resize_edge.set(edge);
        let left = edge == gtk::gdk::SurfaceEdge::SouthWest;
        let top = edge == gtk::gdk::SurfaceEdge::NorthEast;
        if top {
            self.resize.add_css_class("north-resize");
        } else {
            self.resize.remove_css_class("north-resize");
        }
        self.resize.set_halign(if left {
            gtk::Align::Start
        } else {
            gtk::Align::End
        });
        self.resize.set_valign(if top {
            gtk::Align::Start
        } else {
            gtk::Align::End
        });
        self.resize.set_margin_start(if left { 12 } else { 0 });
        self.resize.set_margin_end(if left {
            0
        } else if top {
            10
        } else {
            12
        });
        self.resize.set_margin_top(if top { 10 } else { 0 });
        self.resize.set_margin_bottom(if top { 0 } else { 12 });
        self.resize.set_cursor_from_name(Some(if left {
            "sw-resize"
        } else if top {
            "ne-resize"
        } else {
            "se-resize"
        }));
        self.resize.queue_draw();
    }
    pub fn flush(&self) -> bool {
        self.session.flush()
    }
    pub fn escape(self: &Rc<Self>) {
        self.activity();
        if self.toolbar.is_visible() || self.more_popover.is_visible() {
            self.more_popover.popdown();
            self.toolbar.popdown();
        } else {
            self.close.emit_clicked();
        }
    }
    pub fn stop(&self) -> bool {
        if !self.session.release_view(&self.body) {
            return false;
        }
        self.toolbar.popdown();
        self.stopped.set(true);
        if let Some(s) = self.timeout.borrow_mut().take() {
            s.remove();
        }
        true
    }
    pub fn refresh_appearance(self: &Rc<Self>) {
        let style = self.session.style();
        self.syncing_style.set(true);
        self.controls
            .sync(&style, self.session.color(), self.config.borrow().theme);
        self.pattern_choice
            .set_selected(style.paper.map_or(0, |p| p as u32 + 1));
        self.syncing_style.set(false);
        self.resolved.replace(style.resolve(&self.config.borrow()));
        appearance::apply_note_font(&self.font_provider, &self.resolved.borrow(), &style);
        self.color.set(self.session.color());
        self.background.queue_draw();
        let config = self.resolved.borrow();
        let (above, below) = match config.theme {
            Theme::Paper => (1, 3),
            Theme::Playful => (1, 3),
            Theme::Minimal => (1, 3),
        };
        self.body.set_pixels_above_lines(above);
        self.body.set_pixels_below_lines(below);
        for (class, enabled) in [
            ("minimal", config.theme == Theme::Minimal),
            (
                "minimal-dark",
                config.theme == Theme::Minimal && appearance::dark(&config),
            ),
            ("playful", config.theme == Theme::Playful),
        ] {
            if enabled {
                self.root.add_css_class(class);
            } else {
                self.root.remove_css_class(class);
            }
        }
        ui_assets::refresh_icons(self.root.upcast_ref(), config.theme == Theme::Playful, true);
        let error = self.session.error();
        if *self.last_error.borrow() != error {
            self.last_error.replace(error.clone());
            if let Some(toast) = self.error_toast.borrow_mut().take() {
                toast.dismiss();
            }
            if let Some(error) = error {
                let toast = adw::Toast::builder().title(error).use_markup(false).build();
                self.toast.add_toast(toast.clone());
                self.error_toast.replace(Some(toast));
            }
        }
    }
    pub fn activity(self: &Rc<Self>) {
        if let Some(source) = self.timeout.borrow_mut().take() {
            source.remove();
        }
        let seconds = self.config.borrow().inactivity_seconds;
        if !self.docked || seconds == 0 || self.keep.is_active() || self.stopped.get() {
            return;
        }
        let weak = Rc::downgrade(self);
        self.timeout.replace(Some(glib::timeout_add_local_once(
            Duration::from_secs(seconds),
            move || {
                if let Some(e) = weak.upgrade() {
                    e.timeout.borrow_mut().take();
                    (e.on_timeout)();
                }
            },
        )));
    }
    fn toggle_line(&self, prefix: &str) {
        toggle_line(&self.body, prefix);
    }
    fn continue_list(&self) -> bool {
        continue_list(&self.body)
    }
}

pub fn connect_checklist_backspace(body: &gtk::TextView) {
    body.connect_backspace(|body| {
        let buffer = body.buffer();
        if !body.is_editable() || buffer.has_selection() {
            return;
        }
        let (mut start, _, text) = current_line(body);
        let cursor = buffer.cursor_position() - start.offset();
        let prefix_len = if start.child_anchor().is_some() { 7 } else { 6 };
        if (text.starts_with("- [ ] ") || text.starts_with("- [x] "))
            && (1..=prefix_len).contains(&cursor)
        {
            let mut end = start;
            if end.child_anchor().is_some() {
                end.forward_char();
            }
            end.forward_chars(6);
            buffer.begin_user_action();
            buffer.delete(&mut start, &mut end);
            buffer.end_user_action();
            body.stop_signal_emission_by_name("backspace");
        }
    });
}

fn current_line(body: &gtk::TextView) -> (gtk::TextIter, gtk::TextIter, String) {
    let buffer = body.buffer();
    let mut start = buffer.iter_at_offset(buffer.cursor_position());
    start.set_line_offset(0);
    let mut end = start;
    end.forward_to_line_end();
    let text = buffer.text(&start, &end, true).to_string();
    (start, end, text)
}
pub fn toggle_line(body: &gtk::TextView, prefix: &str) {
    let buffer = body.buffer();
    let (mut start, mut end, text) = current_line(body);
    let replacement = if prefix == "- [ ] " && text.starts_with("- [ ] ") {
        text.replacen("- [ ] ", "- [x] ", 1)
    } else if prefix == "- [ ] " && text.starts_with("- [x] ") {
        text.replacen("- [x] ", "- [ ] ", 1)
    } else if let Some(rest) = text.strip_prefix(prefix) {
        rest.to_owned()
    } else {
        format!("{prefix}{text}")
    };
    buffer.begin_user_action();
    buffer.delete(&mut start, &mut end);
    buffer.insert(&mut start, &replacement);
    buffer.end_user_action();
}
pub fn continue_list(body: &gtk::TextView) -> bool {
    let buffer = body.buffer();
    if buffer.has_selection() {
        return false;
    }
    let (mut start, mut end, text) = current_line(body);
    if buffer.cursor_position() != end.offset() {
        return false;
    }
    let Some((prefix, content)) = next_list_line(&text) else {
        return false;
    };
    buffer.begin_user_action();
    if content.is_empty() {
        buffer.delete(&mut start, &mut end);
    } else {
        buffer.insert_at_cursor(&format!("\n{prefix}"));
    }
    buffer.end_user_action();
    true
}

fn next_list_line(text: &str) -> Option<(String, &str)> {
    for prefix in ["- [ ] ", "- [x] ", "- ", "* "] {
        if let Some(content) = text.strip_prefix(prefix) {
            return Some((
                if prefix == "- [x] " { "- [ ] " } else { prefix }.into(),
                content,
            ));
        }
    }
    let (number, content) = text.split_once(". ")?;
    let number = number.parse::<u32>().ok()?.checked_add(1)?;
    Some((format!("{number}. "), content))
}

#[cfg(test)]
mod tests {
    use super::next_list_line;
    #[test]
    fn continues_lists_without_completing_the_next_checkbox() {
        assert_eq!(
            next_list_line("- [x] Done"),
            Some(("- [ ] ".into(), "Done"))
        );
        assert_eq!(next_list_line("12. Task"), Some(("13. ".into(), "Task")));
        assert_eq!(next_list_line("- "), Some(("- ".into(), "")));
        assert_eq!(next_list_line("Ordinary text"), None);
    }

    #[test]
    #[ignore = "requires a GTK display; uses a disposable database"]
    fn only_docked_notes_time_out_and_pointer_activity_restarts_the_timer() {
        use super::*;

        crate::ui_assets::register().unwrap();
        adw::init().unwrap();
        let path = std::env::temp_dir().join(format!(
            "stickies-editor-timeout-{}-{}.sqlite",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let storage = Rc::new(crate::storage::Storage::open(&path).unwrap());
        let note = storage
            .create_note("Read me", "Body", NoteColor::Blue)
            .unwrap();
        let session = NoteSession::new(note, storage.clone(), || {});
        let config = Rc::new(RefCell::new(Config::default()));
        let docked_closes = Rc::new(Cell::new(0));
        let count = docked_closes.clone();
        let docked = Editor::new(session.clone(), config.clone(), true, move || {
            count.set(count.get() + 1);
        });
        let separate_closes = Rc::new(Cell::new(0));
        let count = separate_closes.clone();
        let separate = Editor::new(session.clone(), config.clone(), false, move || {
            count.set(count.get() + 1);
        });
        let timer = |editor: &Editor| {
            editor
                .timeout
                .borrow()
                .as_ref()
                .and_then(|id| glib::MainContext::default().find_source_by_id(id))
        };
        assert!(timer(&docked).is_some());
        assert!(timer(&separate).is_none());
        for target in [
            docked.root.upcast_ref::<gtk::Widget>(),
            docked.toolbar.upcast_ref(),
            docked.more_popover.upcast_ref(),
        ] {
            let controllers = target.observe_controllers();
            let motion = (0..controllers.n_items())
                .find_map(|index| {
                    controllers
                        .item(index)
                        .and_downcast::<gtk::EventControllerMotion>()
                })
                .unwrap();
            for signal in ["enter", "motion"] {
                let previous = timer(&docked).unwrap();
                motion.emit_by_name::<()>(signal, &[&0f64, &0f64]);
                assert!(
                    previous.is_destroyed(),
                    "{signal} must cancel the old timer"
                );
                assert!(timer(&docked).is_some());
                assert!(timer(&separate).is_none());
            }
        }
        let previous = timer(&docked).unwrap();
        docked.keep.set_active(true);
        assert!(previous.is_destroyed());
        docked.activity();
        assert!(timer(&docked).is_none());
        docked.keep.set_active(false);
        assert!(timer(&docked).is_some());
        for seconds in [0, 60, 120, 300] {
            config.borrow_mut().inactivity_seconds = seconds;
            docked.activity();
            separate.activity();
            assert!(timer(&separate).is_none());
            if seconds == 0 {
                assert!(timer(&docked).is_none());
            } else {
                let remaining = timer(&docked).unwrap().ready_time() - glib::monotonic_time();
                let expected = seconds as i64 * 1_000_000;
                assert!((expected - 1_000_000..=expected).contains(&remaining));
            }
        }
        // Shorten only the in-memory test config to exercise the actual callback.
        config.borrow_mut().inactivity_seconds = 1;
        docked.activity();
        separate.activity();
        let deadline = std::time::Instant::now() + Duration::from_secs(3);
        while docked_closes.get() == 0 && std::time::Instant::now() < deadline {
            while glib::MainContext::default().pending() {
                glib::MainContext::default().iteration(false);
            }
            std::thread::sleep(Duration::from_millis(1));
        }
        assert_eq!(docked_closes.get(), 1);
        assert_eq!(separate_closes.get(), 0);
        assert!(timer(&docked).is_none());
        assert!(docked.stop());
        docked.activity();
        assert!(timer(&docked).is_none());
        assert!(separate.stop());
        drop(docked);
        drop(separate);
        drop(session);
        drop(storage);
        std::fs::remove_file(path).unwrap();
    }
}
