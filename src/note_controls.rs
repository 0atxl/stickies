use crate::{
    app::NoteColor,
    appearance,
    config::{Font, NoteStyle, NoteTextSize, Paper, ScreenEdge, Theme},
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
    pub pages: gtk::Stack,
    normal: gtk::Box,
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
    pub colors: Vec<(gtk::ToggleButton, NoteColor)>,
    pub patterns: Vec<(gtk::ToggleButton, Paper)>,
    pub bullet: Option<gtk::Button>,
    pub numbered: Option<gtk::Button>,
    pub separate: Option<gtk::Button>,
    pub copy: gtk::Button,
    pub archive: Option<gtk::Button>,
    pub restore: Option<gtk::Button>,
    pub delete: gtk::Button,
    color_dot: gtk::Box,
    surface: Surface,
    pub fonts: Vec<(gtk::ToggleButton, Font)>,
}

fn popup(button: &gtk::MenuButton, child: &impl IsA<gtk::Widget>) -> gtk::Popover {
    let popup = gtk::Popover::new();
    popup.add_css_class("note-popover");
    popup.set_autohide(false);
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
        let pages = gtk::Stack::new();
        pages.set_transition_type(gtk::StackTransitionType::None);
        pages.set_hhomogeneous(true);
        pages.set_vhomogeneous(true);
        let normal = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        pages.add_named(&normal, Some("settings"));
        root.append(&pages);
        let color = menu("Color");
        let color_dot = {
            let dot = gtk::Box::new(gtk::Orientation::Vertical, 0);
            dot.add_css_class("current-color-dot");
            dot.set_size_request(12, 12);
            dot.set_halign(gtk::Align::Center);
            dot.set_valign(gtk::Align::Center);
            color.set_child(Some(&dot));
            dot
        };
        let text = {
            let button = menu("Font");
            button.set_label("Aa");
            button
        };
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
        for button in [&color, &text] {
            normal.append(button);
        }
        normal.append(&checklist);
        normal.append(&pin);
        if let Some(popout) = &popout {
            normal.append(popout);
        }
        normal.append(&more);

        let mut sizes = Vec::new();
        let mut fonts = Vec::new();
        let text_popover = {
            let typography = gtk::Box::new(gtk::Orientation::Vertical, 8);
            typography.add_css_class("note-controls");
            let size_row = gtk::Box::new(gtk::Orientation::Horizontal, 0);
            size_row.add_css_class("text-controls");
            size_row.set_homogeneous(true);
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
            let families = gtk::Box::new(gtk::Orientation::Horizontal, 4);
            families.add_css_class("font-choices");
            for (font, name) in [
                (Font::Handwritten, "Virgil"),
                (Font::Grotesk, "Inter"),
                (Font::Mono, "JetBrains Mono"),
                (Font::Kalam, "Kalam"),
            ] {
                let button = gtk::ToggleButton::with_label("Aa");
                button.set_tooltip_text(Some(name));
                button.update_property(&[gtk::accessible::Property::Label(name)]);
                let mut description =
                    gtk::pango::FontDescription::from_string(if font == Font::Handwritten {
                        "Virgil 3 YOFF"
                    } else {
                        name
                    });
                description.set_absolute_size(19. * gtk::pango::SCALE as f64);
                let attributes = gtk::pango::AttrList::new();
                attributes.insert(gtk::pango::AttrFontDesc::new(&description));
                if let Some(label) = button.child().and_downcast::<gtk::Label>() {
                    label.set_attributes(Some(&attributes));
                }
                if let Some((first, _)) = fonts.first() {
                    button.set_group(Some(first));
                }
                families.append(&button);
                fonts.push((button, font));
            }
            typography.prepend(&families);
            popup(&text, &typography)
        };
        let mut colors = Vec::new();
        let color_popover = {
            let grid = gtk::Grid::new();
            grid.add_css_class("note-controls");
            grid.set_row_spacing(7);
            grid.set_column_spacing(7);
            for (index, color) in NoteColor::ALL.into_iter().enumerate() {
                let button = gtk::ToggleButton::new();
                button.add_css_class("color-swatch");
                button.add_css_class(&appearance::color_class(color));
                button.set_tooltip_text(Some(color.label()));
                grid.attach(&button, index as i32 % 8, index as i32 / 8, 1, 1);
                colors.push((button, color));
            }
            popup(&color, &grid)
        };
        let commands = gtk::Box::new(gtk::Orientation::Vertical, 0);
        commands.add_css_class("note-more");
        let mut patterns: Vec<(gtk::ToggleButton, Paper)> = Vec::new();
        let mut bullet = None;
        let mut numbered = None;
        let mut separate = None;
        let row = gtk::Box::new(gtk::Orientation::Horizontal, 4);
        row.add_css_class("pattern-choices");
        for (paper, name) in [
            (Paper::Plain, "Plain"),
            (Paper::Dots, "Dots"),
            (Paper::Grid, "Grid"),
            (Paper::Ruled, "Ruled"),
        ] {
            let button = gtk::ToggleButton::new();
            button.set_tooltip_text(Some(name));
            button.update_property(&[gtk::accessible::Property::Label(name)]);
            button.set_child(Some(&pattern_sample(paper)));
            if let Some((first, _)) = patterns.first() {
                button.set_group(Some(first));
            }
            row.append(&button);
            patterns.push((button, paper));
        }
        commands.append(&row);
        if surface != Surface::Preview {
            let lists = gtk::Box::new(gtk::Orientation::Horizontal, 4);
            lists.add_css_class("list-choices");
            lists.set_homogeneous(true);
            let bullets = ui_assets::icon("bullets", "Bullet list");
            let numbers = ui_assets::icon("numbering", "Numbered list");
            bullets.remove_css_class("icon-control");
            numbers.remove_css_class("icon-control");
            lists.append(&bullets);
            lists.append(&numbers);
            commands.append(&lists);
            bullet = Some(bullets);
            numbered = Some(numbers);
        }
        let command = |icon, label, accessible| {
            if surface == Surface::Preview {
                let button = ui_assets::icon(icon, accessible);
                button.remove_css_class("icon-control");
                button
            } else {
                action(icon, label, accessible)
            }
        };
        let copy = command("copy", "Copy", "Copy text");
        let archive =
            (surface == Surface::Preview).then(|| command("archive", "Archive", "Archive note"));
        let restore =
            (surface == Surface::Preview).then(|| command("restore", "Restore", "Restore note"));
        let delete = command("delete", "Delete", "Delete note");
        delete.add_css_class("delete-action");
        let actions = if surface == Surface::Preview {
            let row = gtk::Box::new(gtk::Orientation::Horizontal, 4);
            row.add_css_class("list-choices");
            row.set_homogeneous(true);
            commands.append(&row);
            row
        } else {
            commands.clone()
        };
        actions.append(&copy);
        if let Some(button) = &archive {
            actions.append(button);
        }
        if let Some(button) = &restore {
            actions.append(button);
            button.set_visible(false);
        }
        if surface == Surface::Docked {
            let button = action("popout", "Open away", "Open away from edge");
            commands.append(&button);
            separate = Some(button);
        }
        if surface != Surface::Preview {
            commands.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
        }
        actions.append(&delete);
        let more_popover = popup(&more, &commands);
        let panels = [&color_popover, &text_popover, &more_popover];
        for panel in panels {
            let others: Vec<_> = panels.iter().map(|p| p.downgrade()).collect();
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
        for button in [&copy, &delete]
            .into_iter()
            .chain(archive.iter())
            .chain(restore.iter())
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
            pages,
            normal,
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
            surface,
            fonts,
        }
    }
    pub fn popovers(&self) -> Vec<gtk::Popover> {
        vec![
            self.color_popover.clone(),
            self.text_popover.clone(),
            self.more_popover.clone(),
        ]
    }
    pub fn position_more(&self, edge: ScreenEdge) {
        if self.surface == Surface::Preview {
            return;
        }
        let left = edge == ScreenEdge::Right || self.surface == Surface::Detached;
        if left {
            self.normal
                .reorder_child_after(&self.more, None::<&gtk::Widget>);
        } else {
            self.normal.reorder_child_after(&self.more, Some(&self.pin));
        }
        self.more_popover
            .set_position(if self.surface == Surface::Detached {
                gtk::PositionType::Left
            } else {
                match edge {
                    ScreenEdge::Right => gtk::PositionType::Left,
                    ScreenEdge::Left => gtk::PositionType::Right,
                    ScreenEdge::Top => gtk::PositionType::Bottom,
                    ScreenEdge::Bottom => gtk::PositionType::Top,
                }
            });
    }
    // Nonmodal popovers let another footer control handle its first click.
    pub fn bind_dismissal(&self, host: &impl IsA<gtk::Widget>) {
        let click = gtk::GestureClick::new();
        click.set_propagation_phase(gtk::PropagationPhase::Capture);
        let weak = host.as_ref().downgrade();
        let panels: Vec<_> = self.popovers().iter().map(|p| p.downgrade()).collect();
        click.connect_pressed(move |_, _, x, y| {
            let Some(host) = weak.upgrade() else {
                return;
            };
            if let Some(target) = host.pick(x, y, gtk::PickFlags::DEFAULT) {
                let mut at = Some(target);
                while let Some(widget) = at {
                    if widget.has_css_class("note-footer") || widget.is::<gtk::Popover>() {
                        return;
                    }
                    at = widget.parent();
                }
            }
            for panel in &panels {
                if let Some(panel) = panel.upgrade() {
                    panel.popdown();
                }
            }
        });
        host.as_ref().add_controller(click);
    }
    pub fn dismiss(&self) -> bool {
        let opened = self.popovers().iter().any(|p| p.is_visible());
        for popup in self.popovers() {
            popup.popdown();
        }
        opened
    }
    pub fn sync(&self, style: &NoteStyle, color: NoteColor, theme: Theme) {
        for (button, size) in &self.sizes {
            button.set_active(style.text_size == *size);
        }
        for (button, value) in &self.colors {
            let selected = *value == color;
            button.set_active(selected);
            button.set_label(if selected { "✓" } else { "" });
        }
        for (button, paper) in &self.patterns {
            button.set_active(style.paper.unwrap_or(Paper::Plain) == *paper);
        }
        let font = match style.font.unwrap_or(Font::Theme) {
            Font::Theme if theme == Theme::Minimal => Font::Grotesk,
            Font::Theme => Font::Handwritten,
            font => font,
        };
        for (button, value) in &self.fonts {
            button.set_active(font == *value);
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
            self.checklist.upcast_ref::<gtk::Widget>(),
            self.pin.upcast_ref(),
            self.delete.upcast_ref(),
        ] {
            widget.set_sensitive(editable);
        }
        if let Some(popout) = &self.popout {
            popout.set_sensitive(editable);
        }
        for button in [&self.color, &self.text] {
            button.set_sensitive(editable);
        }
        for (button, _) in &self.patterns {
            button.set_sensitive(editable);
        }
        if let Some(button) = &self.archive {
            button.set_visible(editable);
        }
        if let Some(button) = &self.restore {
            button.set_visible(!editable);
        }
    }
}

