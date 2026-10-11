use crate::{
    app::NoteColor,
    appearance,
    config::{Appearance, Config, Font, Paper, ScreenEdge, Shape, Theme},
};
use gtk::prelude::*;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

pub struct Panel {
    pub root: gtk::ScrolledWindow,
    position: Rc<PositionControls>,
    timeouts: Rc<Vec<gtk::ToggleButton>>,
    syncing: Rc<Cell<bool>>,
    appearance: Rc<AppearanceControls>,
}
impl Panel {
    pub fn refresh(&self) {
        self.appearance.sync();
        self.position.sync();
        self.syncing.set(true);
        let seconds = self.position.current.borrow().inactivity_seconds;
        for (button, timeout) in self.timeouts.iter().zip([0, 60, 120, 300]) {
            button.set_active(seconds == timeout);
        }
        self.syncing.set(false);
    }
}

struct AppearanceControls {
    current: Rc<RefCell<Config>>,
    syncing: Rc<Cell<bool>>,
    themes: Vec<(gtk::ToggleButton, Theme)>,
    mode: gtk::Box,
    modes: Vec<gtk::ToggleButton>,
    fonts: Vec<gtk::ToggleButton>,
    patterns: Vec<(gtk::ToggleButton, Paper)>,
    motion: gtk::Switch,
    samples: Vec<(Rc<RefCell<Config>>, gtk::DrawingArea)>,
}
impl AppearanceControls {
    fn sync(&self) {
        let config = self.current.borrow().clone();
        let was_syncing = self.syncing.replace(true);
        for (button, theme) in &self.themes {
            button.set_active(config.theme == *theme);
        }
        self.mode.set_visible(config.theme == Theme::Minimal);
        for (index, button) in self.modes.iter().enumerate() {
            button.set_active(config.appearance as usize == index);
        }
        for (index, button) in self.fonts.iter().enumerate() {
            button.set_active(font_index(&config) == index);
        }
        for (button, pattern) in &self.patterns {
            button.set_active(config.paper == *pattern);
        }
        self.motion.set_active(config.reduced_motion);
        for (sample, drawing) in &self.samples {
            let mut sample = sample.borrow_mut();
            sample.appearance = config.appearance;
            sample.paper = config.paper;
            sample.white_paper = config.white_paper;
            drawing.queue_draw();
        }
        self.syncing.set(was_syncing);
    }
}

struct PositionControls {
    root: gtk::Box,
    edges: Vec<gtk::ToggleButton>,
    presets: Vec<gtk::ToggleButton>,
    error: gtk::Label,
    current: Rc<RefCell<Config>>,
    syncing: Cell<bool>,
}
impl PositionControls {
    fn sync(&self) {
        self.syncing.set(true);
        let config = self.current.borrow();
        for (button, edge) in self.edges.iter().zip([
            ScreenEdge::Left,
            ScreenEdge::Right,
            ScreenEdge::Top,
            ScreenEdge::Bottom,
        ]) {
            button.set_active(config.edge == edge);
        }
        let labels = if matches!(config.edge, ScreenEdge::Top | ScreenEdge::Bottom) {
            ["Left", "Center", "Right"]
        } else {
            ["Top", "Center", "Bottom"]
        };
        for (index, button) in self.presets.iter().enumerate() {
            button.set_label(labels[index]);
            button.set_active((config.marker_offset - index as f64 / 2.).abs() < 0.000001);
        }
        self.syncing.set(false);
    }
    fn result(&self, result: Result<(), String>) {
        if let Err(message) = result {
            self.error.set_text(&message);
            self.error.set_visible(true);
        } else {
            self.error.set_visible(false);
        }
        self.sync();
    }
    fn new(
        current: Rc<RefCell<Config>>,
        change: Rc<dyn Fn(Config) -> Result<(), String>>,
        reposition: impl Fn() -> Result<(), String> + 'static,
    ) -> Rc<Self> {
        let root = gtk::Box::new(gtk::Orientation::Vertical, 12);
        let edges = choices(&["Left", "Right", "Top", "Bottom"], 0);
        let presets = choices(&["Top", "Center", "Bottom"], 1);
        section(&root, "Screen edge", &edges.0);
        section(&root, "Position", &presets.0);
        let hint = gtk::Label::new(Some(
            "Choose a position or drag the strip to place it precisely.",
        ));
        hint.set_wrap(true);
        hint.set_xalign(0.);
        hint.add_css_class("metadata");
        root.append(&hint);
        let move_button = gtk::Button::with_label("Reposition…");
        move_button.set_tooltip_text(Some("Drag the strip, then choose Done or Cancel"));
        root.append(&move_button);
        let error = gtk::Label::new(None);
        error.set_wrap(true);
        error.set_max_width_chars(28);
        error.add_css_class("save-error");
        error.set_visible(false);
        root.append(&error);
        let controls = Rc::new(Self {
            root,
            edges: edges.1,
            presets: presets.1,
            error,
            current,
            syncing: Cell::new(false),
        });
        controls.sync();
        for (index, button) in controls.edges.iter().enumerate() {
            let weak = Rc::downgrade(&controls);
            let change = change.clone();
            button.connect_toggled(move |button| {
                if let Some(controls) = weak.upgrade()
                    && !controls.syncing.get()
                    && button.is_active()
                {
                    let mut config = controls.current.borrow().clone();
                    config.edge = [
                        ScreenEdge::Left,
                        ScreenEdge::Right,
                        ScreenEdge::Top,
                        ScreenEdge::Bottom,
                    ][index];
                    controls.result(change(config));
                }
            });
        }
        for (index, button) in controls.presets.iter().enumerate() {
            let weak = Rc::downgrade(&controls);
            let change = change.clone();
            button.connect_toggled(move |button| {
                if let Some(controls) = weak.upgrade()
                    && !controls.syncing.get()
                    && button.is_active()
                {
                    let mut config = controls.current.borrow().clone();
                    config.marker_offset = index as f64 / 2.;
                    controls.result(change(config));
                }
            });
        }
        let weak = Rc::downgrade(&controls);
        move_button.connect_clicked(move |_| {
            if let Some(controls) = weak.upgrade() {
                controls.result(reposition());
            }
        });
        controls
    }
}

