use crate::{
    app::NoteColor,
    config::{Appearance, Config, Font, Paper, Shape, Theme},
};
use gtk::prelude::*;
use std::{cell::RefCell, rc::Rc};

pub fn toggle(name: &str, label: &str) -> gtk::ToggleButton {
    let button = gtk::ToggleButton::new();
    button.set_icon_name(name);
    button.set_tooltip_text(Some(label));
    button.add_css_class("icon-control");
    button
}

pub fn color_class(color: NoteColor) -> String {
    format!("note-{}", color.name())
}

pub fn set_color(widget: &impl IsA<gtk::Widget>, color: NoteColor) {
    for color in NoteColor::ALL {
        widget.remove_css_class(&color_class(color));
    }
    widget.add_css_class(&color_class(color));
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

pub fn note_font_provider(
    owner: &impl IsA<gtk::Widget>,
    title: &gtk::Entry,
    body: &gtk::TextView,
    priority: u32,
) -> (gtk::CssProvider, String) {
    // Display providers need a per-view class to keep different notes isolated.
    let class = format!("note-font-{}", gtk::glib::uuid_string_random());
    title.add_css_class(&class);
    body.add_css_class(&class);
    let provider = gtk::CssProvider::new();
    let display = owner.display();
    gtk::style_context_add_provider_for_display(&display, &provider, priority);
    let cleanup = provider.clone();
    owner.connect_destroy(move |_| {
        gtk::style_context_remove_provider_for_display(&display, &cleanup);
    });
    (provider, class)
}

pub fn apply_note_font(
    provider: &gtk::CssProvider,
    class: &str,
    config: &Config,
    style: &crate::config::NoteStyle,
) {
    let font = match config.font {
        Font::Theme if config.theme != Theme::Minimal => "'Virgil 3 YOFF', cursive",
        Font::Theme | Font::Grotesk => "Inter, sans-serif",
        Font::Mono => "'JetBrains Mono', monospace",
        Font::Kalam => "Kalam, cursive",
        Font::Handwritten => "'Virgil 3 YOFF', cursive",
    };
    let adjustment = match style.text_size {
        crate::config::NoteTextSize::Small => -3,
        crate::config::NoteTextSize::Medium => 0,
        crate::config::NoteTextSize::Large => 3,
    };
    let body_size = 17 + adjustment;
    let title_size = match config.theme {
        Theme::Paper | Theme::Playful => 30,
        Theme::Minimal => 28,
    } + adjustment;
    let weight = match style.legacy_text_weight {
        crate::config::LegacyTextWeight::Regular => 400,
        crate::config::LegacyTextWeight::Bold => 700,
    };
    provider.load_from_data(&crate::edge_surface::scale_css(&format!(
        ".{class}.editor-title, .{class}.editor-body, .{class}.editor-body text, .{class}.inline-title, .{class}.inline-body, .{class}.inline-body text {{ font-family: {font}; font-weight: {weight}; }}
         .{class}.editor-title, .{class}.inline-title {{ font-size: {title_size}px; }}
         .{class}.editor-body, .{class}.editor-body text, .{class}.inline-body, .{class}.inline-body text {{ font-size: {body_size}px; }}"
    )));
}

pub fn palette_css() -> String {
    NoteColor::ALL
        .into_iter()
        .map(|color| {
            let class = color_class(color);
            let (r, g, b) = color.accent();
            let swatch_ink = if matches!(
                color,
                NoteColor::Black | NoteColor::Indigo | NoteColor::Brown | NoteColor::Magenta
            ) {
                "white"
            } else {
                "#23262b"
            };
            format!(
                ".icon-control.{class} {{ color: rgb({r},{g},{b}); }}
            .note-tab.{class} {{ --edge-accent: rgb({r},{g},{b}); }}
            .edge-result-color.{class}, .management .{class} .tag-bar, .current-color-dot.{class},
            .note-controls .color-swatch.{class} {{ background: rgb({r},{g},{b}); }}
            .note-controls .color-swatch.{class} {{ color: {swatch_ink}; }}
            "
            )
        })
        .collect::<String>()
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
        Font::Kalam => "Kalam, cursive",
        Font::Handwritten => "'Virgil 3 YOFF', cursive",
    };
    let palette_css = palette_css();
    let tab_colors = NoteColor::ALL
        .into_iter()
        .map(|color| {
            let (r, g, b) = color.fill();
            format!(
                ".note-tab.{} {{ --tab-fill: rgb({:.0},{:.0},{:.0}); background: var(--tab-fill); color: #202624; }}",
                color_class(color),
                r * 255.,
                g * 255.,
                b * 255.
            )
        })
        .collect::<String>();
    provider.load_from_data(&crate::edge_surface::scale_css(&format!(r#"
        /* No decorative focus/press rings. Value indicators are styled below. */
        * {{ outline: none; }}
        button, modelbutton {{ background: transparent; color: inherit; border-color: transparent; box-shadow: none; }}
        .edge-window, .hit-area, .note-window {{ background: transparent; }}
        .metadata, .dim-label {{ color: {muted}; font-size: 11px; }}
        .editor-title {{ background: transparent; color: inherit; border: none; outline: none; box-shadow: none; font-family: {font}; font-weight: 400; padding: 0; min-width: 0; }}
        .editor-body, .editor-body text {{ background: transparent; color: inherit; font-family: {font}; }}
        .editor-body {{ caret-color: currentColor; }}
        .editor-card button {{ color: inherit; }}
        .icon-control {{ background: transparent; border: none; box-shadow: none; min-width: 24px; min-height: 24px; padding: 5px; border-radius: 7px; }}
        .icon-control > button {{ background: transparent; border: none; box-shadow: none; color: inherit; }}
        .note-tab {{ background: {bg}; color: {fg}; padding: 0; min-width: 0; min-height: 0; }}
        {palette_css}
        {tab_colors}
        .edge-window .note-tab, .edge-window .strip-control, .edge-search, .edge-search * {{ transition: none; animation: none; }}
        .tab-title {{ font-family: Inter, sans-serif; font-size: 11px; font-weight: 600; }}
        .edge-result-color {{ min-width: 8px; min-height: 8px; border-radius: 50%; margin: 0; }}
        .edge-search {{ font-family: Inter, sans-serif; font-size: 13px; background: {bg}; color: {fg}; border: 1px solid {line}; border-radius: 10px; padding: 8px; }}
        popover.edge-search-popover, popover.edge-search-popover > contents {{ background: transparent; padding: 0; margin: 0; border: none; box-shadow: none; }}
        .edge-search searchentry {{ min-height: 30px; background: transparent; color: {fg}; border: 1px solid {line}; border-radius: 8px; box-shadow: none; }}
        .edge-search list {{ background: transparent; }}
        .edge-search row {{ background: transparent; color: {fg}; min-height: 24px; padding: 2px 6px; border-radius: 6px; }}
        .edge-search row:selected {{ background: alpha({fg}, .12); }}
        .edge-search .edge-result-title {{ font-size: 13px; }}
        .edge-search .edge-create-result {{ background: transparent; color: {fg}; border: none; box-shadow: none; font-size: 12px; padding: 8px; }}
        .individual popover > contents {{ background: var(--rail-bg); color: var(--note-ink); border: 1px solid var(--rail-line); border-radius: 8px; padding: 8px; box-shadow: 0 2px 6px alpha(black, .12); }}
        popover button, popover modelbutton {{ min-height: 24px; padding: 4px 8px; font-size: 12px; }}
        scrollbar.vertical, scrollbar.vertical:hover {{ min-width: 6px; background: transparent; }}
        scrollbar.vertical slider, scrollbar.vertical:hover slider {{ min-width: 3px; min-height: 24px; margin: 0 1px; border: none; border-radius: 3px; }}
        scrollbar.horizontal, scrollbar.horizontal:hover {{ min-height: 6px; background: transparent; }}
        scrollbar.horizontal slider, scrollbar.horizontal:hover slider {{ min-height: 3px; min-width: 24px; margin: 1px 0; border: none; border-radius: 3px; }}
        .individual {{ --note-ink: #2d2c29; --rail-bg: #fbf8f0; --rail-line: #e3e0d8; --rail-selected: #ece8df; }}
        .individual.minimal {{ --note-ink: #292f36; --rail-bg: #ffffff; --rail-line: #cdd4db; --rail-selected: #dceaff; }}
        .individual.minimal-dark {{ --note-ink: #edf1f5; --rail-bg: #23262b; --rail-line: #424a55; --rail-selected: #37414d; }}
        .individual.playful {{ --note-ink: #202923; --rail-bg: #fffefe; --rail-line: #e6e1e5; --rail-selected: #ffe5ee; }}
        .individual .editor-card {{ color: var(--note-ink); padding: 26px 26px 28px 30px; }}
        .individual .icon-control {{ border-radius: 8px; }}
        .individual .editor-title {{ font-size: 23px; }}
        .individual.minimal .editor-title {{ font-size: 20px; font-weight: 600; }}
        .individual .note-resize-grip {{ color: var(--note-ink); opacity: 0; }}
        .individual:hover .note-resize-grip {{ opacity: .3; }}
        .individual .editor-body, .individual .editor-body text {{ font-size: 21px; }}
        .note-controls .text-controls {{ border: 1px solid var(--rail-line); border-radius: 8px; }}
        .note-controls .text-controls button {{ min-width: 0; min-height: 24px; padding: 3px 6px; border: 0; border-radius: 0; background: transparent; box-shadow: none; font-family: Inter, sans-serif; font-size: 12px; font-weight: 400; }}
        .note-controls .text-controls button:first-child {{ border-radius: 7px 0 0 7px; }}
        .note-controls .text-controls button:last-child {{ border-radius: 0 7px 7px 0; }}
        .note-controls .text-controls button + button {{ border-left: 1px solid var(--rail-line); }}
        .note-controls .text-controls button:checked {{ background: var(--rail-selected); }}
        .individual.playful .editor-card {{ padding: 26px 24px 28px; }}
        .individual.playful .note-controls {{ --rail-line: #e6e1e5; --rail-selected: #ffe5ee; }}
        .individual.playful .note-controls .text-controls button:checked {{ background: #ffe5ee; color: #202923; }}
        .individual.playful .inline-check,
        .management.playful .inline-check {{ min-height: 0; min-width: 0; padding: 0; margin: 0 8px 0 0; }}
        .individual.playful .inline-check check,
        .management.playful .inline-check check {{ min-width: 14px; min-height: 14px; padding: 0; margin: 0; -gtk-icon-size: 14px; border: 1.5px solid currentColor; border-radius: 3px; }}
        .individual.clean-note .inline-check check, .management.clean-note .inline-check check {{ min-width: 16px; min-height: 16px; -gtk-icon-size: 16px; border-width: 1px; }}
        .inline-check {{ margin-right: 8px; color: inherit; }}
        .inline-check check {{ color: inherit; min-width: 16px; min-height: 16px; border-radius: 3px; border: 1px solid currentColor; background: transparent; box-shadow: none; }}
        .inline-check check:checked {{ background: #30353b; border-color: #30353b; color: white; }}
        .individual.clean-note .inline-check check:checked,
        .management.clean-note .inline-check check:checked {{ background: transparent; border-color: currentColor; color: alpha(#202923, .55); }}
        .individual.minimal-dark .inline-check check:checked, .management.minimal-dark .inline-check check:checked {{ background: #edf1f5; border-color: #edf1f5; color: #23262b; }}

        .individual.minimal .note-controls {{ --note-ink: #292f36; --rail-line: #cdd4db; --rail-selected: #dceaff; }}
        .individual.minimal-dark .note-controls {{ --note-ink: #edf1f5; --rail-line: #424a55; --rail-selected: #37414d; }}
        .individual.minimal:not(.minimal-dark) {{ --note-ink: #252c34; --rail-bg: #ffffff; --rail-line: #dfe5ec; --rail-selected: #eaf3ff; }}
        .individual.minimal:not(.minimal-dark) .editor-card {{ padding: 28px 24px 26px; }}
        .individual.minimal:not(.minimal-dark) .note-controls {{ --note-ink: #252c34; --rail-line: #dfe5ec; --rail-selected: #eaf3ff; }}
        .individual.minimal:not(.minimal-dark) .inline-check,
        .management.minimal:not(.minimal-dark) .inline-check {{ min-height: 0; min-width: 0; padding: 0; margin: 0 8px 0 0; }}
        .individual.minimal:not(.minimal-dark) .inline-check check,
        .management.minimal:not(.minimal-dark) .inline-check check {{ min-width: 14px; min-height: 14px; padding: 0; margin: 0; -gtk-icon-size: 14px; border: 1px solid currentColor; border-radius: 3px; }}
        .individual.minimal-dark {{ --note-ink: #edf0f4; --rail-bg: #272c32; --rail-line: #414953; --rail-selected: #34465e; }}
        .individual.minimal-dark .editor-card {{ padding: 28px 24px 26px; }}
        .individual.minimal-dark .note-controls {{ --note-ink: #edf0f4; --rail-line: #414953; --rail-selected: #34465e; }}
        .individual.minimal-dark .note-controls .text-controls button:checked {{ background: #34465e; }}
        .individual.minimal-dark .inline-check,
        .management.minimal-dark .inline-check {{ min-height: 0; min-width: 0; padding: 0; margin: 0 8px 0 0; }}
        .individual.minimal-dark .inline-check check,
        .management.minimal-dark .inline-check check {{ min-width: 14px; min-height: 14px; padding: 0; margin: 0; -gtk-icon-size: 14px; border: 1px solid currentColor; border-radius: 3px; }}
        .note-footer {{ font-family: Inter, sans-serif; font-size: 13px; background: var(--rail-bg); color: var(--note-ink); border: none; border-radius: 22px; padding: 2px 6px; box-shadow: 0 1px 4px alpha(black, .10); }}
        .note-drag-handle {{ padding: 0; }}
        .note-drag-grip {{ background: alpha(var(--note-ink), .30); border-radius: 2px; }}
        .note-drag-handle:hover .note-drag-grip {{ background: alpha(var(--note-ink), .50); }}
        .note-footer button {{ min-width: 24px; min-height: 24px; padding: 4px; border: none; border-radius: 7px; background: transparent; box-shadow: none; color: inherit; opacity: .65; }}
        .note-footer menubutton {{ padding: 0; }}
        .current-color-dot {{ border-radius: 50%; border: 1px solid alpha(currentColor, .18); }}
        .note-popover contents {{ background: var(--rail-bg); color: var(--note-ink); border: 1px solid var(--rail-line); border-radius: 8px; padding: 8px; box-shadow: 0 2px 6px alpha(black, .12); font-family: Inter, sans-serif; font-size: 12px; }}
        .note-popover > arrow {{ min-width: 10px; min-height: 6px; }}
        popover.note-emoji, popover.note-emoji text {{ font-family: Inter, sans-serif; font-size: 13px; font-weight: 400; }}
        popover.note-emoji > contents {{ padding: 4px; }}
        popover.note-emoji emoji {{ font-size: 20px; padding: 3px; }}
        popover.note-emoji .emoji-searchbar, popover.note-emoji .emoji-toolbar {{ padding: 4px; border-spacing: 2px; }}
        popover.note-emoji .emoji-section {{ min-width: 18px; min-height: 20px; padding: 2px; -gtk-icon-size: 14px; }}
        popover.note-emoji searchentry {{ font-size: 13px; min-height: 28px; padding: 2px 6px; }}
        .selection-tools button {{ min-width: 24px; min-height: 24px; padding: 4px; background: transparent; border: none; box-shadow: none; color: inherit; opacity: .85; }}
        .note-more > button {{ background: transparent; border: none; box-shadow: none; border-radius: 6px; padding: 4px 8px; min-height: 24px; color: inherit; }}
        .note-footer button:hover, .note-more > button:hover, .note-controls button:hover, .pattern-choices button:hover, .selection-tools button:hover {{ background-color: alpha(currentColor, .08); }}
        .selection-tools button:checked {{ background-color: var(--rail-selected); opacity: 1; }}
        .note-controls .color-swatch:hover {{ box-shadow: inset 0 0 0 20px alpha(black, .08); }}
        .font-choices button {{ min-width: 28px; min-height: 28px; padding: 3px 4px; border-radius: 6px; background: transparent; border: none; box-shadow: none; }}
        .font-choices button:checked {{ background: var(--rail-selected); }}
        .note-more separator {{ margin: 5px 2px; background: var(--rail-line); min-height: 1px; }}
        .note-more .list-choices {{ margin-top: 6px; margin-bottom: 4px; }}
        .note-more .list-choices button {{ min-height: 24px; padding: 4px; border-radius: 6px; background: transparent; border: none; box-shadow: none; }}
        .note-more .list-choices button:hover {{ background: alpha(currentColor, .08); }}
        .pattern-choices button {{ min-width: 0; min-height: 0; padding: 4px; border-radius: 6px; background: transparent; border: none; box-shadow: none; color: inherit; }}
        .pattern-choices button:checked {{ background: var(--rail-selected); }}
        .note-controls .color-swatch {{ min-width: 22px; min-height: 22px; padding: 0; border-radius: 50%; color: #23262b; border: 1px solid alpha(currentColor, .18); box-shadow: none; }}
        .individual:not(.minimal):not(.playful) .editor-card {{ padding: 28px 24px 26px 32px; }}
        .individual:not(.minimal):not(.playful) .note-controls {{ --rail-selected: #ffe4c4; }}
        .individual:not(.minimal):not(.playful) .inline-check,
        .management:not(.minimal):not(.playful) .inline-check {{ min-height: 0; min-width: 0; padding: 0; margin: 0 7px 0 0; }}
        .individual:not(.minimal):not(.playful) .inline-check check,
        .management:not(.minimal):not(.playful) .inline-check check {{ min-width: 14px; min-height: 14px; padding: 0; margin: 0; -gtk-icon-size: 14px; border: 1.5px solid currentColor; border-radius: 2px; }}
        .note-popover button {{ opacity: 1; }}
        .note-controls .color-swatch:disabled {{ opacity: .4; }}

    "#)));
}

/// Draw only the paper decoration; text and controls remain native GTK widgets.
pub fn paper_background(
    config: Rc<RefCell<Config>>,
    color: Rc<std::cell::Cell<NoteColor>>,
    seed: Rc<std::cell::Cell<u64>>,
) -> gtk::DrawingArea {
    let area = gtk::DrawingArea::new();
    area.set_draw_func(move |_, cr, width, height| {
        let config = config.borrow();
        let wavy = config.theme == Theme::Playful && !config.clean_playful();
        let inset = if wavy { 20 } else { 16 };
        let w = f64::from(width - inset).max(1.);
        let h = f64::from(height - inset).max(1.);
        cr.translate(8., 8.);
        if config.clean_playful() {
            // A spread shadow stays inside the existing transparent drawing margin.
            for spread in (1..=6).rev() {
                let spread = f64::from(spread);
                let _ = cr.save();
                cr.translate(-spread / 2., 2. - spread / 2.);
                note_outline(cr, w + spread, h + spread, &config, seed.get());
                cr.set_source_rgba(0.12, 0.17, 0.14, 0.018);
                let _ = cr.fill();
                let _ = cr.restore();
            }
        } else if wavy {
            let _ = cr.save();
            cr.translate(6., 7.);
            note_outline(cr, w, h, &config, seed.get());
            cr.set_source_rgba(0.12, 0.16, 0.13, 0.45);
            let _ = cr.fill();
            let _ = cr.restore();
        } else {
            for offset in (1..=5).rev() {
                let _ = cr.save();
                cr.translate(0., f64::from(offset));
                note_outline(cr, w, h, &config, seed.get());
                let alpha = if config.theme == Theme::Paper {
                    0.032
                } else if dark(&config) {
                    0.045
                } else {
                    0.025
                };
                cr.set_source_rgba(0.16, 0.18, 0.22, alpha);
                let _ = cr.fill();
                let _ = cr.restore();
            }
        }
        note_outline(cr, w, h, &config, seed.get());
        let (r, g, b, a) = match config.theme {
            Theme::Minimal if dark(&config) => (39. / 255., 44. / 255., 50. / 255., 1.),
            Theme::Minimal => (1., 1., 1., 1.),
            Theme::Paper => (1., 0.966, 0.864, 1.),
            Theme::Playful => {
                let (r, g, b) = color.get().fill();
                (r, g, b, 1.)
            }
        };
        if config.clean_playful() {
            let fill = gtk::cairo::LinearGradient::new(0., 0., w * 0.35, h);
            fill.add_color_stop_rgb(
                0.,
                r + (1. - r) * 0.20,
                g + (1. - g) * 0.20,
                b + (1. - b) * 0.20,
            );
            fill.add_color_stop_rgb(0.65, r, g, b);
            fill.add_color_stop_rgb(1., r * 0.985, g * 0.985, b * 0.985);
            let _ = cr.set_source(&fill);
        } else {
            cr.set_source_rgba(r, g, b, a);
        }
        let _ = cr.fill_preserve();
        if config.clean_playful() {
            cr.set_source_rgba(0.12, 0.16, 0.13, 0.12);
            cr.set_line_width(0.8);
        } else if wavy {
            cr.set_source_rgb(0.10, 0.12, 0.11);
            cr.set_line_width(3.2);
        } else if config.theme == Theme::Minimal && dark(&config) {
            cr.set_source_rgba(0.75, 0.80, 0.86, 0.12);
            cr.set_line_width(0.7);
        } else {
            cr.set_source_rgba(
                0.22,
                0.24,
                0.27,
                if config.theme == Theme::Minimal && !dark(&config) {
                    0.12
                } else {
                    0.16
                },
            );
            cr.set_line_width(0.7);
        }
        let _ = cr.stroke_preserve();
        cr.clip();
        {
            if config.theme == Theme::Minimal && dark(&config) {
                cr.set_source_rgba(0.82, 0.87, 0.92, 0.14);
            } else {
                cr.set_source_rgba(0.25, 0.27, 0.32, 0.14);
            }
            cr.set_line_width(0.7);
            match config.paper {
                Paper::Plain => {}
                Paper::Dots => {
                    for x in (16..width).step_by(20) {
                        for y in (60..height).step_by(20) {
                            cr.arc(f64::from(x), f64::from(y), 1., 0., std::f64::consts::TAU);
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
        if config.theme != Theme::Playful {
            let (r, g, b) = color.get().accent();
            let (r, g, b) = (
                f64::from(r) / 255.,
                f64::from(g) / 255.,
                f64::from(b) / 255.,
            );
            cr.set_source_rgb(r, g, b);
            if config.theme == Theme::Minimal {
                cr.rectangle(0., 0., w, 5.);
            } else {
                cr.rectangle(0., 0., 9., h);
            }
            let _ = cr.fill();
        }
    });
    area
}

/// Keep the note and its docked pointer clipping on the same outline.
pub(super) fn note_outline(cr: &gtk::cairo::Context, w: f64, h: f64, config: &Config, seed: u64) {
    if config.clean_playful() {
        let radius = 22_f64.min(w / 4.).min(h / 4.);
        let quarter = std::f64::consts::FRAC_PI_2;
        cr.new_path();
        cr.arc(w - radius, radius, radius, -quarter, 0.);
        cr.arc(w - radius, h - radius, radius, 0., quarter);
        cr.arc(radius, h - radius, radius, quarter, 2. * quarter);
        cr.arc(radius, radius, radius, 2. * quarter, 3. * quarter);
        cr.close_path();
        return;
    }
    let shape = if config.theme == Theme::Minimal {
        Shape::Square
    } else {
        config.shape
    };
    let bottom = if config.theme == Theme::Paper && shape != Shape::Square {
        0.
    } else {
        18.
    };
    sheet_path(
        cr,
        w,
        h,
        shape,
        (18., bottom),
        config.theme == Theme::Playful,
        seed,
    );
}

pub(super) fn sheet_path(
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
    let step = if torn { 6. } else { 48. };
    let amplitude = if torn { 3.2 } else { 3.4 };
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
    let mut t = 0.;
    let mut i = 0;
    loop {
        i += 1;
        // Uneven fiber breaks ride on broader, shallow dips in the paper edge.
        t += if torn {
            3. + (noise(i + 101) / amplitude + 1.) * 3.5
        } else {
            step
        };
        if t >= len {
            break;
        }
        let offset = if torn {
            noise(i) * 0.45
                + (t / 23. + (seed % 7) as f64).sin() * 2.1
                + (t / 47. + (seed % 11) as f64).sin() * 1.5
        } else {
            noise(i)
        };
        let taper = if torn {
            (t / 12.).min(1.) * ((len - t) / 12.).min(1.)
        } else {
            ((len - t) / step).min(1.)
        };
        points.push(point(t, offset * taper));
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
