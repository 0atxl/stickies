use crate::{
    app::NoteColor,
    appearance,
    config::{NoteStyle, NoteTextSize, NoteTextWeight, Paper, Theme},
    ui_assets,
};
use gtk::prelude::*;

#[derive(Clone, Copy, PartialEq)]
pub enum Surface {
    Docked,
    Detached,
    Preview,
}

// Shared footer presentation; the owning editor wires the existing handlers.
pub struct NoteControls {
    pub root: gtk::Box,
    pub color: gtk::MenuButton,
    pub text: gtk::MenuButton,
    pub checklist: gtk::ToggleButton,
    pub pin: gtk::ToggleButton,
    pub popout: Option<gtk::Button>,
    pub more: gtk::MenuButton,
    pub color_popover: gtk::Popover,
    pub text_popover: gtk::Popover,
    pub more_popover: gtk::Popover,
    pub sizes: Vec<(gtk::ToggleButton, NoteTextSize)>,
    pub weights: Vec<(gtk::ToggleButton, NoteTextWeight)>,
    pub colors: Vec<(gtk::ToggleButton, NoteColor)>,
    pub patterns: Vec<(gtk::ToggleButton, Option<Paper>)>,
    pub bullet: Option<gtk::Button>,
    pub numbered: Option<gtk::Button>,
    pub separate: Option<gtk::Button>,
    pub copy: gtk::Button,
    pub archive: gtk::Button,
    pub restore: gtk::Button,
    pub delete: gtk::Button,
    color_dot: gtk::Box,
}

fn popup(button: &gtk::MenuButton, child: &impl IsA<gtk::Widget>) -> gtk::Popover {
    let popup = gtk::Popover::new();
    popup.add_css_class("note-popover");
    popup.set_position(gtk::PositionType::Top);
    popup.set_child(Some(child));
    button.set_popover(Some(&popup));
    popup
}

fn menu(label: &str) -> gtk::MenuButton {
    let button = gtk::MenuButton::new();
    button.add_css_class("footer-button");
    button.set_direction(gtk::ArrowType::None);
    button.set_tooltip_text(Some(label));
    button.update_property(&[gtk::accessible::Property::Label(label)]);
    button
}

