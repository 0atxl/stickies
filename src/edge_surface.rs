//! Platform abstraction for the screen-edge overlay surfaces.
//! On Linux, delegates directly to `gtk4-layer-shell`.
//! On Windows, emulates Layer Shell semantics on undecorated GTK4 windows using Win32 APIs.

#[cfg(target_os = "linux")]
pub use gtk4_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};

#[cfg(target_os = "linux")]
pub fn check_display_support() -> Result<(), String> {
    if !gtk4_layer_shell::is_supported() {
        return Err(
            "Stickies requires Wayland with zwlr_layer_shell_v1 version 4 or newer; Layer Shell is unavailable on this display."
                .into(),
        );
    }
    let protocol_version = gtk4_layer_shell::protocol_version();
    if protocol_version < 4 {
        return Err(format!(
            "Stickies requires zwlr_layer_shell_v1 version 4 or newer for on-demand keyboard focus; this display provides version {protocol_version}."
        ));
    }
    Ok(())
}

#[cfg(target_os = "linux")]
pub fn sync_surface_geometry(
    _window: &gtk::ApplicationWindow,
    _monitor: Option<&gtk::gdk::Monitor>,
    _width: i32,
    _height: i32,
) {
}

#[cfg(target_os = "linux")]
pub fn apply_input_region(
    window: &gtk::ApplicationWindow,
    primary_bounds: Option<(i32, i32, i32, i32)>,
    _extra_bounds: Option<(i32, i32, i32, i32)>,
    _visual_pad: i32,
    _passthrough_bounds: Option<(i32, i32, i32, i32)>,
) {
    use gtk::prelude::*;
    if let Some(surface) = window.surface() {
        let region = gtk::cairo::Region::create();
        if let Some((x, y, width, height)) = primary_bounds {
            let _ = region.union_rectangle(&gtk::cairo::RectangleInt::new(x, y, width, height));
        }
        surface.set_input_region(Some(&region));
    }
}

#[cfg(target_os = "linux")]
#[allow(dead_code)]
pub fn set_fullscreen_busy(_busy: bool) {}

#[cfg(target_os = "linux")]
pub fn is_pointer_in_window(_window: &gtk::ApplicationWindow) -> bool {
    true
}

#[cfg(target_os = "linux")]
pub fn connect_hover_fallback(
    _window: &gtk::ApplicationWindow,
    _on_tick: impl Fn(bool) + 'static,
) {
}

#[cfg(target_os = "linux")]
pub fn sync_resize_grip(_resize: &gtk::Box) {}

#[cfg(target_os = "linux")]
pub fn init_dpi_awareness() {}

#[cfg(target_os = "linux")]
pub fn configure_display_dpi() {}

#[cfg(target_os = "linux")]
pub fn ui_scale() -> f64 {
    1.0
}

#[cfg(target_os = "linux")]
pub fn scale_px(px: i32) -> i32 {
    px
}

#[cfg(target_os = "linux")]
pub fn scale_css(css: &str) -> String {
    css.to_owned()
}

#[cfg(target_os = "windows")]
mod imp_windows {
    use gtk::{glib, prelude::*};
    use std::{
        cell::{Cell, RefCell},
        collections::HashMap,
        ffi::c_void,
        time::Duration,
    };
    use windows_sys::Win32::{
        Foundation::{HWND, POINT, RECT},
        Graphics::Gdi::{CombineRgn, CreateRectRgn, DeleteObject, RGN_OR, SetWindowRgn},
        UI::{
            HiDpi::{
                DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, GetDpiForSystem,
                SetProcessDpiAwarenessContext,
            },
            WindowsAndMessaging::{
                ASFW_ANY, AllowSetForegroundWindow, GWL_EXSTYLE, GetCursorPos, GetSystemMetrics,
                GetWindowLongPtrW, GetWindowRect, HWND_NOTOPMOST, HWND_TOPMOST, SM_SWAPBUTTON,
                SWP_NOMOVE, SWP_NOACTIVATE, SWP_NOOWNERZORDER, SWP_NOSIZE, SWP_NOZORDER,
                SetForegroundWindow, SetWindowLongPtrW, SetWindowPos, WS_EX_APPWINDOW,
                WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST,
                WS_EX_TRANSPARENT,
            },
        },
    };

    #[cfg_attr(target_os = "windows", link(name = "gtk-4"))]
    unsafe extern "C" {
        fn gdk_win32_surface_get_handle(surface: *mut c_void) -> HWND;
    }

