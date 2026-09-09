use gtk::{gio, glib, prelude::*};
use std::ffi::{CString, c_void};

#[link(name = "fontconfig")]
unsafe extern "C" {
    fn FcConfigGetCurrent() -> *mut c_void;
    fn FcConfigAppFontAddFile(config: *mut c_void, file: *const u8) -> i32;
}

pub fn register() -> Result<(), Box<dyn std::error::Error>> {
    gio::resources_register_include!("stickies.gresource")?;
    let cache = glib::user_cache_dir().join("stickies/fonts");
    std::fs::create_dir_all(&cache)?;
    for name in ["Virgil.woff2", "Inter.ttf", "JetBrainsMono.ttf"] {
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
    }
    Ok(())
}

pub fn icon(name: &str, label: &str) -> gtk::Button {
    let button = gtk::Button::new();
    button.set_child(Some(&image(name)));
    button.set_tooltip_text(Some(label));
    button.set_valign(gtk::Align::Center);
    button.add_css_class("icon-control");
    button
}

pub fn image(name: &str) -> gtk::Image {
    let pixbuf = gtk::gdk_pixbuf::Pixbuf::from_resource_at_scale(
        &format!("/dev/stickies/Stickies/icons/stickies-{name}-symbolic.svg"),
        18,
        18,
        true,
    )
    .expect("bundled icon");
    gtk::Image::from_pixbuf(Some(&pixbuf))
}
