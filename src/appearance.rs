use crate::{
    app::NoteColor,
    config::{Appearance, Config, Font, Paper, Shape, Theme},
};
use gtk::prelude::*;
use std::{cell::RefCell, rc::Rc};

pub fn icon(name: &str, label: &str) -> gtk::Button {
    let button = gtk::Button::from_icon_name(name);
    if name == "stickies-notes" {
        let drawing = gtk::DrawingArea::new();
        drawing.set_content_width(16);
        drawing.set_content_height(16);
        drawing.set_draw_func(|widget, cr, _, _| {
            let color = widget.style_context().color();
            cr.set_source_rgba(
                color.red().into(),
                color.green().into(),
                color.blue().into(),
                color.alpha().into(),
            );
            cr.set_line_width(1.3);
            cr.move_to(1., 10.);
            cr.line_to(1., 1.);
            cr.line_to(10., 1.);
            cr.move_to(4., 13.);
            cr.line_to(4., 4.);
            cr.line_to(13., 4.);
            cr.rectangle(7., 7., 8., 8.);
            let _ = cr.stroke();
        });
        button.set_child(Some(&drawing));
    }
    button.set_tooltip_text(Some(label));
    button.add_css_class("icon-control");
    button
}

pub fn toggle(name: &str, label: &str) -> gtk::ToggleButton {
    let button = gtk::ToggleButton::new();
    button.set_icon_name(name);
    button.set_tooltip_text(Some(label));
    button.add_css_class("icon-control");
    button
}

pub fn color_class(color: NoteColor) -> &'static str {
    match color {
        NoteColor::Yellow => "note-yellow",
        NoteColor::Blue => "note-blue",
        NoteColor::Purple => "note-purple",
        NoteColor::Green => "note-green",
    }
}

pub fn set_color(widget: &impl IsA<gtk::Widget>, color: NoteColor) {
    for class in ["note-yellow", "note-blue", "note-purple", "note-green"] {
        widget.remove_css_class(class);
    }
    widget.add_css_class(color_class(color));
}

pub fn dark(config: &Config) -> bool {
    match config.appearance {
        Appearance::Dark => true,
        Appearance::Light => false,
        Appearance::System => gtk::Settings::default().is_some_and(|s| {
            s.is_gtk_application_prefer_dark_theme()
                || s.gtk_theme_name()
                    .is_some_and(|n| n.to_lowercase().contains("dark"))
        }),
    }
}

pub fn apply_note_font(provider: &gtk::CssProvider, config: &Config) {
    let font = match config.font {
        Font::Theme if config.theme != Theme::Glass => "'Virgil 3 YOFF', cursive",
        Font::Theme | Font::Grotesk => "Inter, sans-serif",
        Font::Mono => "'JetBrains Mono', monospace",
        Font::Handwritten => "'Virgil 3 YOFF', cursive",
    };
    provider.load_from_data(&format!(
        ".editor-title, .editor-body, .editor-body text, .inline-title, .inline-body, .inline-body text {{ font-family: {font}; }}"
    ));
}