    #[cfg_attr(target_os = "windows", link(name = "user32"))]
    unsafe extern "system" {
        fn GetAsyncKeyState(vkey: i32) -> i16;
    }

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub enum Edge {
        Left,
        Right,
        Top,
        Bottom,
    }

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub enum Layer {
        Top,
        Overlay,
    }

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub enum KeyboardMode {
        None,
        OnDemand,
        Exclusive,
    }

    #[derive(Clone)]
    struct WinSurfaceState {
        window: glib::WeakRef<gtk::ApplicationWindow>,
        layer: Layer,
        keyboard_mode: KeyboardMode,
        monitor: Option<gtk::gdk::Monitor>,
        size: (i32, i32),
        primary_bounds: Option<(i32, i32, i32, i32)>,
        extra_bounds: Option<(i32, i32, i32, i32)>,
        visual_pad: i32,
        passthrough: bool,
    }

    impl Default for WinSurfaceState {
        fn default() -> Self {
            Self {
                window: glib::WeakRef::new(),
                layer: Layer::Top,
                keyboard_mode: KeyboardMode::None,
                monitor: None,
                size: (0, 0),
                primary_bounds: None,
                extra_bounds: None,
                visual_pad: 0,
                passthrough: false,
            }
        }
    }

    thread_local! {
        static STATES: RefCell<HashMap<usize, WinSurfaceState>> = RefCell::new(HashMap::new());
        static FULLSCREEN_BUSY: Cell<bool> = const { Cell::new(false) };
    }

    fn key(window: &gtk::ApplicationWindow) -> usize {
        window.as_ptr() as usize
    }

    pub fn window_hwnd(window: &gtk::ApplicationWindow) -> Option<HWND> {
        let surface = window.surface()?;
        let hwnd = unsafe { gdk_win32_surface_get_handle(surface.as_ptr() as *mut c_void) };
        (!hwnd.is_null()).then_some(hwnd)
    }

    fn rect_contains(
        (x, y, width, height): (i32, i32, i32, i32),
        pad: i32,
        lx: i32,
        ly: i32,
    ) -> bool {
        if width <= 0 || height <= 0 {
            return false;
        }
        let left = (x - pad).max(0);
        let top = (y - pad).max(0);
        let right = x + width + pad;
        let bottom = y + height + pad;
        lx >= left && lx < right && ly >= top && ly < bottom
    }

    fn is_cursor_in_state(
        window: &gtk::ApplicationWindow,
        hwnd: HWND,
        state: &WinSurfaceState,
    ) -> bool {
        if state.passthrough {
            return false;
        }
        let scale = window.surface().map_or(1, |s| s.scale_factor().max(1));
        unsafe {
            let mut pt = POINT { x: 0, y: 0 };
            let mut rect: RECT = std::mem::zeroed();
            if GetCursorPos(&mut pt) == 0 || GetWindowRect(hwnd, &mut rect) == 0 {
                return false;
            }
            let lx = (pt.x - rect.left) / scale;
            let ly = (pt.y - rect.top) / scale;
            let in_primary = state
                .primary_bounds
                .is_some_and(|b| rect_contains(b, state.visual_pad, lx, ly));
            let in_extra = state
                .extra_bounds
                .is_some_and(|b| rect_contains(b, state.visual_pad, lx, ly));
            in_primary || in_extra
        }
    }

    pub fn is_pointer_in_window(window: &gtk::ApplicationWindow) -> bool {
        if !window.is_visible() {
            return false;
        }
        let Some(hwnd) = window_hwnd(window) else {
            return false;
        };
        STATES.with(|map| {
            map.borrow()
                .get(&key(window))
                .is_some_and(|state| is_cursor_in_state(window, hwnd, state))
        })
    }

    fn desired_z_order(layer: Layer) -> HWND {
        if layer == Layer::Top && FULLSCREEN_BUSY.with(Cell::get) {
            HWND_NOTOPMOST
        } else {
            HWND_TOPMOST
        }
    }

