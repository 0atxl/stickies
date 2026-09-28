use crate::{
    app::{NoteColor, NoteId},
    appearance,
    config::{Config, Font, Paper, Theme},
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

pub struct Editor {
    pub root: gtk::Overlay,
    pub title: gtk::Entry,
    pub body: gtk::TextView,
    pub close: gtk::Button,
    pub separate: gtk::Button,
    pub keep: gtk::ToggleButton,
    pin: gtk::ToggleButton,
    syncing_pin: Cell<bool>,
    session: Rc<NoteSession>,
    subscription: RefCell<Option<Rc<dyn Fn()>>>,
    config: Rc<RefCell<Config>>,
    resolved: Rc<RefCell<Config>>,
    font_provider: gtk::CssProvider,
    background: gtk::DrawingArea,
    color: Rc<Cell<NoteColor>>,
    status: gtk::Label,
    save_indicator: gtk::Stack,
    rail_reveal: gtk::Revealer,
    pattern_choice: gtk::DropDown,
    color_buttons: Vec<(gtk::ToggleButton, NoteColor)>,
    color_label: gtk::Label,
    timeout: RefCell<Option<glib::SourceId>>,
    on_timeout: Box<dyn Fn()>,
    stopped: Cell<bool>,
}

impl Editor {
    pub fn new(
        session: Rc<NoteSession>,
        config: Rc<RefCell<Config>>,
        on_timeout: impl Fn() + 'static,
    ) -> Rc<Self> {
        let pinned = session.pinned();
        let root = gtk::Overlay::new();
        root.set_size_request(WIDTH, HEIGHT);
        root.set_halign(gtk::Align::Start);
        root.set_valign(gtk::Align::Start);
        root.add_css_class("individual");
        let color = Rc::new(Cell::new(session.color()));
        let style = session.style();
        let resolved = Rc::new(RefCell::new(style.resolve(&config.borrow())));
        let background = appearance::paper_background(
            resolved.clone(),
            color.clone(),
            Rc::new(Cell::new(session.id().value())),
        );
        background.set_margin_start(44);
        background.add_css_class("individual-sheet");
        root.set_child(Some(&background));
        let content = gtk::Box::new(gtk::Orientation::Vertical, 8);
        content.add_css_class("editor-card");
        content.set_margin_start(44);
        let header = gtk::Box::new(gtk::Orientation::Horizontal, 4);
        let title = gtk::Entry::new();
        title.add_css_class("editor-title");
        title.set_hexpand(true);
        title.set_width_chars(8);
        title.set_placeholder_text(Some("Untitled note"));
        title.set_buffer(&session.title);
        let keep = appearance::toggle(
            "view-continuous-symbolic",
            "Always open · ignore inactivity timeout",
        );
        keep.set_child(Some(&ui_assets::image("infinity")));
        let close = ui_assets::icon("close", "Close note · Escape");
        header.append(&title);
        let save_indicator = gtk::Stack::new();
        save_indicator.set_transition_type(gtk::StackTransitionType::Crossfade);
        save_indicator.set_transition_duration(140);
        save_indicator.add_css_class("note-save");
        let dots = gtk::Box::new(gtk::Orientation::Horizontal, 2);
        for i in 0..3 {
            let dot = gtk::Label::new(Some("·"));
            dot.add_css_class("note-save-dot");
            dot.add_css_class(&format!("dot-{i}"));
            dots.append(&dot);
        }
        save_indicator.add_named(&dots, Some("saving"));
        save_indicator.add_named(&gtk::Label::new(Some("✓")), Some("saved"));
        save_indicator.add_named(&gtk::Label::new(Some("!")), Some("error"));
        header.append(&save_indicator);
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
            .vscrollbar_policy(gtk::PolicyType::External)
            .vexpand(true)
            .child(&body)
            .build();
        let pin = appearance::toggle("view-pin-symbolic", "Pin · show before unpinned notes");
        pin.set_child(Some(&ui_assets::image("pin")));
        pin.set_active(pinned);
        let checklist = appearance::icon(
            "checkbox-checked-symbolic",
            "Checklist · toggle current item",
        );
        checklist.set_child(Some(&ui_assets::image("checklist")));
        let bullet = appearance::icon("view-list-bullet-symbolic", "Bullet list");
        let numbered = appearance::icon("view-list-ordered-symbolic", "Numbered list");
        let separate = ui_assets::icon("popout", "Open away from edge");
        let palette = gtk::MenuButton::builder()
            .icon_name("applications-graphics-symbolic")
            .tooltip_text("Note appearance")
            .build();
        palette.add_css_class("icon-control");
        palette.set_direction(gtk::ArrowType::Right);
        palette.set_child(Some(&ui_assets::image("palette")));
        let popover = gtk::Popover::new();
        popover.set_position(gtk::PositionType::Right);
        let swatches = gtk::Box::new(gtk::Orientation::Horizontal, 5);
        let mut color_buttons = Vec::new();
        for (color, label) in [
            (NoteColor::Yellow, "Amber"),
            (NoteColor::Blue, "Blue"),
            (NoteColor::Purple, "Lilac"),
            (NoteColor::Green, "Sage"),
        ] {
            let button = gtk::ToggleButton::new();
            button.add_css_class("color-swatch");
            button.add_css_class(appearance::color_class(color));
            button.set_tooltip_text(Some(label));
            button.set_label(if session.color() == color { "✓" } else { "" });
            button.set_active(session.color() == color);
            swatches.append(&button);
            color_buttons.push((button, color));
        }
        let customization = gtk::Box::new(gtk::Orientation::Vertical, 8);
        customization.append(&gtk::Label::new(Some("This note")));
        let color_label = gtk::Label::new(None);
        customization.append(&color_label);
        customization.append(&swatches);
        let font_choice = gtk::DropDown::from_strings(&[
            "Default font",
            "Grotesk",
            "JetBrains Mono",
            "Handwritten",
        ]);
        font_choice.set_selected(match style.font {
            Some(Font::Grotesk) => 1,
            Some(Font::Mono) => 2,
            Some(Font::Handwritten) => 3,
            _ => 0,
        });
        let paper_choice = gtk::DropDown::from_strings(&[
            "Default pattern",
            "Plain",
            "Dots",
            "Square grid",
            "Ruled",
        ]);
        paper_choice.set_selected(style.paper.map_or(0, |paper| paper as u32 + 1));
        customization.append(&font_choice);
        customization.append(&paper_choice);
        popover.set_child(Some(&customization));
        palette.set_popover(Some(&popover));
        let rail = gtk::Box::new(gtk::Orientation::Vertical, 24);
        rail.add_css_class("note-rail");
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
            let menu = more_popover.clone();
            button.connect_clicked(move |_| menu.popdown());
        }
        more_popover.set_child(Some(&more_tools));
        more.set_popover(Some(&more_popover));
        rail.append(&collapse);
        rail.append(&pin);
        rail.append(&keep);
        rail.append(&checklist);
        rail.append(&palette);
        rail.append(&more);
        let rail_reveal = gtk::Revealer::new();
        rail_reveal.set_transition_type(gtk::RevealerTransitionType::SlideRight);
        rail_reveal.set_transition_duration(if config.borrow().reduced_motion {
            0
        } else {
            120
        });
        rail_reveal.set_child(Some(&rail));
        rail_reveal.set_halign(gtk::Align::Start);
        rail_reveal.set_valign(gtk::Align::Start);
        rail_reveal.set_margin_top(12);
        rail_reveal.set_margin_start(8);
        let grip = gtk::Button::new();
        grip.add_css_class("rail-grip");
        grip.set_tooltip_text(Some("Open note menu"));
        grip.set_halign(gtk::Align::Start);
        grip.set_valign(gtk::Align::Start);
        let grip_mark = gtk::Box::new(gtk::Orientation::Vertical, 0);
        grip_mark.add_css_class("rail-grip-mark");
        grip_mark.set_halign(gtk::Align::Center);
        grip_mark.set_valign(gtk::Align::Center);
        grip.set_child(Some(&grip_mark));
        grip.set_margin_start(24);
        grip.set_margin_top(18);
        let reveal = rail_reveal.clone();
        grip.connect_clicked(move |button| {
            reveal.set_reveal_child(true);
            button.set_visible(false);
        });
        let reveal = rail_reveal.clone();
        let handle = grip.clone();
        collapse.connect_clicked(move |_| {
            reveal.set_reveal_child(false);
            handle.set_visible(true);
        });
        let status = gtk::Label::new(None);
        status.add_css_class("save-error");
        status.set_visible(false);
        status.set_wrap(true);
        let window_handle = gtk::WindowHandle::new();
        window_handle.set_child(Some(&header));
        content.append(&window_handle);
        content.append(&scroll);
        content.append(&status);
        root.add_overlay(&content);
        root.set_measure_overlay(&content, true);
        root.add_overlay(&rail_reveal);
        root.add_overlay(&grip);
        let font_provider = gtk::CssProvider::new();
        appearance::apply_note_font(&font_provider, &resolved.borrow());
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
            pin: pin.clone(),
            syncing_pin: Cell::new(false),
            session: session.clone(),
            subscription: RefCell::new(None),
            config,
            resolved,
            font_provider,
            background,
            color,
            status,
            save_indicator,
            rail_reveal,
            pattern_choice: paper_choice.clone(),
            color_buttons,
            color_label,
            timeout: RefCell::new(None),
            on_timeout: Box::new(on_timeout),
            stopped: Cell::new(false),
        });
        for (choice, field) in [(&font_choice, 0), (&paper_choice, 1)] {
            let weak = Rc::downgrade(&editor);
            choice.connect_selected_notify(move |choice| {
                if let Some(editor) = weak.upgrade() {
                    if editor.syncing_pin.get() {
                        return;
                    }
                    let mut style = editor.session.style();
                    let selected = choice.selected() as usize;
                    match field {
                        0 => {
                            style.font = [
                                None,
                                Some(Font::Grotesk),
                                Some(Font::Mono),
                                Some(Font::Handwritten),
                            ][selected]
                        }
                        _ => {
                            style.paper = [
                                None,
                                Some(Paper::Plain),
                                Some(Paper::Dots),
                                Some(Paper::Grid),
                                Some(Paper::Ruled),
                            ][selected]
                        }
                    }
                    editor.session.set_style(style);
                }
            });
        }
        let weak = Rc::downgrade(&editor);
        editor.subscription.replace(Some(session.subscribe(move || {
            if let Some(editor) = weak.upgrade() {
                editor.syncing_pin.set(true);
                let style = editor.session.style();
                font_choice.set_selected(match style.font {
                    Some(Font::Grotesk) => 1,
                    Some(Font::Mono) => 2,
                    Some(Font::Handwritten) => 3,
                    _ => 0,
                });
                paper_choice.set_selected(style.paper.map_or(0, |p| p as u32 + 1));
                editor.syncing_pin.set(false);
                editor.sync_pin();
                editor.refresh_appearance();
                let error = editor.session.error();
                editor.status.set_text(error.as_deref().unwrap_or(""));
                editor.status.set_visible(error.is_some());
            }
        })));
        if let Some(error) = session.error() {
            editor.error(&error);
        }
        {
            let weak = Rc::downgrade(&editor);
            editor.keep.connect_toggled(move |_| {
                if let Some(e) = weak.upgrade() {
                    e.activity();
                }
            });
        }
        {
            let weak = Rc::downgrade(&editor);
            pin.connect_toggled(move |button| {
                if let Some(e) = weak.upgrade() {
                    if e.syncing_pin.get() {
                        return;
                    }
                    e.session.set_pinned(button.is_active());
                    e.activity();
                }
            });
        }
        for (button, color) in &editor.color_buttons {
            let color = *color;
            let weak = Rc::downgrade(&editor);
            let popover = popover.clone();
            button.connect_clicked(move |_| {
                if let Some(e) = weak.upgrade() {
                    e.session.set_color(color);
                }
                popover.popdown();
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
        editor.refresh_appearance();
        editor.activity();
        editor
    }

    pub fn id(&self) -> NoteId {
        self.session.id()
    }
    fn error(&self, message: &str) {
        self.status.set_text(message);
        self.status.set_visible(true);
    }
    pub fn flush(&self) -> bool {
        self.session.flush()
    }
    pub fn stop(&self) {
        self.stopped.set(true);
        if let Some(s) = self.timeout.borrow_mut().take() {
            s.remove();
        }
    }
    pub fn sync_pin(&self) {
        self.syncing_pin.set(true);
        self.pin.set_active(self.session.pinned());
        self.syncing_pin.set(false);
    }
    pub fn refresh_appearance(self: &Rc<Self>) {
        self.resolved
            .replace(self.session.style().resolve(&self.config.borrow()));
        appearance::apply_note_font(&self.font_provider, &self.resolved.borrow());
        self.color.set(self.session.color());
        for (button, color) in &self.color_buttons {
            let selected = *color == self.session.color();
            button.set_active(selected);
            button.set_label(if selected { "✓" } else { "" });
        }
        self.background.queue_draw();
        let config = self.resolved.borrow();
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
        self.color_label.set_text(match config.theme {
            Theme::Paper => "Tag color in All Notes",
            Theme::Minimal => "Top accent color",
            Theme::Playful => "Note fill color",
        });
        self.pattern_choice
            .set_visible(config.theme != Theme::Minimal);
        self.rail_reveal
            .set_transition_duration(if config.reduced_motion { 0 } else { 120 });
        ui_assets::refresh_icons(self.root.upcast_ref(), config.theme == Theme::Playful, true);
        let error = self.session.error();
        self.save_indicator
            .set_visible_child_name(if error.is_some() {
                "error"
            } else if self.session.is_dirty() {
                "saving"
            } else {
                "saved"
            });
        self.save_indicator
            .set_tooltip_text(Some(if error.is_some() {
                "Could not save note"
            } else if self.session.is_dirty() {
                "Saving"
            } else {
                "Saved"
            }));
        if !config.reduced_motion
            && gtk::Settings::default().is_some_and(|s| s.is_gtk_enable_animations())
        {
            self.save_indicator.add_css_class("saving-motion");
        } else {
            self.save_indicator.remove_css_class("saving-motion");
        }
    }
    pub fn activity(self: &Rc<Self>) {
        if let Some(source) = self.timeout.borrow_mut().take() {
            source.remove();
        }
        if self.keep.is_active() || self.stopped.get() {
            return;
        }
        let weak = Rc::downgrade(self);
        self.timeout.replace(Some(glib::timeout_add_local_once(
            Duration::from_secs(self.config.borrow().inactivity_seconds),
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
}
