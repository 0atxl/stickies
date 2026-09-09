use crate::{
    app::NoteColor,
    appearance,
    config::{Appearance, Config, Font, Paper, Shape, Theme},
    ui_assets,
};
use gtk::prelude::*;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

pub fn build(
    current: Rc<RefCell<Config>>,
    on_change: impl Fn(Config) -> Result<(), String> + 'static,
    on_close: impl Fn() + 'static,
) -> gtk::ScrolledWindow {
    let root = gtk::Box::new(gtk::Orientation::Vertical, 20);
    root.add_css_class("appearance-panel");
    let header = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let title = gtk::Label::new(Some("Appearance"));
    title.add_css_class("panel-title");
    title.set_hexpand(true);
    title.set_xalign(0.);
    let close = ui_assets::icon("chevron", "Close Appearance");
    close.connect_clicked(move |_| on_close());
    header.append(&title);
    header.append(&close);
    root.append(&header);
    let error = gtk::Label::new(None);
    error.set_wrap(true);
    error.add_css_class("save-error");
    error.set_visible(false);
    root.append(&error);
    let update: Rc<dyn Fn(Config)> = Rc::new(move |config| match on_change(config) {
        Ok(()) => error.set_visible(false),
        Err(message) => {
            error.set_text(&message);
            error.set_visible(true);
        }
    });
    // The error label is retained by its container as well as the update callback.
    // Build controls once; changing a theme must not rebuild the focused drawer.
    let config = current.borrow().clone();
    let syncing = Rc::new(Cell::new(false));
    let themes = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    section(&root, "Theme", &themes);
    let mut theme_buttons = Vec::new();
    let mut group: Option<gtk::ToggleButton> = None;
    for (theme, name) in [
        (Theme::Glass, "Glass"),
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
        let paper = appearance::paper_background(
            Rc::new(RefCell::new(sample)),
            Rc::new(Cell::new(NoteColor::Green)),
        );
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
    mode.set_visible(config.theme == Theme::Glass);
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
    section(&pattern_section, "Paper pattern", &patterns);
    pattern_section.set_visible(config.theme != Theme::Glass);
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
        let patterns = pattern_section.clone();
        let syncing = syncing.clone();
        let fonts = font_buttons.clone();
        button.connect_toggled(move |b| {
            if b.is_active() {
                let mut c = current.borrow().clone();
                c.theme = theme;
                mode.set_visible(theme == Theme::Glass);
                patterns.set_visible(theme != Theme::Glass);
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
    switch.connect_active_notify(move |s| {
        let mut c = current.borrow().clone();
        c.reduced_motion = s.is_active();
        update(c);
    });
    row.append(&label);
    row.append(&switch);
    root.append(&row);
    gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .child(&root)
        .build()
}

fn font_index(c: &Config) -> usize {
    match c.font {
        Font::Handwritten => 0,
        Font::Mono => 2,
        Font::Grotesk => 1,
        Font::Theme => {
            if c.theme == Theme::Glass {
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