    fn sync_hwnd_styles_and_z(window: &gtk::ApplicationWindow) {
        let Some(hwnd) = window_hwnd(window) else {
            return;
        };
        let state = STATES.with(|map| map.borrow().get(&key(window)).cloned());
        let Some(state) = state else {
            return;
        };
        unsafe {
            let current = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
            let mut target = (current | (WS_EX_TOPMOST as isize) | (WS_EX_TOOLWINDOW as isize))
                & !(WS_EX_APPWINDOW as isize);
            if state.keyboard_mode == KeyboardMode::None {
                target |= WS_EX_NOACTIVATE as isize;
            } else {
                target &= !(WS_EX_NOACTIVATE as isize);
            }
            if state.passthrough {
                target |= (WS_EX_LAYERED as isize) | (WS_EX_TRANSPARENT as isize);
            } else {
                target &= !(WS_EX_TRANSPARENT as isize);
            }
            if target != current {
                SetWindowLongPtrW(hwnd, GWL_EXSTYLE, target);
            }
            let z = desired_z_order(state.layer);
            SetWindowPos(
                hwnd,
                z,
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_NOOWNERZORDER,
            );
            if state.keyboard_mode == KeyboardMode::Exclusive {
                let _ = AllowSetForegroundWindow(ASFW_ANY);
                let _ = SetForegroundWindow(hwnd);
            }
        }
    }

    pub fn set_fullscreen_busy(busy: bool) {
        FULLSCREEN_BUSY.with(|cell| cell.set(busy));
        let windows: Vec<gtk::ApplicationWindow> = STATES.with(|map| {
            map.borrow()
                .values()
                .filter_map(|s| s.window.upgrade())
                .collect()
        });
        for window in windows {
            sync_hwnd_styles_and_z(&window);
        }
    }

    pub fn check_display_support() -> Result<(), String> {
        Ok(())
    }

    pub fn init_dpi_awareness() {
        unsafe {
            let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        }
    }

    pub fn configure_display_dpi() {
        let Some(settings) = gtk::Settings::default() else {
            return;
        };
        let sys_dpi = unsafe { GetDpiForSystem() };
        let gdk_scale = gtk::gdk::Display::default()
            .and_then(|d| d.monitors().item(0))
            .and_then(|o| o.downcast::<gtk::gdk::Monitor>().ok())
            .map_or(1, |m| m.scale_factor().max(1));
        if gdk_scale == 1 && sys_dpi > 96 && settings.find_property("gtk-xft-dpi").is_some() {
            settings.set_property("gtk-xft-dpi", (sys_dpi as i32) * 1024);
        }
    }

    pub fn ui_scale() -> f64 {
        let sys_dpi = unsafe { GetDpiForSystem() };
        if sys_dpi <= 96 {
            return 1.0;
        }
        let gdk_scale = gtk::gdk::Display::default()
            .and_then(|d| d.monitors().item(0))
            .and_then(|o| o.downcast::<gtk::gdk::Monitor>().ok())
            .map_or(1, |m| m.scale_factor().max(1));
        let ratio = f64::from(sys_dpi) / (96.0 * f64::from(gdk_scale));
        if (ratio - 1.0).abs() < 0.05 {
            1.0
        } else {
            ratio.clamp(1.0, 3.0)
        }
    }

    pub fn scale_px(px: i32) -> i32 {
        let scale = ui_scale();
        if (scale - 1.0).abs() < 0.05 || px == 0 {
            return px;
        }
        (f64::from(px) * scale).round() as i32
    }

    pub fn scale_css(css: &str) -> String {
        let scale = ui_scale();
        if (scale - 1.0).abs() < 0.05 {
            return css.to_owned();
        }
        let col_w = scale_px(240);
        let col_line = col_w - 1;
        let replaced = css.replace(
            "239px, var(--hairline) 239px, var(--hairline) 240px, transparent 240px",
            &format!(
                "\x01{col_line}px, var(--hairline) \x01{col_line}px, var(--hairline) \x01{col_w}px, transparent \x01{col_w}px"
            ),
        );
        let bytes = replaced.as_bytes();
        let mut out = String::with_capacity(replaced.len() + 256);
        let mut i = 0;
        while i < bytes.len() {
            if bytes[i] == 0x01 {
                i += 1;
                while i < bytes.len() && (bytes[i].is_ascii_digit() || bytes[i] == b'p' || bytes[i] == b'x') {
                    out.push(bytes[i] as char);
                    i += 1;
                }
                continue;
            }
            let start = i;
            let prev_ok = start == 0
                || matches!(
                    bytes[start - 1],
                    b' ' | b'\t' | b'\n' | b':' | b'(' | b','
                );
            if prev_ok {
                let mut j = start;
                if j < bytes.len() && bytes[j] == b'-' {
                    j += 1;
                }
                let digits_start = j;
                while j < bytes.len() && bytes[j].is_ascii_digit() {
                    j += 1;
                }
                if j > digits_start {
                    if j + 1 < bytes.len() && bytes[j] == b'.' && bytes[j + 1].is_ascii_digit() {
                        j += 1;
                        while j < bytes.len() && bytes[j].is_ascii_digit() {
                            j += 1;
                        }
                    }
                    if j + 1 < bytes.len() && bytes[j] == b'p' && bytes[j + 1] == b'x' {
                        let next_ok = j + 2 >= bytes.len()
                            || (!bytes[j + 2].is_ascii_alphanumeric()
                                && bytes[j + 2] != b'_'
                                && bytes[j + 2] != b'-');
                        if next_ok {
                            if let Ok(val) = replaced[start..j].parse::<f64>() {
                                if val.abs() > 1.5 {
                                    let scaled = (val * scale).round() as i32;
                                    out.push_str(&format!("{scaled}px"));
                                } else {
                                    out.push_str(&replaced[start..j + 2]);
                                }
                                i = j + 2;
                                continue;
                            }
                        }
                    }
                }
            }
            let ch = replaced[i..].chars().next().unwrap();
            out.push(ch);
            i += ch.len_utf8();
        }
        out
    }

