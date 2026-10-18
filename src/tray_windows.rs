//! Windows 11 system tray icon (`Shell_NotifyIconW`) companion for Stickies.

use gtk::{glib, prelude::*};
use std::{
    cell::{Cell, RefCell},
    mem::zeroed,
    ptr::null_mut,
};
use windows_sys::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, POINT, WPARAM},
    Graphics::Gdi::{CreateBitmap, DeleteObject},
    System::LibraryLoader::GetModuleHandleW,
    UI::{
        Shell::{
            NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NOTIFYICONDATAW, Shell_NotifyIconW,
        },
        WindowsAndMessaging::{
            AppendMenuW, CreateIconIndirect, CreatePopupMenu, CreateWindowExW, DefWindowProcW,
            DestroyIcon, DestroyMenu, DestroyWindow, GWLP_USERDATA, GetCursorPos,
            GetWindowLongPtrW, HICON, ICONINFO, IDI_APPLICATION, LoadIconW, MF_SEPARATOR,
            MF_STRING, RegisterClassW, RegisterWindowMessageW, SetForegroundWindow,
            SetWindowLongPtrW, TPM_BOTTOMALIGN, TPM_LEFTALIGN, TPM_NONOTIFY, TPM_RETURNCMD,
            TrackPopupMenu, WM_APP, WM_CONTEXTMENU, WM_LBUTTONUP, WM_RBUTTONUP, WNDCLASSW,
        },
    },
};

const WM_TRAYICON: u32 = WM_APP + 42;
const TRAY_UID: u32 = 1001;

const CMD_NEW_NOTE: usize = 1;
const CMD_TOGGLE_STRIP: usize = 2;
const CMD_SEARCH: usize = 3;
const CMD_ALL_NOTES: usize = 4;
const CMD_QUIT: usize = 5;

thread_local! {
    static APP_HANDLE: RefCell<Option<gtk::Application>> = const { RefCell::new(None) };
    static TASKBAR_CREATED: Cell<u32> = const { Cell::new(0) };
}

fn wide_null(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

fn add_icon(hwnd: HWND, hicon: HICON) -> bool {
    let mut nid: NOTIFYICONDATAW = unsafe { zeroed() };
    nid.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
    nid.hWnd = hwnd;
    nid.uID = TRAY_UID;
    nid.uFlags = NIF_ICON | NIF_MESSAGE | NIF_TIP;
    nid.uCallbackMessage = WM_TRAYICON;
    nid.hIcon = hicon;
    let tip = wide_null("Stickies — Left-click: Toggle Strip | Right-click: Menu");
    for (i, &ch) in tip.iter().take(nid.szTip.len() - 1).enumerate() {
        nid.szTip[i] = ch;
    }
    unsafe { Shell_NotifyIconW(NIM_ADD, &nid) != 0 }
}

fn load_app_icon() -> (HICON, bool) {
    let bytes = glib::Bytes::from_static(include_bytes!("../data/dev.stickies.Stickies.png"));
    let stream = gtk::gio::MemoryInputStream::from_bytes(&bytes);
    if let Ok(pixbuf) = gtk::gdk_pixbuf::Pixbuf::from_stream_at_scale(
        &stream,
        32,
        32,
        true,
        None::<&gtk::gio::Cancellable>,
    ) {
        let w = pixbuf.width();
        let h = pixbuf.height();
        let rowstride = pixbuf.rowstride() as usize;
        let n_channels = pixbuf.n_channels() as usize;
        let raw = pixbuf.read_pixel_bytes();
        let mut bgra = vec![0u8; (w * h * 4) as usize];
        for y in 0..h as usize {
            for x in 0..w as usize {
                let src = y * rowstride + x * n_channels;
                let dst = (y * w as usize + x) * 4;
                bgra[dst] = raw[src + 2];
                bgra[dst + 1] = raw[src + 1];
                bgra[dst + 2] = raw[src];
                bgra[dst + 3] = if n_channels >= 4 { raw[src + 3] } else { 255 };
            }
        }
        unsafe {
            let color_bmp = CreateBitmap(w, h, 1, 32, bgra.as_ptr() as *const _);
            let mask_bmp = CreateBitmap(w, h, 1, 1, null_mut());
            let icon = if !color_bmp.is_null() && !mask_bmp.is_null() {
                let info = ICONINFO {
                    fIcon: 1,
                    xHotspot: 0,
                    yHotspot: 0,
                    hbmMask: mask_bmp,
                    hbmColor: color_bmp,
                };
                CreateIconIndirect(&info)
            } else {
                null_mut()
            };
            if !color_bmp.is_null() {
                DeleteObject(color_bmp as _);
            }
            if !mask_bmp.is_null() {
                DeleteObject(mask_bmp as _);
            }
            if !icon.is_null() {
                return (icon, true);
            }
        }
    }
    (unsafe { LoadIconW(null_mut(), IDI_APPLICATION) }, false)
}

unsafe extern "system" fn tray_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if msg != 0 && msg == TASKBAR_CREATED.with(Cell::get) {
        // The host owns this slot; it stores a handle value, never a pointer
        // into the movable TrayIcon. Explorer has discarded the old icon.
        let hicon = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) } as HICON;
        if !hicon.is_null() && !add_icon(hwnd, hicon) {
            eprintln!("Stickies could not restore its tray icon after taskbar recreation.");
        }
        return 0;
    }
    if msg == WM_TRAYICON {
        let event = (lparam as u32) & 0xFFFF;
        if event == WM_LBUTTONUP {
            glib::idle_add_local_once(|| {
                APP_HANDLE.with(|cell| {
                    if let Some(app) = cell.borrow().as_ref() {
                        app.activate_action("toggle-strip", None);
                    }
                });
            });
            return 0;
        } else if event == WM_RBUTTONUP || event == WM_CONTEXTMENU {
            unsafe {
                let menu = CreatePopupMenu();
                if !menu.is_null() {
                    let s_new = wide_null("New Note");
                    let s_toggle = wide_null("Toggle Edge Strip");
                    let s_search = wide_null("Quick Search");
                    let s_all = wide_null("All Notes && Settings");
                    let s_quit = wide_null("Quit Stickies");

                    AppendMenuW(menu, MF_STRING, CMD_NEW_NOTE, s_new.as_ptr());
                    AppendMenuW(menu, MF_STRING, CMD_TOGGLE_STRIP, s_toggle.as_ptr());
                    AppendMenuW(menu, MF_STRING, CMD_SEARCH, s_search.as_ptr());
                    AppendMenuW(menu, MF_STRING, CMD_ALL_NOTES, s_all.as_ptr());
                    AppendMenuW(menu, MF_SEPARATOR, 0, null_mut());
                    AppendMenuW(menu, MF_STRING, CMD_QUIT, s_quit.as_ptr());

                    let mut pt = POINT { x: 0, y: 0 };
                    GetCursorPos(&mut pt);
                    SetForegroundWindow(hwnd);
                    let cmd = TrackPopupMenu(
                        menu,
                        TPM_RETURNCMD | TPM_NONOTIFY | TPM_BOTTOMALIGN | TPM_LEFTALIGN,
                        pt.x,
                        pt.y,
                        0,
                        hwnd,
                        null_mut(),
                    ) as usize;
                    DestroyMenu(menu);

                    if cmd != 0 {
                        glib::idle_add_local_once(move || {
                            APP_HANDLE.with(|cell| {
                                let Some(app) = cell.borrow().clone() else {
                                    return;
                                };
                                match cmd {
                                    CMD_NEW_NOTE => app.activate_action("new-note", None),
                                    CMD_TOGGLE_STRIP => app.activate_action("toggle-strip", None),
                                    CMD_SEARCH => app.activate_action("search", None),
                                    CMD_ALL_NOTES => app.activate_action("show-all-notes", None),
                                    CMD_QUIT => app.activate_action("quit", None),
                                    _ => (),
                                }
                            });
                        });
                    }
                }
            }
            return 0;
        }
    }
    unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
}