pub fn apply(provider: &gtk::CssProvider, config: &Config) {
    let is_dark = config.theme == Theme::Glass && dark(config);
    let (bg, fg, muted, line) = if is_dark {
        ("#22252c", "#f0f0ee", "#afb3ba", "#454953")
    } else {
        ("#faf8f2", "#302f2b", "#8a867d", "#e4dfd5")
    };
    let font = match config.font {
        Font::Theme if config.theme != Theme::Glass => "'Virgil 3 YOFF', cursive",
        Font::Theme | Font::Grotesk => "Inter, sans-serif",
        Font::Mono => "'JetBrains Mono', monospace",
        Font::Handwritten => "'Virgil 3 YOFF', cursive",
    };
    let tab_colors = if config.theme == Theme::Playful {
        ".note-tab.note-yellow { background: #ffe691; } .note-tab.note-blue { background: #b8d9f5; } .note-tab.note-purple { background: #e0c7f5; } .note-tab.note-green { background: #c4e6b8; }"
    } else {
        ""
    };
    let radius = if config.theme == Theme::Playful {
        14
    } else {
        0
    };
    provider.load_from_data(&format!(r#"
        .edge-window, .hit-area, .note-window {{ background: transparent; }}
        .icon-control.note-yellow {{ color: #c99932; }}
        .icon-control.note-blue {{ color: #639cd1; }}
        .icon-control.note-purple {{ color: #ac83bf; }}
        .icon-control.note-green {{ color: #73a681; }}
        .metadata, .dim-label {{ color: {muted}; font-size: 11px; }}
        .editor-card {{ padding: 24px 26px 30px; color: {fg}; }}
        .editor-title {{ background: transparent; color: inherit; border: none; outline: none; box-shadow: none; font-family: {font}; font-size: {}px; font-weight: 400; padding: 0; min-width: 0; }}
        .editor-body, .editor-body text {{ background: transparent; color: inherit; font-family: {font}; font-size: {}px; }}
        .editor-body {{ caret-color: currentColor; }}
        .editor-card button {{ color: inherit; }}
        .note-rail {{ background: {bg}; color: {fg}; border: 1px solid {line}; border-radius: 3px; padding: 4px 2px 8px; }}
        .note-rail button {{ color: {fg}; }}
        .note-rail .icon-control {{ min-width: 22px; min-height: 22px; padding: 3px; }}
        .note-rail menubutton.icon-control {{ padding: 0; }}
        .note-rail .icon-control > button {{ min-width: 22px; min-height: 22px; padding: 3px; }}
        .rail-grip {{ min-width: 3px; min-height: 24px; padding: 0; border: 1px solid {line}; border-radius: 3px; background: {bg}; box-shadow: none; }}
        .rail-grip:hover {{ border-color: {muted}; }}
        .icon-control {{ background: transparent; border: none; box-shadow: none; min-width: 24px; min-height: 24px; padding: 5px; border-radius: 7px; }}
        .icon-control:hover {{ background: alpha(currentColor, 0.09); }}
        .icon-control:checked {{ background: alpha(currentColor, 0.17); }}
        .icon-control > button {{ background: transparent; border: none; box-shadow: none; color: inherit; }}
        .note-rail popover label, .note-rail popover button {{ color: {fg}; }}
        .strip-control {{ background: {bg}; color: {fg}; border: 1px solid {line}; border-radius: 50%; padding: 5px; }}
        .note-tab {{ background: {bg}; color: {fg}; border: 1px solid {line}; border-left-width: 3px; border-radius: {radius}px; padding: 8px 12px; box-shadow: 0 3px 8px alpha(black, 0.12); }}
        .note-tab:hover {{ background: mix({bg}, {fg}, 0.08); }}
        .note-tab.note-yellow {{ border-left-color: #c99932; }}
        .note-tab.note-blue {{ border-left-color: #639cd1; }}
        .note-tab.note-purple {{ border-left-color: #ac83bf; }}
        .note-tab.note-green {{ border-left-color: #73a681; }}
        {tab_colors}
        .tab-title {{ font-family: {font}; font-size: 13px; }}
        .edge-pill {{ min-width: 5px; min-height: 16px; border-radius: 3px; margin: 3px 0; }}
        .edge-pill.note-yellow {{ background: #c99932; }}
        .edge-pill.note-blue {{ background: #639cd1; }}
        .edge-pill.note-purple {{ background: #ac83bf; }}
        .edge-pill.note-green {{ background: #73a681; }}
        popover contents {{ padding: 4px; }}
        popover button, popover modelbutton {{ min-height: 24px; padding: 4px 8px; font-size: 12px; }}
        scrollbar.vertical, scrollbar.vertical:hover {{ min-width: 6px; background: transparent; }}
        scrollbar.vertical slider, scrollbar.vertical:hover slider {{ min-width: 3px; min-height: 24px; margin: 0 1px; border: none; border-radius: 3px; }}
        scrollbar.horizontal, scrollbar.horizontal:hover {{ min-height: 6px; background: transparent; }}
        scrollbar.horizontal slider, scrollbar.horizontal:hover slider {{ min-height: 3px; min-width: 24px; margin: 1px 0; border: none; border-radius: 3px; }}
        .save-error {{ color: #b54343; font-size: 11px; }}
    "#, config.font_size + 6, config.font_size + 8));
}

/// Draw only the paper decoration; text and controls remain native GTK widgets.
pub fn paper_background(
    config: Rc<RefCell<Config>>,
    color: Rc<std::cell::Cell<NoteColor>>,
) -> gtk::DrawingArea {
    let area = gtk::DrawingArea::new();
    area.set_draw_func(move |_, cr, width, height| {
        let config = config.borrow();
        let w = f64::from(width - 16).max(1.);
        let h = f64::from(height - 16).max(1.);
        cr.translate(8., 8.);
        let shape = if config.theme == Theme::Glass {
            Shape::Square
        } else {
            config.shape
        };
        let radius = match config.theme {
            Theme::Glass => 0.,
            Theme::Paper => 10.,
            Theme::Playful => 9.,
        };
        for offset in (1..=5).rev() {
            let _ = cr.save();
            cr.translate(0., f64::from(offset));
            sheet_path(cr, w, h, shape, radius);
            cr.set_source_rgba(0.25, 0.22, 0.16, 0.018);
            let _ = cr.fill();
            let _ = cr.restore();
        }
        sheet_path(cr, w, h, shape, radius);
        let (r, g, b, a) = match config.theme {
            Theme::Glass if dark(&config) => (0.12, 0.14, 0.17, 0.94),
            Theme::Glass => (0.97, 0.98, 0.99, 0.93),
            Theme::Paper if config.white_paper => (1., 1., 1., 1.),
            Theme::Paper => (0.965, 0.937, 0.859, 1.),
            Theme::Playful => match color.get() {
                NoteColor::Yellow => (0.99, 0.91, 0.68, 1.),
                NoteColor::Blue => (0.76, 0.86, 0.97, 1.),
                NoteColor::Purple => (0.86, 0.78, 0.94, 1.),
                NoteColor::Green => (0.69, 0.87, 0.77, 1.),
            },
        };
        cr.set_source_rgba(r, g, b, a);
        let _ = cr.fill_preserve();
        cr.set_source_rgba(0.30, 0.27, 0.20, 0.18);
        cr.set_line_width(0.7);
        let _ = cr.stroke_preserve();
        cr.clip();
        if config.theme != Theme::Glass {
            cr.set_source_rgba(0.25, 0.27, 0.32, 0.07);
            cr.set_line_width(0.7);
            match config.paper {
                Paper::Plain => {}
                Paper::Dots => {
                    for x in (16..width).step_by(20) {
                        for y in (60..height).step_by(20) {
                            cr.arc(f64::from(x), f64::from(y), 0.7, 0., std::f64::consts::TAU);
                            let _ = cr.fill();
                        }
                    }
                }
                Paper::Grid | Paper::Ruled => {
                    for y in (60..height).step_by(24) {
                        cr.move_to(0., f64::from(y));
                        cr.line_to(w, f64::from(y));
                    }
                    if config.paper == Paper::Grid {
                        for x in (16..width).step_by(24) {
                            cr.move_to(f64::from(x), 48.);
                            cr.line_to(f64::from(x), h);
                        }
                    }
                    let _ = cr.stroke();
                }
            }
        }
        if config.theme != Theme::Paper {
            let (r, g, b) = match color.get() {
                NoteColor::Yellow => (0.79, 0.60, 0.20),
                NoteColor::Blue => (0.39, 0.61, 0.82),
                NoteColor::Purple => (0.67, 0.51, 0.75),
                NoteColor::Green => (0.45, 0.65, 0.51),
            };
            cr.set_source_rgb(r, g, b);
            cr.rectangle(0., 0., 3., h);
            let _ = cr.fill();
        }
    });
    area
}

fn sheet_path(cr: &gtk::cairo::Context, w: f64, h: f64, shape: Shape, radius: f64) {
    cr.new_path();
    cr.move_to(radius, 0.);
    cr.line_to(w - radius, 0.);
    cr.curve_to(w, 0., w, 0., w, radius);
    if shape == Shape::Square {
        cr.line_to(w, h - radius);
        cr.curve_to(w, h, w, h, w - radius, h);
        cr.line_to(radius, h);
        cr.curve_to(0., h, 0., h, 0., h - radius);
    } else {
        cr.line_to(w, h - 4.);
        let count = (w / 12.).ceil() as usize;
        for i in (0..count).rev() {
            let x = (i as f64 * 12.).min(w);
            let depth = if shape == Shape::Ticket {
                3.
            } else {
                2. + ((i * 7) % 5) as f64
            };
            cr.curve_to(x + 8., h - 4., x + 6., h - 4. + depth, x, h - 4.);
        }
    }
    cr.line_to(0., radius);
    cr.curve_to(0., 0., 0., 0., radius, 0.);
    cr.close_path();
}

pub fn animate_tab_on_hover(
    fixed: &gtk::Fixed,
    tab: &gtk::Button,
    x: f64,
    y: f64,
    edge: crate::config::ScreenEdge,
    overlapping: bool,
) {
    use crate::config::ScreenEdge;
    let dx = match edge {
        ScreenEdge::Right => -9.,
        ScreenEdge::Left => 9.,
        _ => 0.,
    };
    let dy = match edge {
        ScreenEdge::Top => 9.,
        ScreenEdge::Bottom => -9.,
        _ => 0.,
    };
    let state = Rc::new(std::cell::Cell::new((x, y, 0.)));
    let generation = Rc::new(std::cell::Cell::new(0_u64));
    let motion = gtk::EventControllerMotion::new();
    let weak_fixed = fixed.downgrade();
    let weak_tab = tab.downgrade();
    let position = state.clone();
    let serial = generation.clone();
    motion.connect_enter(move |_, _, _| {
        if let (Some(fixed), Some(tab)) = (weak_fixed.upgrade(), weak_tab.upgrade()) {
            animate_tab(
                &fixed,
                &tab,
                position.clone(),
                serial.clone(),
                (x + dx, y + dy, if overlapping { -3. } else { 0. }),
            );
        }
    });
    let weak_fixed = fixed.downgrade();
    let weak_tab = tab.downgrade();
    motion.connect_leave(move |_| {
        if let (Some(fixed), Some(tab)) = (weak_fixed.upgrade(), weak_tab.upgrade()) {
            animate_tab(&fixed, &tab, state.clone(), generation.clone(), (x, y, 0.));
        }
    });
    tab.add_controller(motion);
}

fn animate_tab(
    fixed: &gtk::Fixed,
    tab: &gtk::Button,
    position: Rc<std::cell::Cell<(f64, f64, f64)>>,
    serial: Rc<std::cell::Cell<u64>>,
    target: (f64, f64, f64),
) {
    let generation = serial.get().wrapping_add(1);
    serial.set(generation);
    let start = position.get();
    let started = std::time::Instant::now();
    let fixed = fixed.downgrade();
    tab.add_tick_callback(move |tab, _| {
        if serial.get() != generation {
            return gtk::glib::ControlFlow::Break;
        }
        let Some(fixed) = fixed.upgrade() else {
            return gtk::glib::ControlFlow::Break;
        };
        let elapsed = (started.elapsed().as_secs_f64() / 0.14).min(1.);
        let t = 1. - (1. - elapsed).powi(3);
        let next = (
            start.0 + (target.0 - start.0) * t,
            start.1 + (target.1 - start.1) * t,
            start.2 + (target.2 - start.2) * t,
        );
        position.set(next);
        fixed.set_child_transform(
            tab,
            Some(
                &gtk::gsk::Transform::new()
                    .translate(&gtk::graphene::Point::new(next.0 as f32, next.1 as f32))
                    .rotate(next.2 as f32),
            ),
        );
        if elapsed >= 1. {
            gtk::glib::ControlFlow::Break
        } else {
            gtk::glib::ControlFlow::Continue
        }
    });
}
