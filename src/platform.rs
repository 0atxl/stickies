//! Desktop-specific notification and fullscreen state.

#[cfg(target_os = "linux")]
use gtk::{gio, glib, prelude::*};
#[cfg(target_os = "linux")]
use serde::Deserialize;

#[cfg(target_os = "linux")]
#[derive(Deserialize)]
struct NotificationSettings {
    dnd: bool,
}

pub struct DndWatch {
    #[cfg(target_os = "linux")]
    _monitor: gio::FileMonitor,
    #[cfg(target_os = "windows")]
    source: Option<gtk::glib::SourceId>,
}

#[cfg(target_os = "windows")]
impl Drop for DndWatch {
    fn drop(&mut self) {
        if let Some(source) = self.source.take() {
            source.remove();
        }
    }
}

#[cfg(target_os = "linux")]
pub fn watch_do_not_disturb(on_change: impl Fn(bool) + 'static) -> Option<DndWatch> {
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
    Some(DndWatch { _monitor: monitor })
}

#[cfg(target_os = "windows")]
pub fn watch_do_not_disturb(on_change: impl Fn(bool) + 'static) -> Option<DndWatch> {
    use std::{cell::Cell, time::Duration};
    use windows_sys::Win32::UI::Shell::{
        QUNS_BUSY, QUNS_PRESENTATION_MODE, QUNS_RUNNING_D3D_FULL_SCREEN,
        SHQueryUserNotificationState,
    };

    let query = || -> (bool, bool) {
        let mut state: i32 = 0;
        let ok = unsafe { SHQueryUserNotificationState(&mut state) } == 0;
        if !ok {
            return (false, false);
        }
        // Presentation mode is explicit. Quiet time is a first-login/upgrade
        // period, not the Windows DND switch, which this API does not expose.
        let dnd = state == QUNS_PRESENTATION_MODE;
        let fullscreen = matches!(
            state,
            QUNS_BUSY | QUNS_RUNNING_D3D_FULL_SCREEN | QUNS_PRESENTATION_MODE
        );
        (dnd, fullscreen)
    };

    let (initial_dnd, initial_fs) = query();
    crate::edge_surface::set_fullscreen_busy(initial_fs);
    on_change(initial_dnd);

    let last_dnd = Cell::new(initial_dnd);
    let last_fs = Cell::new(initial_fs);
    let source = gtk::glib::timeout_add_local(Duration::from_secs(2), move || {
        let (dnd, fullscreen) = query();
        if last_fs.replace(fullscreen) != fullscreen {
            crate::edge_surface::set_fullscreen_busy(fullscreen);
        }
        if last_dnd.replace(dnd) != dnd {
            on_change(dnd);
        }
        gtk::glib::ControlFlow::Continue
    });

    Some(DndWatch {
        source: Some(source),
    })
}
