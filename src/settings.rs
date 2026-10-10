use crate::{
    app::NoteColor,
    appearance,
    config::{Appearance, Config, Font, Paper, Shape, Theme},
};
use gtk::prelude::*;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

pub struct Panel {
    pub root: gtk::ScrolledWindow,
    current: Rc<RefCell<Config>>,
    timeouts: Rc<Vec<gtk::ToggleButton>>,
    syncing: Rc<Cell<bool>>,
}
impl Panel {
    pub fn refresh(&self) {
        self.syncing.set(true);
        let seconds = self.current.borrow().inactivity_seconds;
        for (button, timeout) in self.timeouts.iter().zip([0, 60, 120, 300]) {
            button.set_active(seconds == timeout);
        }
        self.syncing.set(false);
    }
}

pub fn build(
    current: Rc<RefCell<Config>>,
    on_change: impl Fn(Config) -> Result<(), String> + 'static,
) -> Panel {
    let on_change: Rc<dyn Fn(Config) -> Result<(), String>> = Rc::new(on_change);
    let root = gtk::Box::new(gtk::Orientation::Vertical, 20);
    root.add_css_class("appearance-panel");
    let error = gtk::Label::new(None);
    error.set_wrap(true);
    error.add_css_class("save-error");
    error.set_visible(false);
    root.append(&error);
    let samples = Rc::new(RefCell::new(
        Vec::<(Rc<RefCell<Config>>, gtk::DrawingArea)>::new(),
    ));
    let previews = samples.clone();
    let update: Rc<dyn Fn(Config)> = Rc::new(move |config| match on_change(config.clone()) {
        Ok(()) => {
            error.set_visible(false);
            for (sample, drawing) in previews.borrow().iter() {
                let mut sample = sample.borrow_mut();
                sample.appearance = config.appearance;
                sample.paper = config.paper;
                sample.white_paper = config.white_paper;
                drawing.queue_draw();
            }
        }
        Err(message) => {
            error.set_text(&message);
            error.set_visible(true);
        }
    });
    // The error label is retained by its container as well as the update callback.
    // Build controls once; changing a theme must not rebuild the focused drawer.
    let config = current.borrow().clone();
    let syncing = Rc::new(Cell::new(false));
    let heading = gtk::Label::new(Some("Appearance"));
    heading.set_xalign(0.);
    heading.add_css_class("setting-label");
    root.append(&heading);
    let themes = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    section(&root, "Theme", &themes);
    let mut theme_buttons = Vec::new();
    let mut group: Option<gtk::ToggleButton> = None;
    for (theme, name) in [
        (Theme::Minimal, "Minimal"),
        (Theme::Paper, "Paper"),
        (Theme::Playful, "Playful"),
    ] {
        let button = gtk::ToggleButton::new();
        button.add_css_class("theme-choice");
        button.set_hexpand(true);
        button.set_group(group.as_ref());
        button.set_active(config.theme == theme);
        let content = gtk::Box::new(gtk::Orientation::Vertical, 0);
        let overlay = gtk::Overlay::new();
        overlay.add_css_class("theme-sample");
        let sample = Config {
            theme,
            shape: if theme == Theme::Paper {
                Shape::Ticket
            } else {
                Shape::Square
            },
            ..config.clone()
        };
        let sample = Rc::new(RefCell::new(sample));
        let paper = appearance::paper_background(
            sample.clone(),
            Rc::new(Cell::new(NoteColor::Green)),
            Rc::new(Cell::new(1)),
        );
        samples.borrow_mut().push((sample, paper.clone()));
        paper.set_size_request(50, 48);
        overlay.set_child(Some(&paper));
        let badge = gtk::Label::new(Some("✓"));
        badge.add_css_class("selection-badge");
        badge.set_halign(gtk::Align::End);
        badge.set_valign(gtk::Align::Start);
        badge.set_visible(button.is_active());
        overlay.add_overlay(&badge);
        let visible = badge.clone();
        button.connect_toggled(move |b| visible.set_visible(b.is_active()));
        content.append(&overlay);
        let caption = gtk::Label::new(Some(name));
        caption.add_css_class("theme-caption");
        content.append(&caption);
        button.set_child(Some(&content));
        themes.append(&button);
        if group.is_none() {
            group = Some(button.clone());
        }
        theme_buttons.push((button, theme));
    }
    let mode = gtk::Box::new(gtk::Orientation::Vertical, 8);
    let modes = choices(&["System", "Light", "Dark"], config.appearance as usize);
    section(&mode, "Mode", &modes.0);
    mode.set_visible(config.theme == Theme::Minimal);
    root.append(&mode);
    for (i, button) in modes.1.into_iter().enumerate() {
        let current = current.clone();
        let update = update.clone();
        button.connect_toggled(move |b| {
            if b.is_active() {
                let mut c = current.borrow().clone();
                c.appearance = [Appearance::System, Appearance::Light, Appearance::Dark][i];
                update(c);
            }
        });
    }
    let font_section = gtk::Box::new(gtk::Orientation::Vertical, 8);
    let font_header = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let font_label = gtk::Label::new(Some("Font"));
    font_label.add_css_class("setting-label");
    font_label.set_hexpand(true);
    font_label.set_xalign(0.);
    font_header.append(&font_label);
    let reset = gtk::Button::with_label("Reset");
    reset.add_css_class("icon-control");
    reset.set_tooltip_text(Some("Use the theme's default font"));
    font_header.append(&reset);
    font_section.append(&font_header);
    let fonts = choices(&["Handwritten", "Clean", "Mono"], font_index(&config));
    font_section.append(&fonts.0);
    root.append(&font_section);
    let font_buttons = Rc::new(fonts.1);
    for (i, button) in font_buttons.iter().enumerate() {
        let current = current.clone();
        let update = update.clone();
        let syncing = syncing.clone();
        button.connect_toggled(move |b| {
            if b.is_active() && !syncing.get() {
                let mut c = current.borrow().clone();
                c.font = [Font::Handwritten, Font::Grotesk, Font::Mono][i];
                update(c);
            }
        });
    }
    {
        let current = current.clone();
        let update = update.clone();
        let syncing = syncing.clone();
        let buttons = font_buttons.clone();
        reset.connect_clicked(move |_| {
            let mut c = current.borrow().clone();
            c.font = Font::Theme;
            let i = font_index(&c);
            update(c);
            syncing.set(true);
            buttons[i].set_active(true);
            syncing.set(false);
        });
    }
    let patterns = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    let pattern_section = gtk::Box::new(gtk::Orientation::Vertical, 8);
    section(&pattern_section, "Canvas pattern", &patterns);
    root.append(&pattern_section);
    let mut group: Option<gtk::ToggleButton> = None;
    for (pattern, name) in [
        (Paper::Plain, "Plain"),
        (Paper::Dots, "Dots"),
        (Paper::Grid, "Grid"),
        (Paper::Ruled, "Ruled"),
    ] {
        let b = gtk::ToggleButton::with_label(name);
        let sample = gtk::Overlay::new();
        let drawing = gtk::DrawingArea::new();
        drawing.set_content_width(44);
        drawing.set_content_height(20);
        drawing.set_draw_func(move |_, cr, width, height| {
            cr.set_source_rgba(0.35, 0.32, 0.27, 0.12);
            cr.set_line_width(0.5);
            match pattern {
                Paper::Plain => {}
                Paper::Dots => {
                    for x in (3..width).step_by(7) {
                        for y in (3..height).step_by(7) {
                            cr.arc(f64::from(x), f64::from(y), 0.6, 0., std::f64::consts::TAU);
                            let _ = cr.fill();
                        }
                    }
                }
                Paper::Grid | Paper::Ruled => {
                    for y in (4..height).step_by(7) {
                        cr.move_to(0., f64::from(y));
                        cr.line_to(f64::from(width), f64::from(y));
                    }
                    if pattern == Paper::Grid {
                        for x in (3..width).step_by(7) {
                            cr.move_to(f64::from(x), 0.);
                            cr.line_to(f64::from(x), f64::from(height));
                        }
                    }
                    let _ = cr.stroke();
                }
            }
        });
        sample.set_child(Some(&drawing));
        sample.add_overlay(&gtk::Label::new(Some(name)));
        b.set_child(Some(&sample));
        b.add_css_class("pattern-choice");
        b.set_hexpand(true);
        b.set_group(group.as_ref());
        b.set_active(config.paper == pattern);
        let current = current.clone();
        let update = update.clone();
        b.connect_toggled(move |b| {
            if b.is_active() {
                let mut c = current.borrow().clone();
                c.paper = pattern;
                update(c);
            }
        });
        patterns.append(&b);
        if group.is_none() {
            group = Some(b);
        }
    }
    for (button, theme) in theme_buttons {
        let current = current.clone();
        let update = update.clone();
        let mode = mode.clone();
        let syncing = syncing.clone();
        let fonts = font_buttons.clone();
        button.connect_toggled(move |b| {
            if b.is_active() {
                let mut c = current.borrow().clone();
                c.theme = theme;
                mode.set_visible(theme == Theme::Minimal);
                let i = font_index(&c);
                update(c);
                syncing.set(true);
                fonts[i].set_active(true);
                syncing.set(false);
            }
        });
    }
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    let label = gtk::Label::new(Some("Reduce motion"));
    label.set_xalign(0.);
    label.set_hexpand(true);
    let switch = gtk::Switch::new();
    switch.set_valign(gtk::Align::Center);
    switch.set_active(config.reduced_motion);
    let motion_current = current.clone();
    let motion_update = update.clone();
    switch.connect_active_notify(move |s| {
        let mut c = motion_current.borrow().clone();
        c.reduced_motion = s.is_active();
        motion_update(c);
    });
    row.append(&label);
    row.append(&switch);
    root.append(&row);
    root.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
    let heading = gtk::Label::new(Some("Behavior"));
    heading.set_xalign(0.);
    heading.add_css_class("setting-label");
    root.append(&heading);
    let selected = [0, 60, 120, 300]
        .iter()
        .position(|seconds| *seconds == config.inactivity_seconds)
        .unwrap_or(2);
    let timeouts = choices(&["Off", "1 min", "2 min", "5 min"], selected);
    section(&root, "Docked note timeout", &timeouts.0);
    let timeouts = Rc::new(timeouts.1);
    for (index, button) in timeouts.iter().enumerate() {
        let current = current.clone();
        let update = update.clone();
        let syncing = syncing.clone();
        let buttons = Rc::downgrade(&timeouts);
        button.connect_toggled(move |button| {
            if button.is_active() && !syncing.get() {
                let mut config = current.borrow().clone();
                config.inactivity_seconds = [0, 60, 120, 300][index];
                update(config);
                syncing.set(true);
                let seconds = current.borrow().inactivity_seconds;
                if let Some(buttons) = buttons.upgrade() {
                    for (button, timeout) in buttons.iter().zip([0, 60, 120, 300]) {
                        button.set_active(seconds == timeout);
                    }
                }
                syncing.set(false);
            }
        });
    }
    root.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
    let heading = gtk::Label::new(Some("Shortcuts"));
    heading.set_xalign(0.);
    heading.add_css_class("setting-label");
    root.append(&heading);
    let hint = gtk::Label::new(Some(
        "Add these examples to your Hyprland config. Choose keys that suit your bindings.",
    ));
    hint.set_wrap(true);
    hint.set_xalign(0.);
    hint.add_css_class("metadata");
    root.append(&hint);
    for (name, binding) in [
        ("New note", "bind = SUPER, N, exec, stickies --new-note"),
        (
            "Search notes",
            "bind = SUPER, slash, exec, stickies --search",
        ),
        (
            "Toggle edge strip",
            "bind = SUPER, T, exec, stickies --toggle-strip",
        ),
        (
            "All Notes",
            "bind = SUPER, A, exec, stickies --show-all-notes",
        ),
    ] {
        let example = gtk::Box::new(gtk::Orientation::Vertical, 8);
        let header = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        let label = gtk::Label::new(Some(name));
        label.set_xalign(0.);
        label.set_hexpand(true);
        label.add_css_class("setting-label");
        let copy = gtk::Button::with_label("Copy");
        copy.set_tooltip_text(Some(&format!("Copy {name} Hyprland bind")));
        copy.connect_clicked(move |button| button.clipboard().set_text(binding));
        header.append(&label);
        header.append(&copy);
        let command = gtk::Label::new(Some(binding));
        command.set_selectable(true);
        command.set_wrap(true);
        command.set_wrap_mode(gtk::pango::WrapMode::WordChar);
        command.set_max_width_chars(28);
        command.set_xalign(0.);
        command.add_css_class("monospace");
        example.append(&header);
        example.append(&command);
        root.append(&example);
    }
    Panel {
        root: gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .child(&root)
            .build(),
        current,
        timeouts,
        syncing,
    }
}

fn font_index(c: &Config) -> usize {
    match c.font {
        Font::Handwritten => 0,
        Font::Mono => 2,
        Font::Grotesk => 1,
        Font::Theme => {
            if c.theme == Theme::Minimal {
                1
            } else {
                0
            }
        }
    }
}
fn section(root: &gtk::Box, name: &str, control: &impl IsA<gtk::Widget>) {
    let box_ = gtk::Box::new(gtk::Orientation::Vertical, 10);
    let label = gtk::Label::new(Some(name));
    label.set_xalign(0.);
    label.add_css_class("setting-label");
    box_.append(&label);
    box_.append(control);
    root.append(&box_);
}
fn choices(labels: &[&str], selected: usize) -> (gtk::Box, Vec<gtk::ToggleButton>) {
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    row.add_css_class("segments");
    row.set_homogeneous(true);
    let mut buttons = Vec::new();
    for (i, label) in labels.iter().enumerate() {
        let b = gtk::ToggleButton::with_label(label);
        b.set_group(buttons.first());
        b.set_active(i == selected);
        row.append(&b);
        buttons.push(b);
    }
    (row, buttons)
}
