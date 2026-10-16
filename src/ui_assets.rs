use gtk::{gio, glib, prelude::*};
use std::{
    cell::RefCell,
    collections::HashMap,
    ffi::{CString, c_void},
};

thread_local! {
    static ICON_MASKS: RefCell<HashMap<String, gtk::cairo::ImageSurface>> = RefCell::new(HashMap::new());
}

#[link(name = "fontconfig")]
unsafe extern "C" {
    fn FcConfigGetCurrent() -> *mut c_void;
    fn FcConfigAppFontAddFile(config: *mut c_void, file: *const u8) -> i32;
}

pub fn register() -> Result<(), Box<dyn std::error::Error>> {
    gio::resources_register_include!("stickies.gresource")?;
    let cache = glib::user_cache_dir().join("stickies/fonts");
    for name in [
        "Virgil.ttf",
        "Inter.ttf",
        "JetBrainsMono.ttf",
        "Kalam-Regular.ttf",
        "Kalam-Bold.ttf",
    ] {
        let register_font = || -> Result<(), Box<dyn std::error::Error>> {
            std::fs::create_dir_all(&cache)?;
            let bytes = gio::resources_lookup_data(
                &format!("/dev/stickies/Stickies/fonts/{name}"),
                gio::ResourceLookupFlags::NONE,
            )?;
            let path = cache.join(name);
            if std::fs::read(&path).ok().as_deref() != Some(bytes.as_ref()) {
                std::fs::write(&path, &bytes)?;
            }
            let path = CString::new(path.as_os_str().as_encoded_bytes())?;
            // Fontconfig borrows the path during this call; the bundled file remains cached.
            if unsafe { FcConfigAppFontAddFile(FcConfigGetCurrent(), path.as_ptr().cast()) } == 0 {
                return Err(format!("could not register font {name}").into());
            }
            Ok(())
        };
        if let Err(error) = register_font() {
            eprintln!("Warning: could not load bundled font {name}: {error}");
        }
    }
    Ok(())
}

pub fn icon(name: &str, label: &str) -> gtk::Button {
    let button = gtk::Button::new();
    button.set_child(Some(&image(name)));
    button.set_tooltip_text(Some(label));
    button.update_property(&[gtk::accessible::Property::Label(label)]);
    button.set_valign(gtk::Align::Center);
    button.add_css_class("icon-control");
    button
}

pub fn edge_icon(name: &str, label: &str) -> gtk::Button {
    let image = gtk::DrawingArea::new();
    image.set_content_width(16);
    image.set_content_height(16);
    image.set_valign(gtk::Align::Center);
    image.set_halign(gtk::Align::Center);
    let asset = match name {
        "plus" => "new",
        "grid" => "notes",
        _ => "search",
    };
    set_icon(
        &image,
        &format!("/dev/stickies/Stickies/icons/edge/{asset}.svg"),
    );
    let button = gtk::Button::new();
    button.set_child(Some(&image));
    button.set_tooltip_text(Some(label));
    button.update_property(&[gtk::accessible::Property::Label(label)]);
    button.set_valign(gtk::Align::Center);
    button
}

pub fn image(name: &str) -> gtk::DrawingArea {
    let image = gtk::DrawingArea::new();
    image.set_content_width(18);
    image.set_content_height(18);
    image.set_valign(gtk::Align::Center);
    image.set_halign(gtk::Align::Center);
    image.set_widget_name(&format!("stickies-icon-{name}"));
    set_icon(
        &image,
        &format!("/dev/stickies/Stickies/icons/stickies-{name}-symbolic.svg"),
    );
    image
}

// Preserve the supplied SVG strokes, using their alpha as a mask for the CSS color.
fn set_icon(image: &gtk::DrawingArea, path: &str) {
    let mask = ICON_MASKS.with(|masks| {
        let cached = masks.borrow().get(path).cloned();
        if let Some(mask) = cached {
            return mask;
        }
        let pixbuf = gtk::gdk_pixbuf::Pixbuf::from_resource_at_scale(path, 36, 36, true)
            .expect("bundled icon");
        let bytes = pixbuf.read_pixel_bytes();
        let stride = gtk::cairo::Format::A8.stride_for_width(36).unwrap() as usize;
        let mut alpha = vec![0u8; stride * 36];
        for y in 0..36usize {
            for x in 0..36usize {
                alpha[y * stride + x] = bytes[y * pixbuf.rowstride() as usize + x * 4 + 3];
            }
        }
        let mask = gtk::cairo::ImageSurface::create_for_data(
            alpha,
            gtk::cairo::Format::A8,
            36,
            36,
            stride as i32,
        )
        .unwrap();
        masks.borrow_mut().insert(path.to_owned(), mask.clone());
        mask
    });
    image.set_draw_func(move |widget, cr, width, height| {
        let color = widget.style_context().color();
        cr.set_source_rgba(
            color.red().into(),
            color.green().into(),
            color.blue().into(),
            color.alpha().into(),
        );
        let side = f64::from(width.min(height));
        cr.translate(
            (f64::from(width) - side) / 2.,
            (f64::from(height) - side) / 2.,
        );
        cr.scale(side / 36., side / 36.);
        let _ = cr.mask_surface(&mask, 0., 0.);
    });
}

pub fn refresh_icons(widget: &gtk::Widget, playful: bool, filled_pins: bool) {
    if let Ok(image) = widget.clone().downcast::<gtk::DrawingArea>()
        && let Some(name) = image.widget_name().strip_prefix("stickies-icon-")
    {
        let filled = filled_pins
            && name == "pin"
            && image
                .parent()
                .and_downcast::<gtk::ToggleButton>()
                .is_some_and(|b| b.is_active());
        let state = if playful {
            if filled {
                "playful-filled"
            } else {
                "playful-outline"
            }
        } else if filled {
            "standard-filled"
        } else {
            "standard-icon"
        };
        if !image.has_css_class(state) {
            let asset = match name {
                "plus" => "plus-new-note",
                "settings" => "settings-gear",
                "popout" => "pop-out-window",
                "more" => "more-dots",
                "pin" if filled => "pin-filled",
                other => other,
            };
            let path = if playful
                && !matches!(
                    name,
                    "copy" | "delete" | "bullets" | "numbering" | "restore"
                ) {
                format!("/dev/stickies/Stickies/icons/playful/{asset}.svg")
            } else if filled {
                "/dev/stickies/Stickies/icons/stickies-pin-filled-symbolic.svg".to_string()
            } else {
                format!("/dev/stickies/Stickies/icons/stickies-{name}-symbolic.svg")
            };
            set_icon(&image, &path);
            for class in [
                "playful-filled",
                "playful-outline",
                "standard-icon",
                "standard-filled",
            ] {
                image.remove_css_class(class);
            }
            image.add_css_class(state);
        }
    }
    let mut child = widget.first_child();
    while let Some(w) = child {
        refresh_icons(&w, playful, filled_pins);
        child = w.next_sibling();
    }
}