pub fn build(
    current: Rc<RefCell<Config>>,
    on_change: impl Fn(Config) -> Result<(), String> + 'static,
    on_reposition: impl Fn() -> Result<(), String> + 'static,
) -> Panel {
    let on_change: Rc<dyn Fn(Config) -> Result<(), String>> = Rc::new(on_change);
    let position = PositionControls::new(current.clone(), on_change.clone(), on_reposition);
    let root = gtk::Box::new(gtk::Orientation::Vertical, 20);
    root.add_css_class("appearance-panel");
    let error = gtk::Label::new(None);
    error.set_wrap(true);
    error.add_css_class("save-error");
    error.set_visible(false);
    root.append(&error);
    let mut samples = Vec::new();
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
        samples.push((sample, paper.clone()));
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
    let patterns = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    let pattern_section = gtk::Box::new(gtk::Orientation::Vertical, 8);
    section(&pattern_section, "Canvas pattern", &patterns);
    root.append(&pattern_section);
    let mut pattern_buttons = Vec::new();
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
        patterns.append(&b);
        if group.is_none() {
            group = Some(b.clone());
        }
        pattern_buttons.push((b, pattern));
    }
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    let label = gtk::Label::new(Some("Reduce motion"));
    label.set_xalign(0.);
    label.set_hexpand(true);
    let switch = gtk::Switch::new();
    switch.set_valign(gtk::Align::Center);
    switch.set_active(config.reduced_motion);
    row.append(&label);
    row.append(&switch);
    root.append(&row);
    let appearance = Rc::new(AppearanceControls {
        current: current.clone(),
        syncing: syncing.clone(),
        themes: theme_buttons,
        mode,
        modes: modes.1,
        fonts: fonts.1,
        patterns: pattern_buttons,
        motion: switch,
        samples,
    });
    let weak = Rc::downgrade(&appearance);
    let update: Rc<dyn Fn(Config)> = Rc::new(move |config| {
        match on_change(config) {
            Ok(()) => error.set_visible(false),
            Err(message) => {
                error.set_text(&message);
                error.set_visible(true);
            }
        }
        if let Some(controls) = weak.upgrade() {
            controls.sync();
        }
    });
    for (button, theme) in &appearance.themes {
        let theme = *theme;
        let weak = Rc::downgrade(&appearance);
        let update = update.clone();
        button.connect_toggled(move |button| {
            if let Some(controls) = weak.upgrade()
                && button.is_active()
                && !controls.syncing.get()
            {
                let mut config = controls.current.borrow().clone();
                config.theme = theme;
                update(config);
            }
        });
    }
    for (index, button) in appearance.modes.iter().enumerate() {
        let weak = Rc::downgrade(&appearance);
        let update = update.clone();
        button.connect_toggled(move |button| {
            if let Some(controls) = weak.upgrade()
                && button.is_active()
                && !controls.syncing.get()
            {
                let mut config = controls.current.borrow().clone();
                config.appearance =
                    [Appearance::System, Appearance::Light, Appearance::Dark][index];
                update(config);
            }
        });
    }
    for (index, button) in appearance.fonts.iter().enumerate() {
        let weak = Rc::downgrade(&appearance);
        let update = update.clone();
        button.connect_toggled(move |button| {
            if let Some(controls) = weak.upgrade()
                && button.is_active()
                && !controls.syncing.get()
            {
                let mut config = controls.current.borrow().clone();
                config.font = [Font::Handwritten, Font::Grotesk, Font::Mono][index];
                update(config);
            }
        });
    }
    for (button, pattern) in &appearance.patterns {
        let pattern = *pattern;
        let weak = Rc::downgrade(&appearance);
        let update = update.clone();
        button.connect_toggled(move |button| {
            if let Some(controls) = weak.upgrade()
                && button.is_active()
                && !controls.syncing.get()
            {
                let mut config = controls.current.borrow().clone();
                config.paper = pattern;
                update(config);
            }
        });
    }
    {
        let weak = Rc::downgrade(&appearance);
        let update = update.clone();
        reset.connect_clicked(move |_| {
            if let Some(controls) = weak.upgrade()
                && !controls.syncing.get()
            {
                let mut config = controls.current.borrow().clone();
                config.font = Font::Theme;
                update(config);
            }
        });
    }
    {
        let weak = Rc::downgrade(&appearance);
        let update = update.clone();
        appearance.motion.connect_active_notify(move |switch| {
            if let Some(controls) = weak.upgrade()
                && !controls.syncing.get()
                && controls.current.borrow().reduced_motion != switch.is_active()
            {
                let mut config = controls.current.borrow().clone();
                config.reduced_motion = switch.is_active();
                update(config);
            }
        });
    }
    root.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
    let heading = gtk::Label::new(Some("Edge strip"));
    heading.set_xalign(0.);
    heading.add_css_class("setting-label");
    root.append(&heading);
    root.append(&position.root);
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
        position,
        timeouts,
        syncing,
        appearance,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "requires a GTK display"]
    fn appearance_rejections_restore_controls_without_submitting_another_update() {
        adw::init().unwrap();
        let current = Rc::new(RefCell::new(Config {
            theme: Theme::Paper,
            font: Font::Theme,
            appearance: Appearance::Light,
            paper: Paper::Dots,
            ..Config::default()
        }));
        let attempts = Rc::new(Cell::new(0));
        let reject = Rc::new(Cell::new(true));
        let panel = build(
            current.clone(),
            {
                let current = current.clone();
                let attempts = attempts.clone();
                let reject = reject.clone();
                move |config| {
                    attempts.set(attempts.get() + 1);
                    if reject.get() {
                        Err("Rejected for regression coverage".into())
                    } else {
                        current.replace(config);
                        Ok(())
                    }
                }
            },
            || Ok(()),
        );
        let controls = &panel.appearance;
        controls.themes[0].0.set_active(true);
        assert_eq!(attempts.get(), 1);
        assert!(controls.themes[1].0.is_active());
        assert!(!controls.mode.is_visible());
        assert!(controls.fonts[0].is_active());

        controls.fonts[2].set_active(true);
        assert_eq!(attempts.get(), 2);
        controls.patterns[2].0.set_active(true);
        assert_eq!(attempts.get(), 3);
        controls.motion.set_active(true);
        assert_eq!(attempts.get(), 4);
        assert!(controls.fonts[0].is_active());
        assert!(controls.patterns[1].0.is_active());
        assert!(!controls.motion.is_active());

        current.borrow_mut().theme = Theme::Minimal;
        panel.refresh();
        controls.modes[2].set_active(true);
        assert_eq!(attempts.get(), 5);
        assert!(controls.mode.is_visible());
        assert!(controls.modes[1].is_active());
        assert!(controls.fonts[1].is_active());

        reject.set(false);
        controls.themes[2].0.set_active(true);
        assert_eq!(attempts.get(), 6);
        assert_eq!(current.borrow().theme, Theme::Playful);
        assert!(controls.themes[2].0.is_active());
        assert!(!controls.mode.is_visible());
    }
}
