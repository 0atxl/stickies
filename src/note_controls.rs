use crate::{
    app::NoteColor,
    appearance,
    config::{NoteStyle, NoteTextSize, NoteTextWeight, Theme},
};
use gtk::prelude::*;

// Both editors use these widgets; each view wires them to its shared session.
pub struct NoteControls {
    pub root: gtk::Box,
    pub sizes: Vec<(gtk::ToggleButton, NoteTextSize)>,
    pub weights: Vec<(gtk::ToggleButton, NoteTextWeight)>,
    pub colors: Vec<(gtk::ToggleButton, NoteColor)>,
    color_label: gtk::Label,
}

impl NoteControls {
    pub fn new() -> Self {
        let root = gtk::Box::new(gtk::Orientation::Vertical, 8);
        root.add_css_class("note-controls");
        root.set_width_request(120);
        let typography = gtk::Box::new(gtk::Orientation::Vertical, 8);
        let palette = gtk::Box::new(gtk::Orientation::Vertical, 8);
        root.append(&typography);
        root.append(&palette);
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
            button.set_tooltip_text(Some(&format!("{label} text")));
            if let Some((first, _)) = weights.first() {
                button.set_group(Some(first));
            }
            weight_row.append(&button);
            weights.push((button, weight));
        }
        typography.append(&weight_row);
        let color_label = gtk::Label::new(None);
        color_label.set_xalign(0.);
        palette.append(&color_label);
        let grid = gtk::Grid::new();
        grid.set_row_spacing(7);
        grid.set_column_spacing(7);
        grid.set_halign(gtk::Align::Center);
        let mut colors = Vec::new();
        for (index, color) in NoteColor::ALL.into_iter().enumerate() {
            let button = gtk::ToggleButton::new();
            button.add_css_class("color-swatch");
            button.add_css_class(&appearance::color_class(color));
            button.set_tooltip_text(Some(color.label()));
            grid.attach(&button, index as i32 % 4, index as i32 / 4, 1, 1);
            colors.push((button, color));
        }
        palette.append(&grid);
        Self {
            root,
            sizes,
            weights,
            colors,
            color_label,
        }
    }

    pub fn sync(&self, style: &NoteStyle, color: NoteColor, theme: Theme) {
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
        self.color_label.set_text(match theme {
            Theme::Paper => "Side accent",
            Theme::Minimal => "Top accent",
            Theme::Playful => "Note color",
        });
    }
}
