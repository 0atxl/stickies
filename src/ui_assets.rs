use gtk::{gio, glib, pango::prelude::FontMapExt, prelude::*};
use std::{cell::RefCell, collections::HashMap, path::Path};

thread_local! {
    static ICON_MASKS: RefCell<HashMap<(String, i32), gtk::cairo::ImageSurface>> =
        RefCell::new(HashMap::new());
}

#[cfg(target_os = "linux")]
#[link(name = "fontconfig")]
unsafe extern "C" {
    fn FcConfigGetCurrent() -> *mut std::ffi::c_void;
    fn FcConfigAppFontAddFile(config: *mut std::ffi::c_void, file: *const u8) -> i32;
}

fn register_font_path(path: &Path, name: &str) -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(target_os = "windows")]
    let gdi_ok = {
        use std::os::windows::ffi::OsStrExt;
        use windows_sys::Win32::Graphics::Gdi::{AddFontResourceExW, FR_PRIVATE};
        let wide: Vec<u16> = path
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        unsafe { AddFontResourceExW(wide.as_ptr(), FR_PRIVATE, std::ptr::null()) > 0 }
    };

    let pango_ok = pangocairo::FontMap::default()
        .add_font_file(path)
        .is_ok();

    #[cfg(target_os = "linux")]
    {
        let c_path = std::ffi::CString::new(path.as_os_str().as_encoded_bytes())?;
        let fc_ok =
            unsafe { FcConfigAppFontAddFile(FcConfigGetCurrent(), c_path.as_ptr().cast()) } != 0;
        if !pango_ok && !fc_ok {
            return Err(format!("could not register font {name}").into());
        }
    }

    #[cfg(target_os = "windows")]
    {
        if !pango_ok && !gdi_ok {
            return Err(format!("could not register font {name}").into());
        }
    }

    Ok(())
}

pub fn register() -> Result<(), Box<dyn std::error::Error>> {
    gio::resources_register_include!("stickies.gresource")?;
    let cache = glib::user_cache_dir().join("stickies").join("fonts");
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
            register_font_path(&path, name)
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
    let size = crate::edge_surface::scale_px(16);
    image.set_content_width(size);
    image.set_content_height(size);
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
    let size = crate::edge_surface::scale_px(18);
    image.set_content_width(size);
    image.set_content_height(size);
    image.set_valign(gtk::Align::Center);
    image.set_halign(gtk::Align::Center);
    image.set_widget_name(&format!("stickies-icon-{name}"));
    set_icon(
        &image,
        &format!("/dev/stickies/Stickies/icons/stickies-{name}-symbolic.svg"),
    );
    image
}

fn icon_mask(path: &str, px: i32) -> gtk::cairo::ImageSurface {
    let px = px.max(16);
    ICON_MASKS.with(|masks| {
        let key = (path.to_owned(), px);
        if let Some(mask) = masks.borrow().get(&key).cloned() {
            return mask;
        }
        let pixbuf = gtk::gdk_pixbuf::Pixbuf::from_resource_at_scale(path, px, px, true)
            .expect("bundled icon");
        let bytes = pixbuf.read_pixel_bytes();
        let stride = gtk::cairo::Format::A8.stride_for_width(px as u32).unwrap() as usize;
        let size = px as usize;
        let mut alpha = vec![0u8; stride * size];
        for y in 0..size {
            for x in 0..size {
                alpha[y * stride + x] = bytes[y * pixbuf.rowstride() as usize + x * 4 + 3];
            }
        }
        let mask = gtk::cairo::ImageSurface::create_for_data(
            alpha,
            gtk::cairo::Format::A8,
            px,
            px,
            stride as i32,
        )
        .unwrap();
        masks.borrow_mut().insert(key, mask.clone());
        mask
    })
}

// Preserve the supplied SVG strokes, using their alpha as a mask for the CSS color.
fn set_icon(image: &gtk::DrawingArea, path: &str) {
    let path = path.to_owned();
    image.set_draw_func(move |widget, cr, width, height| {
        let color = widget.style_context().color();
        cr.set_source_rgba(
            color.red().into(),
            color.green().into(),
            color.blue().into(),
            color.alpha().into(),
        );
        let side = width.min(height).max(1);
        let scale = widget.scale_factor().max(1);
        let px = (side * scale).max(16);
        let mask = icon_mask(&path, px);
        let side_f = f64::from(side);
        let px_f = f64::from(px);
        cr.translate(
            (f64::from(width) - side_f) / 2.,
            (f64::from(height) - side_f) / 2.,
        );
        cr.scale(side_f / px_f, side_f / px_f);
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
