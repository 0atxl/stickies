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
        Font::Theme if config.theme != Theme::Minimal => "'Virgil 3 YOFF', cursive",
        Font::Theme | Font::Grotesk => "Inter, sans-serif",
        Font::Mono => "'JetBrains Mono', monospace",
        Font::Handwritten => "'Virgil 3 YOFF', cursive",
    };
    provider.load_from_data(&format!(
        ".editor-title, .editor-body, .editor-body text, .inline-title, .inline-body, .inline-body text {{ font-family: {font}; }}"
    ));
}

pub fn apply(provider: &gtk::CssProvider, config: &Config) {
    let is_dark = config.theme == Theme::Minimal && dark(config);
    let (bg, fg, muted, line) = if is_dark {
        ("#22252c", "#f0f0ee", "#afb3ba", "#454953")
    } else {
        ("#f8f7f3", "#2d2c29", "#7d7971", "#e3e0d8")
    };
    let font = match config.font {
        Font::Theme if config.theme != Theme::Minimal => "'Virgil 3 YOFF', cursive",
        Font::Theme | Font::Grotesk => "Inter, sans-serif",
        Font::Mono => "'JetBrains Mono', monospace",
        Font::Handwritten => "'Virgil 3 YOFF', cursive",
    };
    let tab_colors = if config.theme == Theme::Playful {
        ".note-tab.note-yellow { background: #faf0bd; } .note-tab.note-blue { background: #b8d9f5; } .note-tab.note-purple { background: #e0c7f5; } .note-tab.note-green { background: #c5ebdc; }"
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
        .editor-title {{ background: transparent; color: inherit; border: none; outline: none; box-shadow: none; font-family: {font}; font-weight: 400; padding: 0; min-width: 0; }}
        .editor-body, .editor-body text {{ background: transparent; color: inherit; font-family: {font}; }}
        .editor-body {{ caret-color: currentColor; }}
        .editor-card button {{ color: inherit; }}
        .note-rail .icon-control {{ min-width: 22px; min-height: 22px; padding: 3px; }}
        .note-rail menubutton.icon-control {{ padding: 0; }}
        .note-rail .icon-control > button {{ min-width: 22px; min-height: 22px; padding: 3px; }}
        .icon-control {{ background: transparent; border: none; box-shadow: none; min-width: 24px; min-height: 24px; padding: 5px; border-radius: 7px; }}
        .icon-control:hover {{ background: alpha(currentColor, 0.09); }}
        .icon-control:checked {{ background: alpha(currentColor, 0.17); }}
        .icon-control > button {{ background: transparent; border: none; box-shadow: none; color: inherit; }}
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
        .individual {{ --note-ink: #2d2c29; --rail-bg: #fbf8f0; --rail-line: #e3e0d8; --rail-selected: #ece8df; }}
        .individual.minimal {{ --note-ink: #292f36; --rail-bg: #f1f2f4; --rail-line: #cdd4db; --rail-selected: #d9e0e6; }}
        .individual.minimal-dark {{ --note-ink: #edf1f5; --rail-bg: #23262b; --rail-line: #424a55; --rail-selected: #37414d; }}
        .individual.playful {{ --note-ink: #292a2d; --rail-bg: #faf9fc; --rail-line: #e3e0d8; --rail-selected: #f7e5ed; }}
        .individual .editor-card {{ color: var(--note-ink); padding: 26px 20px 28px 32px; }}
        .individual .editor-title {{ font-size: 23px; }}
        .individual.minimal .editor-title {{ font-size: 20px; font-weight: 600; }}
        .individual .editor-body, .individual .editor-body text {{ font-size: 21px; }}
        .individual .note-rail {{ background: var(--rail-bg); color: var(--note-ink); border: 1px solid var(--rail-line); border-radius: 6px; padding: 4px 2px; box-shadow: 0 3px 6px alpha(black, 0.12); }}
        .individual .note-rail button, .individual .note-rail popover button, .individual .note-rail popover label {{ color: var(--note-ink); }}
        .individual .note-rail .icon-control:checked {{ background: var(--rail-selected); }}
        .individual.playful .note-rail .icon-control:checked {{ background: #f5a4c7; color: #292a2d; }}
        .individual .note-rail popover contents {{ background: var(--rail-bg); color: var(--note-ink); border: 1px solid var(--rail-line); padding: 8px; }}
        .individual .note-rail .color-swatch {{ min-width: 26px; min-height: 26px; padding: 0; border-radius: 50%; color: #23262b; border: 2px solid transparent; box-shadow: none; }}
        .individual .note-rail .color-swatch:checked {{ border-color: var(--note-ink); }}
        .individual .note-rail .color-swatch.note-yellow {{ background: #d98c2b; }}
        .individual .note-rail .color-swatch.note-blue {{ background: #5b8def; }}
        .individual .note-rail .color-swatch.note-purple {{ background: #a98fd1; }}
        .individual .note-rail .color-swatch.note-green {{ background: #6fae68; }}
        .individual .note-rail dropdown button {{ background: var(--rail-selected); color: var(--note-ink); border: 1px solid var(--rail-line); box-shadow: none; font-size: 12px; min-height: 24px; padding: 4px 8px; }}
        .individual .rail-grip {{ padding: 0; box-shadow: none; background: transparent; border: none; min-width: 24px; min-height: 32px; }}
        .individual .rail-grip-mark {{ background: #a5a7a7; min-width: 3px; min-height: 24px; border-radius: 2px; }}
        .individual .rail-grip:hover .rail-grip-mark {{ background: var(--note-ink); }}
        .individual.minimal .note-rail, .individual.minimal .icon-control {{ border-radius: 0; }}
        .inline-check {{ margin-right: 8px; color: inherit; }}
        .inline-check check {{ color: inherit; min-width: 16px; min-height: 16px; border-radius: 3px; border: 1px solid currentColor; background: transparent; box-shadow: none; }}
        .inline-check check:checked {{ background: #30353b; border-color: #30353b; color: white; }}
        .individual.minimal-dark .inline-check check:checked, .management.minimal-dark .inline-check check:checked {{ background: #edf1f5; border-color: #edf1f5; color: #23262b; }}
        .individual .note-save {{ color: var(--note-ink); font-family: Inter; font-size: 16px; min-width: 18px; }}
        @keyframes note-saving {{ 0%, 100% {{ opacity: .25; }} 50% {{ opacity: 1; }} }}
        .individual .saving-motion .note-save-dot {{ animation: note-saving 900ms ease-in-out infinite; }}
        .individual .saving-motion .dot-1 {{ animation-delay: 150ms; }}
        .individual .saving-motion .dot-2 {{ animation-delay: 300ms; }}

    "#));
}

/// Draw only the paper decoration; text and controls remain native GTK widgets.
pub fn paper_background(
    config: Rc<RefCell<Config>>,
    color: Rc<std::cell::Cell<NoteColor>>,
    seed: Rc<std::cell::Cell<u64>>,
) -> gtk::DrawingArea {
    let area = gtk::DrawingArea::new();
    area.set_draw_func(move |area, cr, width, height| {
        let config = config.borrow();
        let individual = area.has_css_class("individual-sheet");
        let w = f64::from(width - 16).max(1.);
        let h = f64::from(height - 16).max(1.);
        cr.translate(8., 8.);
        let shape = if config.theme == Theme::Minimal {
            Shape::Square
        } else {
            config.shape
        };
        let radius = match config.theme {
            Theme::Minimal => 0.,
            Theme::Paper if individual => 24.,
            Theme::Paper => 10.,
            Theme::Playful if individual => 24.,
            Theme::Playful => 16.,
        };
        let radius = (
            radius,
            if config.theme == Theme::Paper && individual {
                0.
            } else {
                radius
            },
        );
        for offset in (1..=if config.theme == Theme::Playful && !individual {
            0
        } else {
            5
        })
            .rev()
        {
            let _ = cr.save();
            cr.translate(0., f64::from(offset));
            sheet_path(
                cr,
                w,
                h,
                shape,
                radius,
                config.theme == Theme::Playful,
                seed.get(),
            );
            cr.set_source_rgba(
                0.16,
                0.18,
                0.22,
                if config.theme == Theme::Paper {
                    0.032
                } else {
                    0.018
                },
            );
            let _ = cr.fill();
            let _ = cr.restore();
        }
        if config.theme == Theme::Playful && !individual {
            let _ = cr.save();
            cr.translate(3., 4.);
            sheet_path(
                cr,
                w,
                h,
                shape,
                radius,
                config.theme == Theme::Playful,
                seed.get(),
            );
            cr.set_source_rgba(0.16, 0.15, 0.14, 0.85);
            let _ = cr.fill();
            let _ = cr.restore();
        }
        sheet_path(
            cr,
            w,
            h,
            shape,
            radius,
            config.theme == Theme::Playful,
            seed.get(),
        );
        let (r, g, b, a) = match config.theme {
            Theme::Minimal if individual && dark(&config) => {
                (38. / 255., 42. / 255., 45. / 255., 1.)
            }
            Theme::Minimal if individual => (238. / 255., 240. / 255., 243. / 255., 1.),
            Theme::Minimal if dark(&config) => (0.14, 0.16, 0.19, 1.),
            Theme::Minimal => (0.97, 0.98, 0.99, 1.),
            Theme::Paper if config.white_paper => (1., 1., 1., 1.),
            Theme::Paper if individual => (248. / 255., 241. / 255., 222. / 255., 1.),
            Theme::Paper => (246. / 255., 239. / 255., 219. / 255., 1.),
            Theme::Playful => match color.get() {
                NoteColor::Yellow => (0.98, 0.93, 0.74, 1.),
                NoteColor::Blue => (0.71, 0.85, 1., 1.),
                NoteColor::Purple => (0.86, 0.80, 0.94, 1.),
                NoteColor::Green if individual => (0.81, 0.97, 0.85, 1.),
                NoteColor::Green => (0.65, 0.96, 0.86, 1.),
            },
        };
        cr.set_source_rgba(r, g, b, a);
        let _ = cr.fill_preserve();
        if config.theme == Theme::Playful {
            cr.set_source_rgb(0.16, 0.15, 0.14);
            cr.set_line_width(if individual { 2.5 } else { 2. });
        } else {
            cr.set_source_rgba(0.22, 0.24, 0.27, 0.16);
            cr.set_line_width(0.7);
        }
        let _ = cr.stroke_preserve();
        cr.clip();
        if config.theme != Theme::Minimal {
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
        if config.theme == Theme::Minimal {
            let (r, g, b) = match color.get() {
                NoteColor::Yellow => (0.79, 0.60, 0.20),
                NoteColor::Blue if individual => (0.24, 0.62, 0.91),
                NoteColor::Blue => (0.39, 0.61, 0.82),
                NoteColor::Purple => (0.67, 0.51, 0.75),
                NoteColor::Green => (0.45, 0.65, 0.51),
            };
            cr.set_source_rgb(r, g, b);
            cr.rectangle(0., 0., w, if individual { 3. } else { 2. });
            let _ = cr.fill();
        }
    });
    area
}

fn sheet_path(
    cr: &gtk::cairo::Context,
    w: f64,
    h: f64,
    shape: Shape,
    radius: (f64, f64),
    playful: bool,
    seed: u64,
) {
    let r = radius.0.min(w / 4.).min(h / 4.);
    let bottom = radius.1.min(w / 4.).min(h / 4.);
    cr.new_path();
    cr.move_to(r, 0.);
    edge_path(cr, (r, 0.), (w - r, 0.), playful, false, seed ^ 11);
    cr.curve_to(w, 0., w, 0., w, r);
    edge_path(cr, (w, r), (w, h - bottom), playful, false, seed ^ 23);
    cr.curve_to(w, h, w, h, w - bottom, h);
    edge_path(
        cr,
        (w - bottom, h),
        (bottom, h),
        playful || shape != Shape::Square,
        !playful,
        seed ^ 37,
    );
    cr.curve_to(0., h, 0., h, 0., h - bottom);
    edge_path(cr, (0., h - bottom), (0., r), playful, false, seed ^ 53);
    cr.curve_to(0., 0., 0., 0., r, 0.);
    cr.close_path();
}

fn edge_path(
    cr: &gtk::cairo::Context,
    from: (f64, f64),
    to: (f64, f64),
    wobble: bool,
    torn: bool,
    seed: u64,
) {
    if !wobble {
        cr.line_to(to.0, to.1);
        return;
    }
    let (dx, dy) = (to.0 - from.0, to.1 - from.1);
    let len = dx.hypot(dy);
    if len < 1. {
        cr.line_to(to.0, to.1);
        return;
    }
    let step = if torn { 6. } else { 30. };
    let amplitude = if torn { 2.6 } else { 2.3 };
    let noise = |i: usize| {
        let mut n = seed.wrapping_add((i as u64).wrapping_mul(0x9e3779b97f4a7c15));
        n = (n ^ (n >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        n = (n ^ (n >> 27)).wrapping_mul(0x94d049bb133111eb);
        ((n ^ (n >> 31)) as f64 / u64::MAX as f64 * 2. - 1.) * amplitude
    };
    let point = |t: f64, offset: f64| {
        (
            from.0 + dx / len * t - dy / len * offset,
            from.1 + dy / len * t + dx / len * offset,
        )
    };
    let mut points = vec![from];
    for i in 1..=(len / step) as usize {
        let t = i as f64 * step;
        if t >= len {
            break;
        }
        points.push(point(t, noise(i) * ((len - t) / step).min(1.)));
    }
    points.push(to);
    if torn {
        for &(x, y) in &points[1..] {
            cr.line_to(x, y);
        }
    } else {
        // Shared tangents at segment joins keep all four edges smooth.
        for i in 0..points.len() - 1 {
            let p0 = points[i.saturating_sub(1)];
            let p1 = points[i];
            let p2 = points[i + 1];
            let p3 = points[(i + 2).min(points.len() - 1)];
            cr.curve_to(
                p1.0 + (p2.0 - p0.0) / 6.,
                p1.1 + (p2.1 - p0.1) / 6.,
                p2.0 - (p3.0 - p1.0) / 6.,
                p2.1 - (p3.1 - p1.1) / 6.,
                p2.0,
                p2.1,
            );
        }
    }
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