    pub fn cursor_screen_pos(window: &gtk::ApplicationWindow) -> Option<(i32, i32)> {
        let scale = window.surface().map_or(1, |s| s.scale_factor().max(1));
        unsafe {
            let mut pt = POINT { x: 0, y: 0 };
            if GetCursorPos(&mut pt) == 0 {
                return None;
            }
            Some((pt.x / scale, pt.y / scale))
        }
    }

    pub fn resize_window(window: &gtk::ApplicationWindow, width: i32, height: i32) {
        window.set_default_size(width, height);
        let Some(hwnd) = window_hwnd(window) else {
            return;
        };
        let scale = window.surface().map_or(1, |s| s.scale_factor().max(1));
        let w = (width * scale).max(1);
        let h = (height * scale).max(1);
        unsafe {
            SetWindowPos(
                hwnd,
                std::ptr::null_mut(),
                0,
                0,
                w,
                h,
                SWP_NOMOVE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_NOOWNERZORDER,
            );
        }
    }

    pub fn sync_surface_geometry(
        window: &gtk::ApplicationWindow,
        monitor: Option<&gtk::gdk::Monitor>,
        width: i32,
        height: i32,
    ) {
        STATES.with(|map| {
            let mut map = map.borrow_mut();
            let entry = map.entry(key(window)).or_default();
            entry.window.set(Some(window));
            entry.monitor = monitor.cloned();
            entry.size = (width, height);
        });
        let Some(hwnd) = window_hwnd(window) else {
            return;
        };
        let scale = window
            .surface()
            .map_or(1, |s| s.scale_factor().max(1));
        let (mon_x, mon_y) = monitor
            .map(|m| {
                let g = m.geometry();
                (g.x() * scale, g.y() * scale)
            })
            .unwrap_or((0, 0));
        let w = (width * scale).max(1);
        let h = (height * scale).max(1);
        let layer = STATES.with(|map| {
            map.borrow()
                .get(&key(window))
                .map_or(Layer::Top, |s| s.layer)
        });
        sync_hwnd_styles_and_z(window);
        unsafe {
            SetWindowPos(
                hwnd,
                desired_z_order(layer),
                mon_x,
                mon_y,
                w,
                h,
                SWP_NOACTIVATE | SWP_NOOWNERZORDER,
            );
        }
    }

    pub fn apply_input_region(
        window: &gtk::ApplicationWindow,
        primary_bounds: Option<(i32, i32, i32, i32)>,
        extra_bounds: Option<(i32, i32, i32, i32)>,
        visual_pad: i32,
        passthrough_bounds: Option<(i32, i32, i32, i32)>,
    ) {
        // On Windows, let SetWindowRgn control the OS hit-test region and keep GDK's
        // internal input_region unclipped so GDK never drops mouse-leave transitions
        // inside the padded HRGN border.
        if let Some(surface) = window.surface() {
            surface.set_input_region(None);
        }

        let passthrough = primary_bounds.is_none() && passthrough_bounds.is_some();
        let hwnd_opt = window_hwnd(window);
        STATES.with(|map| {
            let mut map = map.borrow_mut();
            let state = map.entry(key(window)).or_default();
            state.window.set(Some(window));
            state.primary_bounds = primary_bounds;
            state.extra_bounds = extra_bounds;
            state.visual_pad = visual_pad;
            state.passthrough = passthrough;
        });
        sync_hwnd_styles_and_z(window);

        let Some(hwnd) = hwnd_opt else {
            return;
        };
        let scale = window
            .surface()
            .map_or(1, |s| s.scale_factor().max(1));
        let make_rgn = |(x, y, w, h): (i32, i32, i32, i32), pad: i32| unsafe {
            let left = ((x - pad).max(0)) * scale;
            let top = ((y - pad).max(0)) * scale;
            let right = ((x + w + pad).max(0)) * scale;
            let bottom = ((y + h + pad).max(0)) * scale;
            CreateRectRgn(left, top, right, bottom)
        };

        unsafe {
            let hrgn = if let Some(rect) = primary_bounds.or(passthrough_bounds) {
                let base = make_rgn(rect, visual_pad);
                if let Some(extra) = extra_bounds {
                    let extra_rgn = make_rgn(extra, visual_pad);
                    CombineRgn(base, base, extra_rgn, RGN_OR);
                    DeleteObject(extra_rgn as _);
                }
                base
            } else {
                CreateRectRgn(0, 0, 0, 0)
            };
            SetWindowRgn(hwnd, hrgn, 1);
        }
    }

