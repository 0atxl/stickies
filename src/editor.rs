use crate::{
    app::{NoteColor, NoteId},
    appearance,
    config::{Config, Font, Paper, Shape},
    note_session::NoteSession,
};
use gtk::{glib, prelude::*};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::Duration,
};

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
        root.set_size_request(400, 420);
        let color = Rc::new(Cell::new(session.color()));
        let style = session.style();
        let resolved = Rc::new(RefCell::new(style.resolve(&config.borrow())));
        let background = appearance::paper_background(resolved.clone(), color.clone());
        background.set_margin_start(36);
        root.set_child(Some(&background));
        let content = gtk::Box::new(gtk::Orientation::Vertical, 8);
        content.add_css_class("editor-card");
        content.set_margin_start(36);
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
        keep.set_label("∞");
        let close = appearance::icon("window-close-symbolic", "Close note · Escape");
        header.append(&title);
        header.append(&close);
        let body = gtk::TextView::with_buffer(&session.body);
        body.add_css_class("editor-body");
        body.set_wrap_mode(gtk::WrapMode::WordChar);
        body.set_top_margin(34);
        body.set_bottom_margin(8);
        body.set_left_margin(4);
        body.set_right_margin(4);
        let scroll = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .vscrollbar_policy(gtk::PolicyType::External)
            .vexpand(true)
            .child(&body)
            .build();
        let pin = appearance::toggle("view-pin-symbolic", "Pin · show before unpinned notes");
        pin.set_active(pinned);
        let checklist = appearance::icon(
            "checkbox-checked-symbolic",
            "Checklist · toggle current item",
        );
        let bullet = appearance::icon("view-list-bullet-symbolic", "Bullet list");
        let numbered = appearance::icon("view-list-ordered-symbolic", "Numbered list");
        let separate = appearance::icon("window-new-symbolic", "Open away from edge");
        let palette = gtk::MenuButton::builder()
            .icon_name("applications-graphics-symbolic")
            .tooltip_text("Note color")
            .build();
        palette.add_css_class("icon-control");
        let popover = gtk::Popover::new();
        let swatches = gtk::Box::new(gtk::Orientation::Horizontal, 5);
        let mut color_buttons = Vec::new();
        for (color, label) in [
            (NoteColor::Yellow, "Amber"),
            (NoteColor::Blue, "Blue"),
            (NoteColor::Purple, "Lilac"),
            (NoteColor::Green, "Sage"),
        ] {
            let button = appearance::icon("media-record-symbolic", label);
            button.add_css_class(appearance::color_class(color));
            swatches.append(&button);
            color_buttons.push((button, color));
        }
        let customization = gtk::Box::new(gtk::Orientation::Vertical, 8);
        customization.append(&gtk::Label::new(Some("This note")));
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
        let shape_choice = gtk::DropDown::from_strings(&[
            "Default edge",
            "Straight",
            "Torn ticket",
            "Uneven paper",
        ]);
        shape_choice.set_selected(style.shape.map_or(0, |shape| shape as u32 + 1));
        customization.append(&font_choice);
        customization.append(&paper_choice);
        customization.append(&shape_choice);
        popover.set_child(Some(&customization));
        palette.set_popover(Some(&popover));
        let rail = gtk::Box::new(gtk::Orientation::Vertical, 12);
        rail.add_css_class("note-rail");
        let collapse = appearance::icon("pan-start-symbolic", "Collapse note menu");
        let more = gtk::MenuButton::builder()
            .icon_name("view-more-symbolic")
            .tooltip_text("More note tools")
            .build();
        more.add_css_class("icon-control");
        let more_popover = gtk::Popover::new();
        let more_tools = gtk::Box::new(gtk::Orientation::Vertical, 6);
        for button in [&bullet, &numbered, &separate] {
            more_tools.append(button);
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
        rail_reveal.set_margin_top(8);
        let grip = gtk::Button::new();
        grip.add_css_class("rail-grip");
        grip.set_tooltip_text(Some("Open note menu"));
        grip.set_halign(gtk::Align::Start);
        grip.set_valign(gtk::Align::Start);
        grip.set_margin_start(30);
        grip.set_margin_top(28);
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
            timeout: RefCell::new(None),
            on_timeout: Box::new(on_timeout),
            stopped: Cell::new(false),
        });
        for (choice, field) in [(&font_choice, 0), (&paper_choice, 1), (&shape_choice, 2)] {
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
                        1 => {
                            style.paper = [
                                None,
                                Some(Paper::Plain),
                                Some(Paper::Dots),
                                Some(Paper::Grid),
                                Some(Paper::Ruled),
                            ][selected]
                        }
                        _ => {
                            style.shape = [
                                None,
                                Some(Shape::Square),
                                Some(Shape::Ticket),
                                Some(Shape::Torn),
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
                shape_choice.set_selected(style.shape.map_or(0, |s| s as u32 + 1));
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
        for (button, color) in color_buttons {
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
        self.background.queue_draw();
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