fn sync_checklist_button(button: &gtk::ToggleButton, body: &gtk::TextView) {
    let buffer = body.buffer();
    let mut start = buffer.iter_at_offset(buffer.cursor_position());
    start.set_line_offset(0);
    let mut end = start;
    end.forward_to_line_end();
    let line = crate::checklist::text(&buffer, &start, &end);
    button.set_active(line.starts_with("- [ ] ") || line.starts_with("- [x] "));
}

fn action(icon: &str, label: &str, accessible: &str) -> gtk::Button {
    let button = gtk::Button::new();
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    row.append(&ui_assets::image(icon));
    let text = gtk::Label::new(Some(label));
    text.set_xalign(0.);
    row.append(&text);
    button.set_child(Some(&row));
    button.update_property(&[gtk::accessible::Property::Label(accessible)]);
    button.set_tooltip_text(Some(accessible));
    button
}

fn pattern_sample(paper: Paper) -> gtk::DrawingArea {
    let sample = gtk::DrawingArea::new();
    sample.set_content_width(28);
    sample.set_content_height(22);
    sample.set_draw_func(move |area, cr, width, height| {
        let ink = area.color();
        cr.set_source_rgba(
            ink.red().into(),
            ink.green().into(),
            ink.blue().into(),
            0.55,
        );
        cr.set_line_width(1.);
        match paper {
            Paper::Plain => {
                cr.rectangle(2.5, 2.5, (width - 5) as f64, (height - 5) as f64);
            }
            Paper::Dots => {
                for x in (4..width).step_by(7) {
                    for y in (4..height).step_by(7) {
                        cr.new_sub_path();
                        cr.arc(x as f64, y as f64, 1., 0., std::f64::consts::TAU);
                    }
                }
                let _ = cr.fill();
            }
            Paper::Grid | Paper::Ruled => {
                for y in (4..height).step_by(7) {
                    cr.move_to(2., y as f64 + 0.5);
                    cr.line_to((width - 2) as f64, y as f64 + 0.5);
                }
                if paper == Paper::Grid {
                    for x in (4..width).step_by(7) {
                        cr.move_to(x as f64 + 0.5, 2.);
                        cr.line_to(x as f64 + 0.5, (height - 2) as f64);
                    }
                }
            }
        }
        let _ = cr.stroke();
    });
    sample
}