    pub fn connect_hover_fallback(
        window: &gtk::ApplicationWindow,
        on_tick: impl Fn(bool) + 'static,
    ) {
        let weak = window.downgrade();
        let source = glib::timeout_add_local(Duration::from_millis(50), move || {
            let Some(window) = weak.upgrade() else {
                return glib::ControlFlow::Break;
            };
            if !window.is_visible() {
                return glib::ControlFlow::Continue;
            }
            on_tick(is_pointer_in_window(&window));
            glib::ControlFlow::Continue
        });
        let source = RefCell::new(Some(source));
        window.connect_destroy(move |_| {
            if let Some(id) = source.borrow_mut().take() {
                id.remove();
            }
        });
    }

    fn is_primary_mouse_down() -> bool {
        unsafe {
            let swapped = GetSystemMetrics(SM_SWAPBUTTON) != 0;
            let vk = if swapped { 0x02 } else { 0x01 };
            (GetAsyncKeyState(vk) as u16 & 0x8000) != 0
        }
    }

    pub fn sync_resize_grip(resize: &gtk::Box) {
        let left = resize.halign() == gtk::Align::Start;
        let top = resize.valign() == gtk::Align::Start;
        let hit = scale_px(32);
        let pad = scale_px(10);
        resize.set_size_request(hit, hit);
        resize.set_margin_start(0);
        resize.set_margin_end(0);
        resize.set_margin_top(0);
        resize.set_margin_bottom(0);
        if let Some(mark) = resize.first_child() {
            mark.set_halign(if left {
                gtk::Align::Start
            } else {
                gtk::Align::End
            });
            mark.set_valign(if top {
                gtk::Align::Start
            } else {
                gtk::Align::End
            });
            mark.set_margin_start(if left { pad } else { 0 });
            mark.set_margin_end(if left { 0 } else { pad });
            mark.set_margin_top(if top { pad } else { 0 });
            mark.set_margin_bottom(if top { 0 } else { pad });
            mark.queue_draw();
        }
        resize.queue_draw();
    }

    pub fn connect_resize_drag(
        resize: &gtk::Box,
        window: &gtk::ApplicationWindow,
        on_begin: impl Fn() -> (i32, i32) + 'static,
        on_update: impl Fn(i32, i32, i32, i32) + 'static,
    ) {
        sync_resize_grip(resize);
        let click = gtk::GestureClick::new();
        click.set_button(1);
        click.set_propagation_phase(gtk::PropagationPhase::Capture);
        let weak_window = window.downgrade();
        let dragging = std::rc::Rc::new(Cell::new(false));
        let on_begin = std::rc::Rc::new(on_begin);
        let on_update = std::rc::Rc::new(on_update);
        click.connect_pressed(move |gesture, _, _, _| {
            gesture.set_state(gtk::EventSequenceState::Claimed);
            gesture.reset();
            if dragging.get() {
                return;
            }
            let Some(window) = weak_window.upgrade() else {
                return;
            };
            let Some((start_x, start_y)) = cursor_screen_pos(&window) else {
                return;
            };
            let (start_w, start_h) = on_begin();
            dragging.set(true);
            let dragging_loop = dragging.clone();
            let weak_win = window.downgrade();
            let on_update = on_update.clone();
            glib::timeout_add_local(Duration::from_millis(16), move || {
                let Some(win) = weak_win.upgrade() else {
                    dragging_loop.set(false);
                    return glib::ControlFlow::Break;
                };
                if !win.is_visible() || !is_primary_mouse_down() {
                    dragging_loop.set(false);
                    return glib::ControlFlow::Break;
                }
                if let Some((cx, cy)) = cursor_screen_pos(&win) {
                    on_update(start_w, start_h, cx - start_x, cy - start_y);
                }
                glib::ControlFlow::Continue
            });
        });
        resize.add_controller(click);
    }

