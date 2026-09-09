//! Desktop-specific notification state. Other desktops keep the strip visible.
use gtk::{gio, glib, prelude::*};
use serde::Deserialize;

#[derive(Deserialize)]
struct NotificationSettings {
    dnd: bool,
}

pub fn watch_do_not_disturb(on_change: impl Fn(bool) + 'static) -> Option<gio::FileMonitor> {
    // Omarchy's shell currently writes this path, including with a custom XDG_STATE_HOME.
    let path = glib::home_dir().join(".local/state/omarchy/notifications.json");
    if std::env::var_os("OMARCHY_PATH").is_none() || !path.parent()?.is_dir() {
        return None;
    }
    let read = move || {
        std::fs::read_to_string(&path)
            .ok()
            .and_then(|text| serde_json::from_str::<NotificationSettings>(&text).ok())
            .is_some_and(|state| state.dnd)
    };
    on_change(read());
    let directory = gio::File::for_path(glib::home_dir().join(".local/state/omarchy"));
    let monitor = directory
        .monitor_directory(
            gio::FileMonitorFlags::WATCH_MOVES,
            None::<&gio::Cancellable>,
        )
        .ok()?;
    monitor.connect_changed(move |_, file, other, _| {
        let relevant = |file: &gio::File| {
            file.basename()
                .is_some_and(|name| name == std::path::Path::new("notifications.json"))
        };
        if relevant(file) || other.is_some_and(relevant) {
            on_change(read());
        }
    });
    Some(monitor)
}