impl NoteControls {
    pub fn new(surface: Surface) -> Self {
        let root = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        root.add_css_class("note-footer");
        root.set_widget_name("note-footer");
        root.set_height_request(44);
        root.set_halign(gtk::Align::Center);
        let color = menu("Color");
        let color_dot = gtk::Box::new(gtk::Orientation::Vertical, 0);
        color_dot.add_css_class("current-color-dot");
        color_dot.set_size_request(12, 12);
        color_dot.set_halign(gtk::Align::Center);
        color_dot.set_valign(gtk::Align::Center);
        color.set_child(Some(&color_dot));
        let text = menu("Text");
        text.set_label("Aa");
        let checklist = appearance::toggle("checkbox-checked-symbolic", "Checklist");
        checklist.set_child(Some(&ui_assets::image("checklist")));
        checklist.set_focus_on_click(false);
        let pin = appearance::toggle(
            "view-pin-symbolic",
            if surface == Surface::Preview {
                "Pin note"
            } else {
                "Always open"
            },
        );
        pin.set_child(Some(&ui_assets::image("pin")));
        let more = menu("More");
        more.set_child(Some(&ui_assets::image("more")));
        let popout = (surface == Surface::Preview).then(|| ui_assets::icon("popout", "Pop out"));
        for widget in [
            color.upcast_ref::<gtk::Widget>(),
            text.upcast_ref(),
            checklist.upcast_ref(),
            pin.upcast_ref(),
        ] {
            root.append(widget);
        }
        if let Some(popout) = &popout {
            root.append(popout);
        }
        root.append(&more);

        let typography = gtk::Box::new(gtk::Orientation::Vertical, 8);
        typography.add_css_class("note-controls");
        let size_row = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        size_row.add_css_class("text-controls");
        size_row.set_homogeneous(true);
        let mut sizes: Vec<(gtk::ToggleButton, NoteTextSize)> = Vec::new();
        for (size, label, tooltip) in [
            (NoteTextSize::Small, "S", "Small text"),
            (NoteTextSize::Medium, "M", "Medium text"),
            (NoteTextSize::Large, "L", "Large text"),
        ] {
            let button = gtk::ToggleButton::with_label(label);
            button.set_tooltip_text(Some(tooltip));
            if let Some((first, _)) = sizes.first() {
                button.set_group(Some(first));
            }
            size_row.append(&button);
            sizes.push((button, size));
        }
        typography.append(&size_row);
        let weight_row = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        weight_row.add_css_class("text-controls");
        weight_row.set_homogeneous(true);
        let mut weights: Vec<(gtk::ToggleButton, NoteTextWeight)> = Vec::new();
        for (weight, label) in [
            (NoteTextWeight::Regular, "Regular"),
            (NoteTextWeight::Bold, "Bold"),
        ] {
            let button = gtk::ToggleButton::with_label(label);
            if let Some((first, _)) = weights.first() {
                button.set_group(Some(first));
            }
            weight_row.append(&button);
            weights.push((button, weight));
        }
        typography.append(&weight_row);
        let text_popover = popup(&text, &typography);

        let grid = gtk::Grid::new();
        grid.add_css_class("note-controls");
        grid.set_row_spacing(7);
        grid.set_column_spacing(7);
        let mut colors = Vec::new();
        for (index, color) in NoteColor::ALL.into_iter().enumerate() {
            let button = gtk::ToggleButton::new();
            button.add_css_class("color-swatch");
            button.add_css_class(&appearance::color_class(color));
            button.set_tooltip_text(Some(color.label()));
            grid.attach(&button, index as i32 % 4, index as i32 / 4, 1, 1);
            colors.push((button, color));
        }
        let color_popover = popup(&color, &grid);
        let commands = gtk::Box::new(gtk::Orientation::Vertical, 4);
        commands.add_css_class("note-more");
        let mut patterns: Vec<(gtk::ToggleButton, Option<Paper>)> = Vec::new();
        let mut bullet = None;
        let mut numbered = None;
        let mut separate = None;
        if surface != Surface::Preview {
            let bullets = gtk::Button::with_label("Bullet list");
            let numbers = gtk::Button::with_label("Numbered list");
            commands.append(&bullets);
            commands.append(&numbers);
            bullet = Some(bullets);
            numbered = Some(numbers);
            let label = gtk::Label::new(Some("Pattern"));
            label.set_xalign(0.);
            commands.append(&label);
            let row = gtk::Box::new(gtk::Orientation::Horizontal, 2);
            row.add_css_class("pattern-choices");
            for (paper, label) in [
                (None, "Default"),
                (Some(Paper::Plain), "Plain"),
                (Some(Paper::Dots), "Dots"),
                (Some(Paper::Grid), "Grid"),
                (Some(Paper::Ruled), "Ruled"),
            ] {
                let button = gtk::ToggleButton::with_label(label);
                if let Some((first, _)) = patterns.first() {
                    button.set_group(Some(first));
                }
                row.append(&button);
                patterns.push((button, paper));
            }
            commands.append(&row);
        }
        let copy = gtk::Button::with_label("Copy text");
        let archive = gtk::Button::with_label("Archive");
        let restore = gtk::Button::with_label("Restore");
        let delete = gtk::Button::with_label("Delete");
        for button in [&copy, &archive] {
            commands.append(button);
        }
        if surface == Surface::Preview {
            commands.append(&restore);
        }
        commands.append(&delete);
        restore.set_visible(false);
        if surface == Surface::Docked {
            let button = gtk::Button::with_label("Open away from edge");
            commands.append(&button);
            separate = Some(button);
        }
        let more_popover = popup(&more, &commands);
        for panel in [&color_popover, &text_popover, &more_popover] {
            let others = [
                color_popover.downgrade(),
                text_popover.downgrade(),
                more_popover.downgrade(),
            ];
            panel.connect_visible_notify(move |panel| {
                if panel.is_visible() {
                    for other in &others {
                        if let Some(other) = other.upgrade()
                            && &other != panel
                        {
                            other.popdown();
                        }
                    }
                }
            });
        }
        for button in [&copy, &archive, &restore, &delete]
            .into_iter()
            .chain(bullet.iter())
            .chain(numbered.iter())
            .chain(separate.iter())
        {
            let weak = more_popover.downgrade();
            button.connect_clicked(move |_| {
                if let Some(menu) = weak.upgrade() {
                    menu.popdown();
                }
            });
        }
        Self {
            root,
            color,
            text,
            checklist,
            pin,
            popout,
            more,
            color_popover,
            text_popover,
            more_popover,
            sizes,
            weights,
            colors,
            patterns,
            bullet,
            numbered,
            separate,
            copy,
            archive,
            restore,
            delete,
            color_dot,
        }
    }
    pub fn popovers(&self) -> [gtk::Popover; 3] {
        [
            self.color_popover.clone(),
            self.text_popover.clone(),
            self.more_popover.clone(),
        ]
    }
    pub fn dismiss(&self) -> bool {
        let opened = self.popovers().iter().any(|p| p.is_visible());
        for popup in self.popovers() {
            popup.popdown();
        }
        opened
    }
    pub fn sync(&self, style: &NoteStyle, color: NoteColor, _theme: Theme) {
        for (button, size) in &self.sizes {
            button.set_active(style.text_size == *size);
        }
        for (button, weight) in &self.weights {
            button.set_active(style.text_weight == *weight);
        }
        for (button, value) in &self.colors {
            let selected = *value == color;
            button.set_active(selected);
            button.set_label(if selected { "✓" } else { "" });
        }
        for (button, paper) in &self.patterns {
            button.set_active(style.paper == *paper);
        }
        appearance::set_color(&self.color_dot, color);
    }
    pub fn sync_checklist(&self, body: &gtk::TextView) {
        sync_checklist_button(&self.checklist, body);
    }
    pub fn bind_checklist(&self, body: &gtk::TextView) {
        use std::{cell::RefCell, rc::Rc};
        let previous = Rc::new(RefCell::new(
            None::<(gtk::TextBuffer, gtk::glib::SignalHandlerId)>,
        ));
        let button = self.checklist.downgrade();
        let bind = move |body: &gtk::TextView| {
            if let Some((buffer, id)) = previous.borrow_mut().take() {
                buffer.disconnect(id);
            }
            let weak = body.downgrade();
            let button = button.clone();
            let buffer = body.buffer();
            let id = buffer.connect_mark_set(move |_, _, _| {
                if let (Some(body), Some(button)) = (weak.upgrade(), button.upgrade()) {
                    sync_checklist_button(&button, &body);
                }
            });
            previous.replace(Some((buffer, id)));
        };
        bind(body);
        body.connect_buffer_notify(move |body| bind(body));
        self.sync_checklist(body);
    }
    pub fn set_editable(&self, editable: bool) {
        for widget in [
            self.color.upcast_ref::<gtk::Widget>(),
            self.text.upcast_ref(),
            self.checklist.upcast_ref(),
            self.pin.upcast_ref(),
            self.delete.upcast_ref(),
        ] {
            widget.set_sensitive(editable);
        }
        if let Some(popout) = &self.popout {
            popout.set_sensitive(editable);
        }
        self.archive.set_visible(editable);
        self.restore.set_visible(!editable);
    }
}

fn sync_checklist_button(button: &gtk::ToggleButton, body: &gtk::TextView) {
    let buffer = body.buffer();
    let mut start = buffer.iter_at_offset(buffer.cursor_position());
    start.set_line_offset(0);
    let mut end = start;
    end.forward_to_line_end();
    let line = buffer.text(&start, &end, true);
    button.set_active(line.starts_with("- [ ] ") || line.starts_with("- [x] "));
}