    pub trait LayerShell {
        fn init_layer_shell(&self);
        fn set_namespace(&self, namespace: Option<&str>);
        fn set_exclusive_zone(&self, zone: i32);
        fn set_keyboard_mode(&self, mode: KeyboardMode);
        fn keyboard_mode(&self) -> KeyboardMode;
        fn set_monitor(&self, monitor: Option<&gtk::gdk::Monitor>);
        fn set_anchor(&self, edge: Edge, anchor: bool);
        fn set_layer(&self, layer: Layer);
        fn layer(&self) -> Layer;
        fn margin(&self, edge: Edge) -> i32;
    }

    impl LayerShell for gtk::ApplicationWindow {
        fn init_layer_shell(&self) {
            STATES.with(|map| {
                let mut map = map.borrow_mut();
                let entry = map.entry(key(self)).or_default();
                entry.window.set(Some(self));
            });
            self.connect_realize(|window| {
                sync_hwnd_styles_and_z(window);
                let info = STATES.with(|map| {
                    map.borrow()
                        .get(&key(window))
                        .map(|s| (s.monitor.clone(), s.size))
                });
                if let Some((monitor, (w, h))) = info
                    && w > 0
                    && h > 0
                {
                    sync_surface_geometry(window, monitor.as_ref(), w, h);
                }
            });
            self.connect_map(|window| {
                sync_hwnd_styles_and_z(window);
                let info = STATES.with(|map| {
                    map.borrow()
                        .get(&key(window))
                        .map(|s| (s.monitor.clone(), s.size))
                });
                if let Some((monitor, (w, h))) = info
                    && w > 0
                    && h > 0
                {
                    sync_surface_geometry(window, monitor.as_ref(), w, h);
                }
            });
            self.connect_destroy(|window| {
                STATES.with(|map| {
                    map.borrow_mut().remove(&key(window));
                });
            });
        }

        fn set_namespace(&self, _namespace: Option<&str>) {}

        fn set_exclusive_zone(&self, _zone: i32) {}

        fn set_keyboard_mode(&self, mode: KeyboardMode) {
            STATES.with(|map| {
                let mut map = map.borrow_mut();
                let entry = map.entry(key(self)).or_default();
                entry.window.set(Some(self));
                entry.keyboard_mode = mode;
            });
            sync_hwnd_styles_and_z(self);
        }

        fn keyboard_mode(&self) -> KeyboardMode {
            STATES.with(|map| {
                map.borrow()
                    .get(&key(self))
                    .map_or(KeyboardMode::None, |s| s.keyboard_mode)
            })
        }

        fn set_monitor(&self, monitor: Option<&gtk::gdk::Monitor>) {
            let size = STATES.with(|map| {
                let mut map = map.borrow_mut();
                let entry = map.entry(key(self)).or_default();
                entry.window.set(Some(self));
                entry.monitor = monitor.cloned();
                entry.size
            });
            if size.0 > 0 && size.1 > 0 {
                sync_surface_geometry(self, monitor, size.0, size.1);
            }
        }

        fn set_anchor(&self, _edge: Edge, _anchor: bool) {}

        fn set_layer(&self, layer: Layer) {
            STATES.with(|map| {
                let mut map = map.borrow_mut();
                let entry = map.entry(key(self)).or_default();
                entry.window.set(Some(self));
                entry.layer = layer;
            });
            sync_hwnd_styles_and_z(self);
        }

        fn layer(&self) -> Layer {
            STATES.with(|map| {
                map.borrow()
                    .get(&key(self))
                    .map_or(Layer::Top, |s| s.layer)
            })
        }

        fn margin(&self, _edge: Edge) -> i32 {
            0
        }
    }
}

#[cfg(target_os = "windows")]
pub use imp_windows::{
    Edge, KeyboardMode, Layer, LayerShell, apply_input_region, check_display_support,
    configure_display_dpi, connect_hover_fallback, connect_resize_drag, cursor_screen_pos,
    init_dpi_awareness, is_pointer_in_window, resize_window, scale_css, scale_px,
    set_fullscreen_busy, sync_resize_grip, sync_surface_geometry, ui_scale,
};
