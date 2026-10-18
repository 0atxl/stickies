use crate::{
    app::NoteColor,
    appearance,
    config::{Appearance, Config, MarkerStyle, ScreenEdge, Shape, TabLayout, Theme},
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
    extras: Vec<Box<dyn Fn()>>,
}
impl Panel {
    pub fn refresh(&self) {
        self.appearance.sync();
        self.position.sync();
        for sync in &self.extras {
            sync();
        }
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
        self.motion.set_active(config.reduced_motion);
        for (sample, drawing) in &self.samples {
            let mut sample = sample.borrow_mut();
            sample.appearance = config.appearance;
            drawing.queue_draw();
        }
        self.syncing.set(was_syncing);
    }
}

struct PositionControls {
    root: gtk::Box,
    edges: Vec<gtk::ToggleButton>,
    presets: Vec<gtk::ToggleButton>,
    counts: Vec<gtk::ToggleButton>,
    layouts: Vec<gtk::ToggleButton>,
    markers: Vec<gtk::ToggleButton>,
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
        for (button, count) in self.counts.iter().zip([4, 5]) {
            button.set_active(config.visible_notes == count);
        }
        for (button, layout) in self
            .layouts
            .iter()
            .zip([TabLayout::Classic, TabLayout::Stacked])
        {
            button.set_active(config.tab_layout == layout);
        }
        for (button, marker) in self
            .markers
            .iter()
            .zip([MarkerStyle::Dots, MarkerStyle::Bar])
        {
            button.set_active(config.marker_style == marker);
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
        let root = gtk::Box::new(gtk::Orientation::Vertical, 16);
        let edges = choices(&["Left", "Right", "Top", "Bottom"], 0);
        let presets = choices(&["Top", "Center", "Bottom"], 1);
        section(&root, "Edge", &edges.0);
        let position = gtk::Box::new(gtk::Orientation::Vertical, 10);
        position.append(&presets.0);
        let move_button = gtk::Button::with_label("Reposition");
        move_button.set_halign(gtk::Align::Start);
        move_button.add_css_class("reposition-button");
        position.append(&move_button);
        section(&root, "Position", &position);
        let counts = choices(&["4", "5"], 0);
        section(&root, "Visible notes", &counts.0);
        let layouts = choices(&["Classic", "Stacked"], 1);
        section(&root, "Tabs", &layouts.0);
        let markers = choices(&["Pills", "Bar"], 1);
        section(&root, "Marker", &markers.0);
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
            counts: counts.1,
            layouts: layouts.1,
            markers: markers.1,
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
        for (index, button) in controls.counts.iter().enumerate() {
            let weak = Rc::downgrade(&controls);
            let change = change.clone();
            button.connect_toggled(move |button| {
                if let Some(controls) = weak.upgrade()
                    && !controls.syncing.get()
                    && button.is_active()
                {
                    let mut config = controls.current.borrow().clone();
                    config.visible_notes = [4, 5][index];
                    controls.result(change(config));
                }
            });
        }
        for (buttons, write) in [
            (
                &controls.layouts,
                (|c: &mut Config, index: usize| {
                    c.tab_layout = [TabLayout::Classic, TabLayout::Stacked][index]
                }) as fn(&mut Config, usize),
            ),
            (
                &controls.markers,
                (|c: &mut Config, index: usize| {
                    c.marker_style = [MarkerStyle::Dots, MarkerStyle::Bar][index]
                }) as fn(&mut Config, usize),
            ),
        ] {
            for (index, button) in buttons.iter().enumerate() {
                let weak = Rc::downgrade(&controls);
                let change = change.clone();
                button.connect_toggled(move |button| {
                    if let Some(controls) = weak.upgrade()
                        && !controls.syncing.get()
                        && button.is_active()
                    {
                        let mut config = controls.current.borrow().clone();
                        write(&mut config, index);
                        controls.result(change(config));
                    }
                });
            }
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
    let root = gtk::Box::new(gtk::Orientation::Vertical, 18);
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
    let themes = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    root.append(&themes);
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
        paper.set_size_request(
            crate::edge_surface::scale_px(50),
            crate::edge_surface::scale_px(48),
        );
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
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    let label = gtk::Label::new(Some("Reduce motion"));
    label.set_xalign(0.);
    label.set_hexpand(true);
    let switch = gtk::Switch::new();
    switch.set_valign(gtk::Align::Center);
    switch.set_active(config.reduced_motion);
    row.append(&label);
    row.append(&switch);
    let motion_row = row;
    let appearance = Rc::new(AppearanceControls {
        current: current.clone(),
        syncing: syncing.clone(),
        themes: theme_buttons,
        mode,
        modes: modes.1,
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
    root.append(&position.root);
    root.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
    let selected = [0, 60, 120, 300]
        .iter()
        .position(|seconds| *seconds == config.inactivity_seconds)
        .unwrap_or(2);
    let timeouts = choices(&["Off", "1 min", "2 min", "5 min"], selected);
    section(&root, "Auto-close", &timeouts.0);
    timeouts
        .0
        .set_tooltip_text(Some("Close docked notes after inactivity"));
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
    root.append(&motion_row);
    let mut extras: Vec<Box<dyn Fn()>> = Vec::new();
    for (label, read, write) in [
        (
            "Hover to reveal",
            (|c: &Config| c.edge_hover_enabled) as fn(&Config) -> bool,
            (|c: &mut Config, v| c.edge_hover_enabled = v) as fn(&mut Config, bool),
        ),
        (
            "Launch at login",
            |c: &Config| c.launch_at_login,
            |c: &mut Config, v| c.launch_at_login = v,
        ),
    ] {
        let row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        let text = gtk::Label::new(Some(label));
        text.set_hexpand(true);
        text.set_xalign(0.);
        let switch = gtk::Switch::new();
        switch.set_valign(gtk::Align::Center);
        switch.update_property(&[gtk::accessible::Property::Label(label)]);
        switch.set_active(read(&current.borrow()));
        row.append(&text);
        row.append(&switch);
        root.append(&row);
        let current = current.clone();
        let update = update.clone();
        let guard = Rc::new(Cell::new(false));
        let sync: Rc<dyn Fn()> = Rc::new({
            let current = current.clone();
            let weak = switch.downgrade();
            let guard = guard.clone();
            move || {
                guard.set(true);
                if let Some(switch) = weak.upgrade() {
                    switch.set_active(read(&current.borrow()));
                }
                guard.set(false);
            }
        });
        let sync_after = sync.clone();
        switch.connect_active_notify(move |switch| {
            if guard.get() || read(&current.borrow()) == switch.is_active() {
                return;
            }
            let mut config = current.borrow().clone();
            write(&mut config, switch.is_active());
            update(config);
            sync_after();
        });
        extras.push(Box::new(move || sync()));
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
        extras,
    }
}

fn section(root: &gtk::Box, name: &str, control: &impl IsA<gtk::Widget>) {
    // Give the four-way choices and position presets the drawer's full width.
    let stacked = matches!(name, "Edge" | "Position" | "Auto-close");
    let box_ = gtk::Box::new(
        if stacked {
            gtk::Orientation::Vertical
        } else {
            gtk::Orientation::Horizontal
        },
        if stacked { 8 } else { 14 },
    );
    box_.add_css_class("setting-row");
    let label = gtk::Label::new(Some(name));
    label.set_xalign(0.);
    label.add_css_class("setting-label");
    label.set_valign(gtk::Align::Center);
    control.as_ref().set_hexpand(true);
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
            appearance: Appearance::Light,
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
        controls.motion.set_active(true);
        assert_eq!(attempts.get(), 2);
        assert!(!controls.motion.is_active());

        current.borrow_mut().theme = Theme::Minimal;
        panel.refresh();
        controls.modes[2].set_active(true);
        assert_eq!(attempts.get(), 3);
        assert!(controls.mode.is_visible());
        assert!(controls.modes[1].is_active());

        reject.set(false);
        controls.themes[2].0.set_active(true);
        assert_eq!(attempts.get(), 4);
        assert_eq!(current.borrow().theme, Theme::Playful);
        assert!(controls.themes[2].0.is_active());
        assert!(!controls.mode.is_visible());
        reject.set(true);
        fn descendants(widget: &gtk::Widget, result: &mut Vec<gtk::Widget>) {
            result.push(widget.clone());
            let mut child = widget.first_child();
            while let Some(widget) = child {
                descendants(&widget, result);
                child = widget.next_sibling();
            }
        }
        let mut widgets = Vec::new();
        descendants(panel.root.upcast_ref(), &mut widgets);
        for widget in widgets {
            if let Some(switch) = widget.downcast_ref::<gtk::Switch>() {
                let before = switch.is_active();
                let count = attempts.get();
                switch.set_active(!before);
                assert_eq!(switch.is_active(), before, "rejected switch must revert");
                assert_eq!(
                    attempts.get(),
                    count + 1,
                    "reverting must not submit another write"
                );
            }
        }
    }
}