pub struct TrayIcon {
    hwnd: HWND,
    hicon: HICON,
    owns_icon: bool,
}

impl TrayIcon {
    pub fn new(application: &gtk::Application) -> Option<Self> {
        APP_HANDLE.with(|cell| {
            cell.replace(Some(application.clone()));
        });
        unsafe {
            let message = RegisterWindowMessageW(wide_null("TaskbarCreated").as_ptr());
            if message == 0 {
                eprintln!("Stickies could not register taskbar recreation notifications.");
                return None;
            }
            TASKBAR_CREATED.with(|cell| cell.set(message));
            let hinstance = GetModuleHandleW(null_mut());
            let class_name = wide_null("StickiesWin32TrayHost");
            let mut wc: WNDCLASSW = zeroed();
            wc.lpfnWndProc = Some(tray_wnd_proc);
            wc.hInstance = hinstance;
            wc.lpszClassName = class_name.as_ptr();
            RegisterClassW(&wc);

            let hwnd = CreateWindowExW(
                0,
                class_name.as_ptr(),
                class_name.as_ptr(),
                0,
                0,
                0,
                0,
                0,
                null_mut(),
                null_mut(),
                hinstance,
                null_mut(),
            );
            if hwnd.is_null() {
                return None;
            }

            let (hicon, owns_icon) = load_app_icon();
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, hicon as isize);
            if !add_icon(hwnd, hicon) {
                if owns_icon {
                    let _ = DestroyIcon(hicon);
                }
                DestroyWindow(hwnd);
                return None;
            }
            Some(Self {
                hwnd,
                hicon,
                owns_icon,
            })
        }
    }
}

impl Drop for TrayIcon {
    fn drop(&mut self) {
        APP_HANDLE.with(|cell| {
            cell.replace(None);
        });
        unsafe {
            let mut nid: NOTIFYICONDATAW = zeroed();
            nid.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
            nid.hWnd = self.hwnd;
            nid.uID = TRAY_UID;
            let _ = Shell_NotifyIconW(NIM_DELETE, &nid);
            SetWindowLongPtrW(self.hwnd, GWLP_USERDATA, 0);
            if self.owns_icon {
                let _ = DestroyIcon(self.hicon);
            }
            let _ = DestroyWindow(self.hwnd);
        }
    }
}
