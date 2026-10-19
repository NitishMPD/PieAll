//! Native Win32 UI: tray, Ctrl+` hotkey, GDI action ring, dashboard, parameter editor, result window.
//! Single UI thread. Capture and action execution run on worker threads and post results back.
#![allow(clippy::missing_safety_doc)]

use crate::actions::{self, Action, InputKind};
use pieall::updates;
use std::collections::BTreeMap;
use std::mem::zeroed;
use std::os::windows::ffi::OsStringExt;
use std::path::PathBuf;
use std::ptr::{null, null_mut};
use std::sync::atomic::{AtomicPtr, Ordering};
use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::Graphics::Gdi::*;
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::Threading::{CreateMutexW, GetCurrentThreadId};
use windows_sys::Win32::UI::HiDpi::*;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::*;
use windows_sys::Win32::UI::Shell::*;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

// Palette (COLORREF is 0x00BBGGRR).
const fn rgb(r: u32, g: u32, b: u32) -> u32 {
    r | (g << 8) | (b << 16)
}
const BASE: u32 = rgb(0xF3, 0xEB, 0xDD);
const WEDGE: u32 = rgb(0xF7, 0xD2, 0xAE);
const SELECTED: u32 = rgb(0xC9, 0x4F, 0x22);
const INK: u32 = rgb(0x29, 0x25, 0x1F);
const ON_SELECTED: u32 = rgb(0xFF, 0xF8, 0xEE);
const RIM: u32 = rgb(0xD9, 0xC6, 0xA8);
const CENTER_FILL: u32 = rgb(0xFB, 0xF6, 0xEC);
const WEDGE_EDGE: u32 = rgb(0xFF, 0xF8, 0xEC);

// Geometry in DIPs.
const PIE_DIP: i32 = 320;
const CENTER_R: f64 = 52.0; // 104 DIP center diameter
const RING_IN: f64 = 58.0; // 6 DIP gap from center
const RING_OUT: f64 = 152.0;
const GAP: f64 = 6.0;
const CORNER_R: f64 = 9.0;
const MAX_WEDGES: usize = 6;

const WM_APP_TRAY: u32 = WM_APP + 1;
const WM_APP_CAPTURED: u32 = WM_APP + 2;
const WM_APP_RESULT: u32 = WM_APP + 3;
const WM_APP_SHOW: u32 = WM_APP + 4;
const WM_APP_PREVIEW: u32 = WM_APP + 5;
const WM_APP_UPDATE: u32 = WM_APP + 6;
const TIMER_UPDATE: usize = 2;
const TIMER_CAPTURE: usize = 1;
const CAPTURE_MS: u32 = 1500;
const PREVIEW_H: i32 = 150;
const MORE: usize = usize::MAX; // ring entry that opens the dashboard list

const ID_SEARCH: i32 = 104;
const ID_LIST: i32 = 105;
const ID_OPEN: i32 = 101;
const ID_CLIP: i32 = 102;
const ID_RING: i32 = 106;
const ID_UPDATE: i32 = 107;
const IDOK_: i32 = 1;
const IDCANCEL_: i32 = 2;
const HOTKEY_ID: i32 = 1;

const C_MAIN: &str = "PieAllMain";
const C_PIE: &str = "PieAllPie";
const C_DASH: &str = "PieAllDash";
const C_EDIT: &str = "PieAllEditor";
const C_RESULT: &str = "PieAllResult";
const C_PREVIEW: &str = "PieAllPreview";

struct Done {
    title: String,
    result: Result<String, String>,
}

enum UpdateEvent {
    Checked(Result<Option<updates::Release>, String>),
    Downloaded(Result<updates::Download, String>),
    Progress(u8),
}

/// Bounded thumbnail of the first selected image, BGRX top-down, plus a faded copy for the crop overlay.
struct Preview {
    ow: u32,
    oh: u32,
    tw: i32,
    th: i32,
    px: Vec<u8>,
    faded: Vec<u8>,
}

struct App {
    actions: Vec<Action>,
    main: HWND,
    pie: HWND,
    dash: HWND,
    c_target: HWND,
    c_search: HWND,
    c_list: HWND,
    c_status: HWND,
    editor: HWND,
    edit_action: usize,
    edit_fields: Vec<(&'static str, HWND)>,
    preview_wnd: HWND,
    preview: Option<Preview>,
    preview_note: String,
    preview_gen: u64,
    restore_to: usize,
    paths: Vec<PathBuf>,
    text: Option<String>,
    source: usize,
    source_pid: u32,
    ring: Vec<usize>,
    more: usize,
    sel: Option<usize>,
    list_map: Vec<usize>,
    resolving: bool,
    gen: u64,
    busy: u32,
    update_busy: bool,
    update_available: Option<updates::Release>,
    update_ready: Option<updates::Download>,
    c_update: HWND,
    c_update_status: HWND,
    pie_active: bool,
    taskbar_msg: u32,
    dpi: i32,
    font: HFONT,
    font_bold: HFONT,
    font_small: HFONT,
    bg: HBRUSH,
}

static APP: AtomicPtr<App> = AtomicPtr::new(null_mut());

// UI-thread-owned state. Keep a raw pointer so reentrant Win32 calls never alias an &mut App.
unsafe fn app() -> *mut App {
    APP.load(Ordering::Relaxed)
}

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}
fn lo(l: isize) -> i32 {
    (l & 0xffff) as i16 as i32
}
fn hi(l: isize) -> i32 {
    ((l >> 16) & 0xffff) as i16 as i32
}
unsafe fn sc(v: i32) -> i32 {
    v * (*app()).dpi / 96
}
unsafe fn hinst() -> HINSTANCE {
    GetModuleHandleW(null())
}

unsafe fn get_text(h: HWND) -> String {
    let n = GetWindowTextLengthW(h);
    if n <= 0 {
        return String::new();
    }
    let mut b = vec![0u16; n as usize + 1];
    let c = GetWindowTextW(h, b.as_mut_ptr(), b.len() as i32);
    String::from_utf16_lossy(&b[..c.max(0) as usize])
}
unsafe fn set_text(h: HWND, s: &str) {
    SetWindowTextW(h, wide(s).as_ptr());
}

// ---------- geometry ----------

#[derive(Debug, PartialEq)]
enum Hit {
    Center,
    Wedge(usize),
    Outside,
}

/// Clockwise from 12 o'clock; wedge 0 is centered on the top.
fn hit(dx: f64, dy: f64, n: usize) -> Hit {
    let r = (dx * dx + dy * dy).sqrt();
    if r < CENTER_R {
        return Hit::Center;
    }
    if n == 0 || !(RING_IN..=RING_OUT).contains(&r) {
        return Hit::Outside;
    }
    let sector = std::f64::consts::TAU / n as f64;
    let a = (dx.atan2(-dy) + sector / 2.0).rem_euclid(std::f64::consts::TAU);
    Hit::Wedge(((a / sector) as usize).min(n - 1))
}

const ARC_STEPS: usize = 14;

/// Wedge outline: outer arc clockwise, then inner arc back. Corners are rounded with small quadratic curves.
fn wedge_points(i: usize, n: usize, cx: f64, cy: f64, k: f64) -> Vec<POINT> {
    let sector = std::f64::consts::TAU / n as f64;
    let mid = i as f64 * sector;
    let mut v: Vec<(f64, f64)> = Vec::new();
    for (r, rev) in [(RING_OUT, false), (RING_IN, true)] {
        // The angular inset keeps a constant GAP-wide channel between neighbours.
        let inset = if n > 1 { (GAP / 2.0) / r } else { 0.0 };
        let (a0, a1) = (mid - sector / 2.0 + inset, mid + sector / 2.0 - inset);
        for s in 0..=ARC_STEPS {
            let t = if rev { ARC_STEPS - s } else { s } as f64 / ARC_STEPS as f64;
            let a = a0 + (a1 - a0) * t;
            v.push((cx + r * k * a.sin(), cy - r * k * a.cos()));
        }
    }
    let corners = [0, ARC_STEPS, ARC_STEPS + 1, 2 * ARC_STEPS + 1];
    let len = v.len();
    let mut out = Vec::new();
    for (idx, &(x, y)) in v.iter().enumerate() {
        if !corners.contains(&idx) {
            out.push(POINT {
                x: x.round() as i32,
                y: y.round() as i32,
            });
            continue;
        }
        let (px, py) = v[(idx + len - 1) % len];
        let (nx, ny) = v[(idx + 1) % len];
        let (d1, d2) = (
            ((px - x).powi(2) + (py - y).powi(2)).sqrt(),
            ((nx - x).powi(2) + (ny - y).powi(2)).sqrt(),
        );
        let t = (CORNER_R * k).min(d1 / 2.0).min(d2 / 2.0);
        let (ax, ay, bx, by) = (
            x + (px - x) * t / d1,
            y + (py - y) * t / d1,
            x + (nx - x) * t / d2,
            y + (ny - y) * t / d2,
        );
        for s in 0..=6 {
            let u = s as f64 / 6.0;
            let (w0, w1, w2) = ((1.0 - u) * (1.0 - u), 2.0 * u * (1.0 - u), u * u);
            out.push(POINT {
                x: (w0 * ax + w1 * x + w2 * bx).round() as i32,
                y: (w0 * ay + w1 * y + w2 * by).round() as i32,
            });
        }
    }
    out
}

/// Ring priority per action (lower first); unlisted actions keep catalog order after these.
fn ring_rank(id: &str) -> u8 {
    match id {
        "convert" => 0,
        "crop" => 1,
        "resize" => 2,
        "frame" => 3,
        "compress" => 4,
        "copy_paths" => 10,
        "reveal" => 11,
        "create_zip" => 12,
        "extract_zip" => 13,
        "sha256" => 14,
        "duplicate" => 15,
        "terminal" => 16,
        "trim" => 20,
        "json_format" => 21,
        "count" => 22,
        "upper" => 23,
        "lower" => 24,
        "sort" => 25,
        "center_window" => 30,
        "always_top" => 31,
        "fit_window" => 32,
        "window_details" => 33,
        _ => 50,
    }
}

// ---------- context ----------

unsafe fn center_label() -> String {
    let a = app();
    if (*a).resolving {
        return "Reading selection…".into();
    }
    match (*a).paths.len() {
        0 => {}
        1 => {
            let p = &(&(*a).paths)[0];
            let name = p
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| p.display().to_string());
            return if p.is_dir() {
                format!("Folder: {name}")
            } else {
                name
            };
        }
        n => return format!("{n} items"),
    }
    if let Some(t) = &(*a).text {
        return format!("Clipboard text ({} chars)", t.chars().count());
    }
    if (*a).source != 0 {
        return "Active window".into();
    }
    "Nothing selected".into()
}

unsafe fn target_label() -> String {
    let a = app();
    if (*a).paths.len() > 1 {
        return format!(
            "Target: {} items, first {}",
            (*a).paths.len(),
            (&(*a).paths)[0].display()
        );
    }
    if (*a).paths.is_empty() && (*a).text.is_none() && (*a).source != 0 && !(*a).resolving {
        let t = get_text((*a).source as HWND);
        return format!(
            "Target: active window{}",
            if t.is_empty() {
                String::new()
            } else {
                format!(" — {t}")
            }
        );
    }
    format!("Target: {}", center_label())
}

unsafe fn rebuild_ring() {
    let a = app();
    (*a).ring.clear();
    (*a).more = 0;
    if !(*a).resolving {
        let has_text = (*a).text.is_some();
        let mut all: Vec<usize> = (0..(*a).actions.len())
            .filter(|&i| actions::eligible(&(&(*a).actions)[i], &(*a).paths, has_text))
            .collect();
        // Window actions need a captured window; drop them without one.
        if (*a).source == 0 || IsWindow((*a).source as HWND) == 0 {
            all.retain(|&i| !matches!((&(*a).actions)[i].input, InputKind::Window));
        }
        all.sort_by_key(|&i| ring_rank((&(*a).actions)[i].id));
        if all.len() > MAX_WEDGES {
            (*a).more = all.len() - (MAX_WEDGES - 1);
            all.truncate(MAX_WEDGES - 1);
            all.push(MORE);
        }
        (*a).ring = all;
    }
    (*a).sel = if (*a).ring.is_empty() { None } else { Some(0) };
}

unsafe fn sync_ui() {
    rebuild_ring();
    let a = app();
    if !(*a).dash.is_null() {
        set_text((*a).c_target, &target_label());
        refresh_list();
    }
    InvalidateRect((*a).pie, null(), 0);
}

unsafe fn set_paths(p: Vec<PathBuf>) {
    let a = app();
    (*a).paths = p;
    (*a).text = None;
    sync_ui();
}

unsafe fn status(s: &str) {
    let a = app();
    if !(*a).dash.is_null() {
        set_text((*a).c_status, s);
    }
}

fn need_label(k: &InputKind) -> &'static str {
    match k {
        InputKind::Files => "files or folders",
        InputKind::Images => "images",
        InputKind::Text => "text (use clipboard)",
        InputKind::Window => "a captured active window",
    }
}

// ---------- tray ----------

unsafe fn tray(msg: u32, tip: &str) {
    let a = app();
    let mut nid: NOTIFYICONDATAW = zeroed();
    nid.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
    nid.hWnd = (*a).main;
    nid.uID = 1;
    nid.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP;
    nid.uCallbackMessage = WM_APP_TRAY;
    nid.hIcon = LoadIconW(null_mut(), IDI_APPLICATION);
    for (i, c) in tip.encode_utf16().take(127).enumerate() {
        nid.szTip[i] = c;
    }
    Shell_NotifyIconW(msg, &nid);
}

unsafe fn tray_menu() {
    let a = app();
    let m = CreatePopupMenu();
    AppendMenuW(m, MF_STRING, 1, wide("Open PieAll").as_ptr());
    AppendMenuW(m, MF_STRING, 2, wide("Show action ring").as_ptr());
    AppendMenuW(m, if (*a).update_busy { MF_GRAYED } else { MF_STRING }, 4, wide(update_label()).as_ptr());
    AppendMenuW(m, MF_STRING, 3, wide("Exit").as_ptr());
    let mut pt: POINT = zeroed();
    GetCursorPos(&mut pt);
    SetForegroundWindow((*a).main);
    let cmd = TrackPopupMenu(
        m,
        TPM_RETURNCMD | TPM_RIGHTBUTTON,
        pt.x,
        pt.y,
        0,
        (*a).main,
        null(),
    );
    DestroyMenu(m);
    match cmd {
        1 => show_dashboard(),
        2 => show_ring(),
        3 => exit_app(),
        4 => update_action(),
        _ => {}
    }
}

unsafe fn exit_app() {
    let a = app();
    if (*a).busy > 0 || (*a).resolving || (*a).update_busy || !(*a).editor.is_null() {
        show_dashboard();
        status("Finish the current action, selection or update, and close the editor before exiting.");
        return;
    }
    DestroyWindow((*a).main);
}

// All update state belongs to the UI thread; workers deliver owned events only.
unsafe fn update_label() -> &'static str {
    let a = app();
    if (*a).update_busy { "Updating…" }
    else if (*a).update_ready.is_some() { "Install update…" }
    else if (*a).update_available.is_some() { "Download update" }
    else { "Check for updates" }
}

unsafe fn update_status(message: &str) {
    let a = app();
    set_text((*a).c_update_status, message);
    set_text((*a).c_update, update_label());
    EnableWindow((*a).c_update, i32::from(!(*a).update_busy));
}

fn post_update(main: usize, event: UpdateEvent) {
    let event = Box::into_raw(Box::new(event));
    unsafe {
        if PostMessageW(main as HWND, WM_APP_UPDATE, 0, event as isize) == 0 {
            drop(Box::from_raw(event));
        }
    }
}

unsafe fn check_update(manual: bool) {
    let a = app();
    if (*a).update_busy || (*a).update_ready.is_some() || (!manual && (*a).update_available.is_some()) { return; }
    if !updates::configured() {
        if manual { update_status("Updates require a signed installed release; see docs/releases.md."); }
        return;
    }
    (*a).update_busy = true;
    update_status("Checking for updates…");
    let main = (*a).main as usize;
    std::thread::spawn(move || post_update(main, UpdateEvent::Checked(updates::check())));
}

unsafe fn update_action() {
    let a = app();
    if (*a).update_busy { return; }
    if (*a).update_ready.is_some() {
        if (*a).busy > 0 || (*a).resolving || !(*a).editor.is_null() {
            show_dashboard();
            update_status("Finish the action or selection and close the editor before installing.");
            return;
        }
        // Reserve shutdown before opening the reentrant native confirmation dialog.
        (*a).update_busy = true;
        (*a).busy += 1;
        let version = (*a).update_ready.as_ref().unwrap().version().to_owned();
        let answer = MessageBoxW((*a).dash, wide(&format!("Install PieAll {version}? PieAll will close. The installer will let you reopen it when finished. Your files are preserved.")).as_ptr(), wide("Install PieAll update").as_ptr(), MB_YESNO | MB_ICONQUESTION | MB_DEFBUTTON2);
        (*a).busy -= 1;
        (*a).update_busy = false;
        if answer != IDYES { return; }
        let ready = (*a).update_ready.take().unwrap();
        match ready.install() {
            Ok(()) => { DestroyWindow((*a).main); }
            Err(e) => update_status(&format!("Install failed: {e}. Download again to retry.")),
        }
    } else if let Some(release) = (*a).update_available.clone() {
        (*a).update_busy = true;
        update_status("Downloading update…");
        let main = (*a).main as usize;
        std::thread::spawn(move || {
            let result = updates::download(release, |n| post_update(main, UpdateEvent::Progress(n)));
            post_update(main, UpdateEvent::Downloaded(result));
        });
    } else { check_update(true); }
}

unsafe fn on_update(event: UpdateEvent) {
    let a = app();
    match event {
        UpdateEvent::Progress(n) => update_status(&format!("Downloading update: {n}% (signature checked before install).")),
        UpdateEvent::Checked(result) => {
            (*a).update_busy = false;
            match result {
                Ok(release) => {
                    (*a).update_available = release;
                    if let Some(release) = &(*a).update_available {
                        update_status(&format!("PieAll {} is available. Choose Download update.", release.version));
                        tray(NIM_MODIFY, &format!("PieAll — {} available", release.version));
                    } else { update_status(&format!("PieAll {} is up to date.", env!("CARGO_PKG_VERSION"))); }
                }
                Err(e) => update_status(&format!("Update check failed: {e}. Check again to retry.")),
            }
        }
        UpdateEvent::Downloaded(result) => {
            (*a).update_busy = false;
            match result {
                Ok(download) => {
                    (*a).update_ready = Some(download);
                    update_status("Update verified and ready. Choose Install update when convenient.");
                }
                Err(e) => update_status(&format!("Download failed: {e}. Download again to retry.")),
            }
        }
    }
}

// ---------- pie ----------

unsafe fn pie_px() -> i32 {
    sc(PIE_DIP)
}

unsafe fn place_pie() {
    let a = app();
    let mut pt: POINT = zeroed();
    GetCursorPos(&mut pt);
    let sz = pie_px();
    let (x, y) = clamp_to_work(pt.x - sz / 2, pt.y - sz / 2, sz, sz);
    SetWindowPos(
        (*a).pie,
        HWND_TOPMOST,
        x,
        y,
        sz,
        sz,
        SWP_NOACTIVATE | SWP_SHOWWINDOW,
    );
    InvalidateRect((*a).pie, null(), 0);
}

unsafe fn clamp_to_work(x: i32, y: i32, w: i32, h: i32) -> (i32, i32) {
    let mut mi: MONITORINFO = zeroed();
    mi.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
    let pt = POINT {
        x: x + w / 2,
        y: y + h / 2,
    };
    let mon = MonitorFromPoint(pt, MONITOR_DEFAULTTONEAREST);
    if GetMonitorInfoW(mon, &mut mi) == 0 {
        return (x, y);
    }
    let r = mi.rcWork;
    (
        x.min(r.right - w).max(r.left),
        y.min(r.bottom - h).max(r.top),
    )
}

unsafe fn show_ring() {
    let a = app();
    (*a).gen += 1;
    (*a).resolving = false;
    (*a).pie_active = false;
    (*a).restore_to = 0;
    KillTimer((*a).main, TIMER_CAPTURE);
    rebuild_ring();
    place_pie();
    SetForegroundWindow((*a).pie);
}

unsafe fn hide_pie() {
    let a = app();
    (*a).pie_active = false;
    (*a).gen += 1; // invalidates an in-flight capture
    KillTimer((*a).main, TIMER_CAPTURE);
    if (*a).resolving {
        (*a).resolving = false;
    }
    ShowWindow((*a).pie, SW_HIDE);
}

/// Close the ring without acting and hand focus back to the window it was opened over.
unsafe fn cancel_pie() {
    let back = (*app()).restore_to as HWND;
    hide_pie();
    if !back.is_null() && IsWindow(back) != 0 {
        SetForegroundWindow(back);
    }
}

unsafe fn on_hotkey() {
    let a = app();
    if IsWindowVisible((*a).pie) != 0 {
        cancel_pie();
        return;
    }
    let fg = GetForegroundWindow();
    // Capture the source window before any of our windows take focus.
    if !fg.is_null() && GetWindowThreadProcessId(fg, null_mut()) == GetCurrentThreadId() {
        show_ring(); // invoked from our own window: keep the context the user built explicitly
        return;
    }
    let src = fg as usize;
    (*a).source = src;
    GetWindowThreadProcessId(src as HWND, &mut (*a).source_pid);
    (*a).restore_to = src;
    (*a).paths.clear();
    (*a).text = None;
    (*a).resolving = true;
    (*a).gen += 1;
    let gen = (*a).gen;
    let main = (*a).main as usize;
    rebuild_ring();
    place_pie();
    SetTimer((*a).main, TIMER_CAPTURE, CAPTURE_MS, None);
    std::thread::spawn(move || {
        // Query the window captured at hotkey time, never the foreground at worker time.
        let r = std::panic::catch_unwind(|| crate::platform::capture_selection_from(src))
            .unwrap_or_else(|_| Err("selection capture panicked".to_string()));
        let b = Box::into_raw(Box::new((gen, r)));
        unsafe {
            if PostMessageW(main as HWND, WM_APP_CAPTURED, 0, b as isize) == 0 {
                drop(Box::from_raw(b));
            }
        }
    });
}

unsafe fn on_captured(b: (u64, Result<Vec<PathBuf>, String>)) {
    let a = app();
    let (gen, r) = b;
    if gen != (*a).gen || !(*a).resolving {
        return;
    }
    finish_capture(r);
}

/// Shared by the worker reply and the 1.5 s deadline; both end the "resolving" state exactly once.
unsafe fn finish_capture(r: Result<Vec<PathBuf>, String>) {
    let a = app();
    KillTimer((*a).main, TIMER_CAPTURE);
    (*a).resolving = false;
    let mut msg = String::new();
    match r {
        Ok(v) if !v.is_empty() => (*a).paths = v,
        Ok(_) => msg = "No file selection was found in the source window.".into(),
        Err(e) => msg = format!("Selection capture failed: {e}"),
    }
    sync_ui();
    if msg.is_empty() {
        (*a).pie_active = false;
        SetForegroundWindow((*a).pie);
        return;
    }
    status(&format!(
        "{msg} Drop files onto this window, choose Open files, or use clipboard text."
    ));
    if (*app()).ring.is_empty() {
        hide_pie();
        show_dashboard();
    } else {
        SetForegroundWindow((*app()).pie);
    }
}

unsafe fn paint_pie(hwnd: HWND) {
    let a = app();
    let mut ps: PAINTSTRUCT = zeroed();
    let dc = BeginPaint(hwnd, &mut ps);
    let sz = pie_px();
    let k = sz as f64 / PIE_DIP as f64;
    let c = sz as f64 / 2.0;
    let mem = CreateCompatibleDC(dc);
    let bmp = CreateCompatibleBitmap(dc, sz, sz);
    let old_bmp = SelectObject(mem, bmp);
    let full = RECT {
        left: 0,
        top: 0,
        right: sz,
        bottom: sz,
    };
    let fill = |c: u32| {
        let b = CreateSolidBrush(c);
        FillRect(mem, &full, b);
        DeleteObject(b);
    };
    fill(BASE);
    SetBkMode(mem, TRANSPARENT as i32);

    // Thin outer rim.
    let rim_pen = CreatePen(PS_SOLID, 2, RIM);
    let (op, ob) = (
        SelectObject(mem, rim_pen),
        SelectObject(mem, GetStockObject(NULL_BRUSH)),
    );
    Ellipse(mem, 1, 1, sz - 1, sz - 1);
    SelectObject(mem, op);
    SelectObject(mem, ob);
    DeleteObject(rim_pen);

    // Wedges.
    let n = (*a).ring.len();
    let edge = CreatePen(PS_SOLID, 1, WEDGE_EDGE);
    let old_pen = SelectObject(mem, edge);
    for i in 0..n {
        let sel = (*a).sel == Some(i);
        let br = CreateSolidBrush(if sel { SELECTED } else { WEDGE });
        let old_br = SelectObject(mem, br);
        let pts = wedge_points(i, n, c, c, k);
        Polygon(mem, pts.as_ptr(), pts.len() as i32);
        SelectObject(mem, old_br);
        DeleteObject(br);
        let mid = i as f64 * std::f64::consts::TAU / n as f64;
        let rm = (RING_IN + RING_OUT) / 2.0 * k;
        let (lx, ly) = ((c + rm * mid.sin()) as i32, (c - rm * mid.cos()) as i32);
        SelectObject(mem, (*a).font);
        let title = if (&(*a).ring)[i] == MORE {
            "More…"
        } else {
            (&(*a).actions)[(&(*a).ring)[i]].title
        };
        text_at(
            mem,
            title,
            lx,
            ly,
            (84.0 * k) as i32,
            (54.0 * k) as i32,
            if sel { ON_SELECTED } else { INK },
        );
    }

    // Quiet center.
    let cp = CreatePen(PS_SOLID, 1, RIM);
    let cb = CreateSolidBrush(CENTER_FILL);
    SelectObject(mem, cp);
    let old_cb = SelectObject(mem, cb);
    let r = (CENTER_R * k) as i32;
    Ellipse(mem, c as i32 - r, c as i32 - r, c as i32 + r, c as i32 + r);
    SelectObject(mem, old_cb);
    SelectObject(mem, old_pen);
    DeleteObject(cb);
    DeleteObject(cp);
    DeleteObject(edge);
    SelectObject(mem, (*a).font_bold);
    let (cx, cy) = (c as i32, c as i32);
    text_at(
        mem,
        &center_label(),
        cx,
        cy - (8.0 * k) as i32,
        (84.0 * k) as i32,
        (46.0 * k) as i32,
        INK,
    );
    SelectObject(mem, (*a).font_small);
    let hint = if (*a).resolving {
        "Please wait".to_string()
    } else if (*a).ring.is_empty() {
        "Drop files here".to_string()
    } else if (*a).more > 0 {
        format!("+{} in dashboard", (*a).more)
    } else {
        "Esc closes".to_string()
    };
    text_at(
        mem,
        &hint,
        cx,
        cy + (30.0 * k) as i32,
        (84.0 * k) as i32,
        (18.0 * k) as i32,
        rgb(0x6B, 0x5F, 0x50),
    );

    BitBlt(dc, 0, 0, sz, sz, mem, 0, 0, SRCCOPY);
    SelectObject(mem, old_bmp);
    DeleteObject(bmp);
    DeleteDC(mem);
    EndPaint(hwnd, &ps);
}

unsafe fn text_at(dc: HDC, s: &str, cx: i32, cy: i32, maxw: i32, maxh: i32, color: u32) {
    let t = wide(s);
    let mut r = RECT {
        left: 0,
        top: 0,
        right: maxw,
        bottom: 0,
    };
    DrawTextW(
        dc,
        t.as_ptr(),
        -1,
        &mut r,
        DT_CALCRECT | DT_WORDBREAK | DT_CENTER | DT_NOPREFIX,
    );
    let h = r.bottom.min(maxh);
    let mut r = RECT {
        left: cx - maxw / 2,
        top: cy - h / 2,
        right: cx + maxw / 2,
        bottom: cy + h / 2,
    };
    SetTextColor(dc, color);
    DrawTextW(
        dc,
        t.as_ptr(),
        -1,
        &mut r,
        DT_WORDBREAK | DT_CENTER | DT_NOPREFIX | DT_END_ELLIPSIS,
    );
}

unsafe fn pie_hit(lp: isize) -> Hit {
    let c = pie_px() as f64 / 2.0;
    let k = pie_px() as f64 / PIE_DIP as f64;
    hit(
        (lo(lp) as f64 - c) / k,
        (hi(lp) as f64 - c) / k,
        (*app()).ring.len(),
    )
}

unsafe fn move_sel(delta: isize) {
    let a = app();
    let n = (*a).ring.len() as isize;
    if n == 0 {
        return;
    }
    (*a).sel = Some((((*a).sel.unwrap_or(0) as isize + delta).rem_euclid(n)) as usize);
    InvalidateRect((*a).pie, null(), 0);
}

unsafe fn activate_wedge(i: usize) {
    let a = app();
    if (*a).resolving || i >= (*a).ring.len() {
        return;
    }
    let idx = (&(*a).ring)[i];
    hide_pie();
    if idx == MORE {
        show_dashboard();
    } else {
        start_action(idx);
    }
}

unsafe extern "system" fn pie_proc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    let a = app();
    match msg {
        WM_PAINT => {
            paint_pie(hwnd);
            0
        }
        WM_ERASEBKGND => 1,
        WM_MOUSEACTIVATE => MA_ACTIVATE as isize,
        WM_ACTIVATE => {
            if (wp & 0xffff) == 0 {
                if (*a).pie_active {
                    hide_pie();
                }
            } else {
                (*a).pie_active = true;
                SetFocus(hwnd);
            }
            0
        }
        WM_MOUSEMOVE => {
            if let Hit::Wedge(i) = pie_hit(lp) {
                if (*a).sel != Some(i) {
                    (*a).sel = Some(i);
                    InvalidateRect(hwnd, null(), 0);
                }
            }
            0
        }
        WM_LBUTTONUP => {
            match pie_hit(lp) {
                Hit::Wedge(i) => activate_wedge(i),
                Hit::Center => cancel_pie(),
                Hit::Outside => {}
            }
            0
        }
        WM_KEYDOWN => {
            match wp as u16 {
                VK_ESCAPE => cancel_pie(),
                VK_LEFT | VK_UP => move_sel(-1),
                VK_RIGHT | VK_DOWN | VK_TAB => move_sel(1),
                VK_RETURN | VK_SPACE => {
                    if let Some(i) = (*a).sel {
                        activate_wedge(i)
                    }
                }
                v @ 0x31..=0x36 => activate_wedge((v - 0x31) as usize),
                _ => {}
            }
            0
        }
        WM_DROPFILES => {
            let p = drop_paths(wp as HDROP);
            if !p.is_empty() {
                (*a).resolving = false;
                (*a).gen += 1;
                set_paths(p);
            }
            0
        }
        _ => DefWindowProcW(hwnd, msg, wp, lp),
    }
}

unsafe fn drop_paths(h: HDROP) -> Vec<PathBuf> {
    let n = DragQueryFileW(h, 0xFFFF_FFFF, null_mut(), 0);
    let mut v = Vec::new();
    for i in 0..n {
        let len = DragQueryFileW(h, i, null_mut(), 0) as usize;
        let mut buf = vec![0u16; len + 1];
        DragQueryFileW(h, i, buf.as_mut_ptr(), buf.len() as u32);
        v.push(PathBuf::from(std::ffi::OsString::from_wide(&buf[..len])));
    }
    DragFinish(h);
    v
}

// ---------- actions ----------

unsafe fn start_action(idx: usize) {
    let a = app();
    if (*a).resolving {
        status("Still reading the selection; try again in a moment.");
        return;
    }
    if (*a).busy > 0 {
        status("A job is already running. Wait for it to finish.");
        return;
    }
    let act = &(&(*a).actions)[idx];
    if !actions::eligible(act, &(*a).paths, (*a).text.is_some()) {
        let msg = format!("{} needs {}.", act.title, need_label(&act.input));
        status(&msg);
        return;
    }
    if matches!(act.input, InputKind::Window)
        && ((*a).source == 0 || IsWindow((*a).source as HWND) == 0)
    {
        status("No active window was captured. Press Ctrl+` while the target window is focused.");
        return;
    }
    if act.fields.is_empty() {
        run_action(idx, BTreeMap::new());
    } else {
        open_editor(idx);
    }
}

unsafe fn run_action(idx: usize, mut params: BTreeMap<String, String>) {
    let a = app();
    if (*a).busy > 0 {
        return; // one job at a time; callers check first so this is only a backstop
    }
    let act = &(&(*a).actions)[idx];
    let (id, title) = (act.id.to_string(), act.title.to_string());
    if matches!(act.input, InputKind::Window) {
        params.insert("source_hwnd".into(), (*a).source.to_string());
        params.insert("source_pid".into(), (*a).source_pid.to_string());
    }
    let paths = (*a).paths.clone();
    let text = (*a).text.clone().unwrap_or_default();
    let main = (*a).main as usize;
    (*a).busy += 1;
    status(&format!("Running {title}…"));
    tray(NIM_MODIFY, &format!("PieAll — running {title}"));
    std::thread::spawn(move || {
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            actions::execute(&id, &paths, &text, &params)
        }))
        .unwrap_or_else(|_| Err("action panicked".to_string()));
        let b = Box::into_raw(Box::new(Done { title, result: r }));
        unsafe {
            if PostMessageW(main as HWND, WM_APP_RESULT, 0, b as isize) == 0 {
                drop(Box::from_raw(b));
            }
        }
    });
}

unsafe fn on_result(d: Done) {
    let a = app();
    (*a).busy = (*a).busy.saturating_sub(1);
    let (head, body) = match d.result {
        Ok(s) => (format!("{} — done", d.title), s),
        Err(e) => (format!("{} — failed", d.title), e),
    };
    status(&format!("{head}."));
    tray(
        NIM_MODIFY,
        if (*a).busy > 0 {
            "PieAll — working"
        } else {
            "PieAll"
        },
    );
    show_result(&head, &body);
}

// ---------- shared window helpers ----------

unsafe fn register(name: &str, proc_: WNDPROC, bg: HBRUSH, cursor: bool) {
    let mut wc: WNDCLASSW = zeroed();
    wc.lpfnWndProc = proc_;
    wc.hInstance = hinst();
    wc.hbrBackground = bg;
    wc.hCursor = if cursor {
        LoadCursorW(null_mut(), IDC_ARROW)
    } else {
        null_mut()
    };
    wc.hIcon = LoadIconW(null_mut(), IDI_APPLICATION);
    let n = wide(name);
    wc.lpszClassName = n.as_ptr();
    RegisterClassW(&wc);
}

#[allow(clippy::too_many_arguments)]
unsafe fn child(
    parent: HWND,
    class: &str,
    text: &str,
    ex: u32,
    style: u32,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    id: i32,
) -> HWND {
    let hw = CreateWindowExW(
        ex,
        wide(class).as_ptr(),
        wide(text).as_ptr(),
        WS_CHILD | WS_VISIBLE | style,
        sc(x),
        sc(y),
        sc(w),
        sc(h),
        parent,
        id as usize as HMENU,
        hinst(),
        null(),
    );
    SendMessageW(hw, WM_SETFONT, (*app()).font as usize, 1);
    hw
}

unsafe fn button(parent: HWND, text: &str, x: i32, y: i32, w: i32, id: i32, default: bool) -> HWND {
    let st = (if default {
        BS_DEFPUSHBUTTON
    } else {
        BS_PUSHBUTTON
    }) as u32;
    child(parent, "BUTTON", text, 0, st | WS_TABSTOP, x, y, w, 32, id)
}

unsafe fn outer_size(cw: i32, ch: i32, style: u32, ex: u32) -> (i32, i32) {
    let mut r = RECT {
        left: 0,
        top: 0,
        right: sc(cw),
        bottom: sc(ch),
    };
    AdjustWindowRectEx(&mut r, style, 0, ex);
    (r.right - r.left, r.bottom - r.top)
}

unsafe fn ctl_color(wp: WPARAM) -> LRESULT {
    let dc = wp as HDC;
    SetTextColor(dc, INK);
    SetBkColor(dc, BASE);
    (*app()).bg as isize
}

// ---------- dashboard ----------

unsafe fn create_dashboard() {
    let a = app();
    let style = WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX;
    let ex = WS_EX_CONTROLPARENT | WS_EX_ACCEPTFILES;
    let (w, h) = outer_size(600, 570, style, ex);
    let hw = CreateWindowExW(
        ex,
        wide(C_DASH).as_ptr(),
        wide("PieAll").as_ptr(),
        style,
        CW_USEDEFAULT,
        CW_USEDEFAULT,
        w,
        h,
        null_mut(),
        null_mut(),
        hinst(),
        null(),
    );
    (*a).dash = hw;
    (*a).c_target = child(
        hw,
        "STATIC",
        "",
        0,
        0xCu32 | 0x4000u32, /* SS_LEFTNOWORDWRAP | SS_ENDELLIPSIS */
        16,
        14,
        568,
        22,
        0,
    );
    SendMessageW((*a).c_target, WM_SETFONT, (*a).font_bold as usize, 1);
    child(hw, "STATIC", "Search actions", 0, 0, 16, 40, 568, 18, 0);
    (*a).c_search = child(
        hw,
        "EDIT",
        "",
        WS_EX_CLIENTEDGE,
        ES_AUTOHSCROLL as u32 | WS_TABSTOP,
        16,
        60,
        568,
        28,
        ID_SEARCH,
    );
    SendMessageW(
        (*a).c_search,
        0x1501,
        1,
        wide("Search all actions").as_ptr() as isize,
    ); // EM_SETCUEBANNER
    child(
        hw,
        "STATIC",
        "All actions (accessible list, same as the ring)",
        0,
        0,
        16,
        94,
        568,
        18,
        0,
    );
    (*a).c_list = child(
        hw,
        "LISTBOX",
        "",
        WS_EX_CLIENTEDGE,
        (LBS_NOTIFY | LBS_NOINTEGRALHEIGHT) as u32 | WS_VSCROLL | WS_TABSTOP,
        16,
        114,
        568,
        262,
        ID_LIST,
    );
    button(hw, "Open files…", 16, 388, 120, ID_OPEN, false);
    button(hw, "Use clipboard text", 144, 388, 160, ID_CLIP, false);
    button(hw, "Show ring", 312, 388, 100, ID_RING, false);
    button(hw, "Run", 484, 388, 100, IDOK_, true);
    (*a).c_status = child(
        hw,
        "STATIC",
        "Press Ctrl+` over an Explorer selection, or drop files here.",
        0,
        0,
        16,
        432,
        568,
        44,
        0,
    );
    (*a).c_update = child(hw, "BUTTON", "Check for updates", 0, WS_TABSTOP | BS_PUSHBUTTON as u32, 16, 490, 160, 28, ID_UPDATE);
    (*a).c_update_status = child(hw, "STATIC", &format!("PieAll {} — updates use signed GitHub releases.", env!("CARGO_PKG_VERSION")), 0, 0, 188, 490, 396, 64, 0);
}

unsafe fn show_dashboard() {
    let a = app();
    sync_ui();
    ShowWindow((*a).dash, SW_SHOWNORMAL);
    SetForegroundWindow((*a).dash);
    SetFocus((*a).c_search);
}

unsafe fn refresh_list() {
    let a = app();
    let q = get_text((*a).c_search).to_lowercase();
    SendMessageW((*a).c_list, LB_RESETCONTENT, 0, 0);
    (*a).list_map.clear();
    let has_text = (*a).text.is_some();
    for i in 0..(*a).actions.len() {
        let act = &(&(*a).actions)[i];
        if !q.is_empty()
            && !format!("{} {} {}", act.title, act.group, act.id)
                .to_lowercase()
                .contains(&q)
        {
            continue;
        }
        let ok = actions::eligible(act, &(*a).paths, has_text);
        let line = format!(
            "{}   ·   {}{}",
            act.title,
            act.group,
            if ok {
                String::new()
            } else {
                format!("   (needs {})", need_label(&act.input))
            }
        );
        SendMessageW((*a).c_list, LB_ADDSTRING, 0, wide(&line).as_ptr() as isize);
        (*a).list_map.push(i);
    }
    if !(*a).list_map.is_empty() {
        SendMessageW((*a).c_list, LB_SETCURSEL, 0, 0);
    }
}

unsafe fn run_selected() {
    let a = app();
    let s = SendMessageW((*a).c_list, LB_GETCURSEL, 0, 0);
    if s >= 0 {
        if let Some(&idx) = (*a).list_map.as_slice().get(s as usize) {
            start_action(idx);
        }
    }
}

unsafe extern "system" fn dash_proc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    match msg {
        WM_COMMAND => {
            let (id, code) = ((wp & 0xffff) as i32, ((wp >> 16) & 0xffff) as u32);
            match id {
                ID_SEARCH if code == EN_CHANGE => refresh_list(),
                ID_LIST if code == LBN_DBLCLK => run_selected(),
                ID_OPEN => match crate::platform::pick_files() {
                    Ok(v) if !v.is_empty() => {
                        set_paths(v);
                        status("Files loaded from the picker.");
                    }
                    Ok(_) => status("No files chosen."),
                    Err(e) => status(&format!("Open files failed: {e}")),
                },
                ID_CLIP => match crate::platform::clipboard_text() {
                    Ok(s) if !s.is_empty() => {
                        let a = app();
                        (*a).paths.clear();
                        (*a).text = Some(s);
                        sync_ui();
                        status("Clipboard text loaded (read because you asked).");
                    }
                    Ok(_) => status("The clipboard has no text."),
                    Err(e) => status(&format!("Clipboard read failed: {e}")),
                },
                ID_RING => show_ring(),
                ID_UPDATE => update_action(),
                IDOK_ => run_selected(),
                IDCANCEL_ => {
                    ShowWindow(hwnd, SW_HIDE);
                }
                _ => {}
            }
            0
        }
        WM_DROPFILES => {
            let p = drop_paths(wp as HDROP);
            if !p.is_empty() {
                set_paths(p);
                status("Dropped files loaded.");
            }
            0
        }
        WM_CTLCOLORSTATIC | WM_CTLCOLOREDIT | WM_CTLCOLORLISTBOX => ctl_color(wp),
        WM_CLOSE => {
            ShowWindow(hwnd, SW_HIDE);
            0
        }
        _ => DefWindowProcW(hwnd, msg, wp, lp),
    }
}

// ---------- parameter editor ----------

fn is_numeric_key(k: &str) -> bool {
    matches!(
        k,
        "x" | "y" | "width" | "height" | "padding" | "corners" | "quality"
    )
}

unsafe fn field_text(key: &str) -> Option<String> {
    let (_, h) = *(*app()).edit_fields.iter().find(|(k, _)| *k == key)?;
    Some(get_text(h))
}

unsafe fn field_num(key: &str) -> Option<u32> {
    field_text(key)?.trim().parse().ok()
}

fn parse_hex(s: &str) -> Option<u32> {
    let s = s.trim().trim_start_matches('#');
    if s.len() != 6 {
        return None;
    }
    let v = u32::from_str_radix(s, 16).ok()?;
    Some(rgb(v >> 16, (v >> 8) & 0xff, v & 0xff))
}

unsafe fn open_editor(idx: usize) {
    if !(*app()).editor.is_null() {
        DestroyWindow((*app()).editor);
    }
    let a = app();
    let fields: Vec<(&'static str, &'static str, &'static str)> = (&(*a).actions)[idx]
        .fields
        .iter()
        .map(|f| (f.key, f.label, f.default))
        .collect();
    let title = (&(*a).actions)[idx].title;
    let n = fields.len() as i32;
    let want_preview =
        matches!((&(*a).actions)[idx].input, InputKind::Images) && !(*a).paths.is_empty();
    let top = if want_preview {
        16 + PREVIEW_H + 10
    } else {
        16
    };
    let (cw, ch) = (380, top + n * 54 + 8 + 32 + 16);
    let style = WS_POPUP | WS_CAPTION | WS_SYSMENU;
    let ex = WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_CONTROLPARENT;
    let (w, h) = outer_size(cw, ch, style, ex);
    let mut pt: POINT = zeroed();
    GetCursorPos(&mut pt);
    let (x, y) = clamp_to_work(pt.x - w / 2, pt.y - h / 2, w, h);
    let hw = CreateWindowExW(
        ex,
        wide(C_EDIT).as_ptr(),
        wide(title).as_ptr(),
        style,
        x,
        y,
        w,
        h,
        null_mut(),
        null_mut(),
        hinst(),
        null(),
    );
    (*a).editor = hw;
    (*a).edit_action = idx;
    (*a).edit_fields.clear();
    (*a).preview = None;
    (*a).preview_gen += 1;
    if want_preview {
        (*a).preview_note = "Loading preview…".into();
        (*a).preview_wnd = child(
            hw,
            C_PREVIEW,
            "Image preview",
            0,
            0,
            16,
            16,
            348,
            PREVIEW_H,
            0,
        );
        let (gen, path, main) = (
            (*a).preview_gen,
            (&(*a).paths)[0].clone(),
            (*a).main as usize,
        );
        std::thread::spawn(move || {
            let r = std::panic::catch_unwind(|| load_thumb(&path))
                .unwrap_or_else(|_| Err("preview failed".to_string()));
            let b = Box::into_raw(Box::new((gen, r)));
            unsafe {
                if PostMessageW(main as HWND, WM_APP_PREVIEW, 0, b as isize) == 0 {
                    drop(Box::from_raw(b));
                }
            }
        });
    }
    // Each label is created directly before its edit so Windows pairs them as the control's accessible name.
    for (i, (key, label, def)) in fields.iter().enumerate() {
        let y = top + i as i32 * 54;
        child(hw, "STATIC", label, 0, 0, 16, y, 348, 18, 0);
        let num = if is_numeric_key(key) {
            ES_NUMBER as u32
        } else {
            0
        };
        let e = child(
            hw,
            "EDIT",
            def,
            WS_EX_CLIENTEDGE,
            ES_AUTOHSCROLL as u32 | num | WS_TABSTOP,
            16,
            y + 20,
            348,
            26,
            200 + i as i32,
        );
        (*a).edit_fields.push((key, e));
    }
    let by = top + n * 54 + 8;
    button(hw, "Run", cw - 16 - 100 - 8 - 90, by, 90, IDOK_, true);
    button(hw, "Cancel", cw - 16 - 100, by, 100, IDCANCEL_, false);
    ShowWindow(hw, SW_SHOW);
    SetForegroundWindow(hw);
    if let Some((_, first)) = (*a).edit_fields.first() {
        SetFocus(*first);
        SendMessageW(*first, 0xB1 /* EM_SETSEL */, 0, -1);
    }
}

/// Worker-thread thumbnail: size and alloc limits keep a hostile or huge file from exhausting memory.
fn load_thumb(p: &std::path::Path) -> Result<Preview, String> {
    let md = std::fs::metadata(p).map_err(|e| e.to_string())?;
    if md.len() > 64 * 1024 * 1024 {
        return Err("File too large to preview".into());
    }
    let open = || {
        image::ImageReader::open(p)
            .and_then(|r| r.with_guessed_format())
            .map_err(|e| e.to_string())
    };
    let (ow, oh) = open()?.into_dimensions().map_err(|e| e.to_string())?;
    if ow as u64 * oh as u64 > 50_000_000 {
        return Err("Image too large to preview".into());
    }
    let mut r = open()?;
    let mut lim = image::Limits::default();
    lim.max_image_width = Some(16384);
    lim.max_image_height = Some(16384);
    lim.max_alloc = Some(256 * 1024 * 1024);
    r.limits(lim);
    use image::ImageDecoder;
    let mut decoder = open()?.into_decoder().map_err(|e| e.to_string())?;
    let orientation = decoder.orientation().map_err(|e| e.to_string())?;
    drop(decoder);
    let mut decoded = r.decode().map_err(|e| e.to_string())?;
    decoded.apply_orientation(orientation);
    let (ow, oh) = (decoded.width(), decoded.height());
    let t = decoded.thumbnail(256, 256).to_rgba8();
    let (tw, th) = (t.width() as i32, t.height() as i32);
    let ivory = [0xFBu32, 0xF6, 0xEC];
    let (mut px, mut faded) = (Vec::new(), Vec::new());
    for q in t.pixels() {
        let al = q[3] as u32;
        for ch in [2usize, 1, 0] {
            let c = (q[ch] as u32 * al + ivory[ch] * (255 - al)) / 255;
            px.push(c as u8);
            faded.push(((c * 35 + 0xF3 * 65) / 100) as u8);
        }
        px.push(0);
        faded.push(0);
    }
    Ok(Preview {
        ow,
        oh,
        tw,
        th,
        px,
        faded,
    })
}

unsafe fn on_preview(b: (u64, Result<Preview, String>)) {
    let a = app();
    let (gen, r) = b;
    if gen != (*a).preview_gen || (*a).editor.is_null() {
        return; // editor closed or reopened since this thumbnail was requested
    }
    match r {
        Ok(p) => (*a).preview = Some(p),
        Err(e) => (*a).preview_note = format!("No preview: {e}"),
    }
    InvalidateRect((*a).preview_wnd, null(), 0);
}

unsafe fn blit(dc: HDC, d: RECT, px: &[u8], tw: i32, th: i32, src: RECT) {
    let mut bi: BITMAPINFO = zeroed();
    bi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
    bi.bmiHeader.biWidth = tw;
    bi.bmiHeader.biHeight = -th; // top-down
    bi.bmiHeader.biPlanes = 1;
    bi.bmiHeader.biBitCount = 32;
    let (dw, dh, sw, sh) = (
        d.right - d.left,
        d.bottom - d.top,
        src.right - src.left,
        src.bottom - src.top,
    );
    if dw > 0 && dh > 0 && sw > 0 && sh > 0 {
        StretchDIBits(
            dc,
            d.left,
            d.top,
            dw,
            dh,
            src.left,
            src.top,
            sw,
            sh,
            px.as_ptr().cast(),
            &bi,
            DIB_RGB_COLORS,
            SRCCOPY,
        );
    }
}

unsafe fn outline(dc: HDC, r: RECT, color: u32, w: i32) {
    let pen = CreatePen(PS_SOLID, w, color);
    let (op, ob) = (
        SelectObject(dc, pen),
        SelectObject(dc, GetStockObject(NULL_BRUSH)),
    );
    Rectangle(dc, r.left, r.top, r.right, r.bottom);
    SelectObject(dc, op);
    SelectObject(dc, ob);
    DeleteObject(pen);
}

unsafe fn paint_preview(hwnd: HWND) {
    let a = app();
    let mut ps: PAINTSTRUCT = zeroed();
    let dc = BeginPaint(hwnd, &mut ps);
    let mut rc: RECT = zeroed();
    GetClientRect(hwnd, &mut rc);
    let (w, h) = (rc.right, rc.bottom);
    let mem = CreateCompatibleDC(dc);
    let bmp = CreateCompatibleBitmap(dc, w, h);
    let old = SelectObject(mem, bmp);
    let b = CreateSolidBrush(CENTER_FILL);
    FillRect(mem, &rc, b);
    DeleteObject(b);
    SetBkMode(mem, TRANSPARENT as i32);
    SelectObject(mem, (*a).font_small);
    let cap_h = sc(18);
    let (bw, bh) = (w - 2 * sc(6), h - cap_h - sc(6));
    let mut caption = (*a).preview_note.clone();
    if let Some(p) = &(*a).preview {
        let id = (&(*a).actions)[(*a).edit_action].id;
        let (ow, oh) = (p.ow as f64, p.oh as f64);
        caption = format!("{} × {} px", p.ow, p.oh);
        let num = |k: &str, d: u32| field_num(k).unwrap_or(d) as f64;
        // Canvas size in source pixels: frame adds padding around the image.
        let pad = if id == "frame" {
            num("padding", 64).min(2048.0)
        } else {
            0.0
        };
        let (cwid, chgt) = (ow + 2.0 * pad, oh + 2.0 * pad);
        let s = (bw as f64 / cwid).min(bh as f64 / chgt);
        let (cvw, cvh) = ((cwid * s) as i32, (chgt * s) as i32);
        let (cx0, cy0) = ((w - cvw) / 2, sc(6) + (bh - cvh) / 2);
        let (iw, ih) = ((ow * s) as i32, (oh * s) as i32);
        let (ix, iy) = (cx0 + (pad * s) as i32, cy0 + (pad * s) as i32);
        let whole = RECT {
            left: 0,
            top: 0,
            right: p.tw,
            bottom: p.th,
        };
        let img = RECT {
            left: ix,
            top: iy,
            right: ix + iw,
            bottom: iy + ih,
        };
        SetStretchBltMode(mem, HALFTONE);
        match id {
            "crop" => {
                blit(mem, img, &p.faded, p.tw, p.th, whole);
                let (x, y, cw, chh) = (
                    num("x", 0),
                    num("y", 0),
                    num("width", 512),
                    num("height", 512),
                );
                if cw > 0.0 && chh > 0.0 && x + cw <= ow && y + chh <= oh {
                    let d = RECT {
                        left: ix + (x * s) as i32,
                        top: iy + (y * s) as i32,
                        right: ix + ((x + cw) * s) as i32,
                        bottom: iy + ((y + chh) * s) as i32,
                    };
                    let (kx, ky) = (p.tw as f64 / ow, p.th as f64 / oh);
                    let src = RECT {
                        left: (x * kx) as i32,
                        top: (y * ky) as i32,
                        right: (((x + cw) * kx).ceil() as i32).min(p.tw),
                        bottom: (((y + chh) * ky).ceil() as i32).min(p.th),
                    };
                    blit(mem, d, &p.px, p.tw, p.th, src);
                    outline(mem, d, SELECTED, 2);
                    caption = format!(
                        "Crop {} × {} at {}, {} of {} × {}",
                        cw, chh, x, y, p.ow, p.oh
                    );
                } else {
                    caption = "Crop must be positive and inside the image".into();
                }
            }
            "frame" => {
                let bg = field_text("background").and_then(|t| parse_hex(&t));
                let cv = RECT {
                    left: cx0,
                    top: cy0,
                    right: cx0 + cvw,
                    bottom: cy0 + cvh,
                };
                let fb = CreateSolidBrush(bg.unwrap_or(rgb(0xC8, 0xC8, 0xC8)));
                FillRect(mem, &cv, fb);
                DeleteObject(fb);
                outline(mem, cv, RIM, 1);
                let r = (num("corners", 24).min(ow / 2.0).min(oh / 2.0) * s) as i32;
                let rgn = CreateRoundRectRgn(
                    img.left,
                    img.top,
                    img.right + 1,
                    img.bottom + 1,
                    2 * r,
                    2 * r,
                );
                SelectClipRgn(mem, rgn);
                blit(mem, img, &p.px, p.tw, p.th, whole);
                SelectClipRgn(mem, null_mut());
                DeleteObject(rgn);
                caption = if bg.is_some() {
                    format!("Output {} × {} px", ow + 2.0 * pad, oh + 2.0 * pad)
                } else {
                    "Background needs 6 hex digits".into()
                };
            }
            _ => {
                blit(mem, img, &p.px, p.tw, p.th, whole);
                if id == "resize" {
                    let k = (num("width", 1200) / ow)
                        .min(num("height", 1200) / oh)
                        .min(1.0);
                    caption = format!(
                        "{} × {} px becomes {} × {}",
                        p.ow,
                        p.oh,
                        (ow * k).round(),
                        (oh * k).round()
                    );
                }
            }
        }
    }
    let mut tr = RECT {
        left: sc(6),
        top: h - cap_h,
        right: w - sc(6),
        bottom: h,
    };
    let flags = if (*a).preview.is_none() {
        tr.top = 0; // note is centered in the whole box while loading or failed
        DT_CENTER | DT_VCENTER | DT_SINGLELINE
    } else {
        DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_END_ELLIPSIS
    };
    SetTextColor(mem, rgb(0x6B, 0x5F, 0x50));
    DrawTextW(
        mem,
        wide(&caption).as_ptr(),
        -1,
        &mut tr,
        flags | DT_NOPREFIX,
    );
    outline(
        mem,
        RECT {
            left: 0,
            top: 0,
            right: w,
            bottom: h,
        },
        RIM,
        1,
    );
    BitBlt(dc, 0, 0, w, h, mem, 0, 0, SRCCOPY);
    SelectObject(mem, old);
    DeleteObject(bmp);
    DeleteDC(mem);
    EndPaint(hwnd, &ps);
}

unsafe extern "system" fn preview_proc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    match msg {
        WM_PAINT => {
            paint_preview(hwnd);
            0
        }
        WM_ERASEBKGND => 1,
        _ => DefWindowProcW(hwnd, msg, wp, lp),
    }
}

unsafe extern "system" fn editor_proc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    match msg {
        WM_COMMAND => {
            let (id, code) = ((wp & 0xffff) as i32, ((wp >> 16) & 0xffff) as u32);
            match id {
                IDOK_ => {
                    let a = app();
                    if (*a).busy > 0 {
                        set_text(hwnd, "PieAll — a job is already running");
                        return 0;
                    }
                    let params: BTreeMap<String, String> = (*a)
                        .edit_fields
                        .iter()
                        .map(|(k, h)| (k.to_string(), get_text(*h)))
                        .collect();
                    let idx = (*a).edit_action;
                    DestroyWindow(hwnd);
                    run_action(idx, params);
                }
                IDCANCEL_ => {
                    DestroyWindow(hwnd);
                }
                200..=210 if code == EN_CHANGE => {
                    let a = app();
                    if !(*a).preview_wnd.is_null() {
                        InvalidateRect((*a).preview_wnd, null(), 0);
                    }
                }
                _ => {}
            }
            0
        }
        WM_CTLCOLORSTATIC | WM_CTLCOLOREDIT => ctl_color(wp),
        WM_CLOSE => {
            DestroyWindow(hwnd);
            0
        }
        WM_NCDESTROY => {
            let a = app();
            if (*a).editor == hwnd {
                (*a).editor = null_mut();
                (*a).edit_fields.clear();
                (*a).preview = None;
                (*a).preview_wnd = null_mut();
                (*a).preview_gen += 1;
            }
            DefWindowProcW(hwnd, msg, wp, lp)
        }
        _ => DefWindowProcW(hwnd, msg, wp, lp),
    }
}

// ---------- result window ----------

unsafe fn show_result(title: &str, body: &str) {
    let style = WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX;
    let ex = WS_EX_CONTROLPARENT;
    let (w, h) = outer_size(560, 360, style, ex);
    let mut pt: POINT = zeroed();
    GetCursorPos(&mut pt);
    let (x, y) = clamp_to_work(pt.x - w / 2, pt.y - h / 2, w, h);
    let hw = CreateWindowExW(
        ex,
        wide(C_RESULT).as_ptr(),
        wide(title).as_ptr(),
        style,
        x,
        y,
        w,
        h,
        null_mut(),
        null_mut(),
        hinst(),
        null(),
    );
    SetWindowLongPtrW(
        hw,
        GWLP_USERDATA,
        Box::into_raw(Box::new(body.to_string())) as isize,
    );
    let shown = body.replace("\r\n", "\n").replace('\n', "\r\n");
    child(hw, "STATIC", "Result", 0, 0, 12, 10, 536, 18, 0);
    let e = child(
        hw,
        "EDIT",
        &shown,
        WS_EX_CLIENTEDGE,
        (ES_MULTILINE | ES_READONLY | ES_AUTOVSCROLL) as u32 | WS_VSCROLL | WS_TABSTOP,
        12,
        30,
        536,
        272,
        300,
    );
    SendMessageW(e, 0xC5 /* EM_SETLIMITTEXT */, 0, 0);
    button(hw, "Copy", 12, 312, 110, IDOK_, true);
    button(hw, "Close", 438, 312, 110, IDCANCEL_, false);
    ShowWindow(hw, SW_SHOW);
    SetForegroundWindow(hw);
}

unsafe extern "system" fn result_proc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    match msg {
        WM_COMMAND => {
            match (wp & 0xffff) as i32 {
                IDOK_ => {
                    let p = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const String;
                    if !p.is_null() {
                        let r = crate::platform::set_clipboard_text(&*p);
                        let b = GetDlgItem(hwnd, IDOK_);
                        set_text(b, if r.is_ok() { "Copied" } else { "Copy failed" });
                    }
                }
                IDCANCEL_ => {
                    DestroyWindow(hwnd);
                }
                _ => {}
            }
            0
        }
        WM_CTLCOLORSTATIC | WM_CTLCOLOREDIT => ctl_color(wp),
        WM_CLOSE => {
            DestroyWindow(hwnd);
            0
        }
        WM_NCDESTROY => {
            let p = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut String;
            if !p.is_null() {
                drop(Box::from_raw(p));
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            }
            DefWindowProcW(hwnd, msg, wp, lp)
        }
        _ => DefWindowProcW(hwnd, msg, wp, lp),
    }
}

// ---------- main window ----------

unsafe extern "system" fn main_proc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    let state = APP.load(Ordering::Relaxed);
    match msg {
        WM_HOTKEY if wp as i32 == HOTKEY_ID => on_hotkey(),
        WM_APP_TRAY => match lp as u32 {
            WM_LBUTTONUP => show_dashboard(),
            WM_RBUTTONUP => tray_menu(),
            _ => {}
        },
        WM_APP_SHOW => show_dashboard(),
        WM_APP_UPDATE => on_update(*Box::from_raw(lp as *mut UpdateEvent)),
        WM_APP_CAPTURED => on_captured(*Box::from_raw(
            lp as *mut (u64, Result<Vec<PathBuf>, String>),
        )),
        WM_APP_RESULT => on_result(*Box::from_raw(lp as *mut Done)),
        WM_APP_PREVIEW => on_preview(*Box::from_raw(lp as *mut (u64, Result<Preview, String>))),
        WM_TIMER if wp == TIMER_CAPTURE && !state.is_null() => {
            KillTimer(hwnd, TIMER_CAPTURE);
            let a = app();
            if (*a).resolving {
                (*a).gen += 1; // a late worker reply is now stale
                finish_capture(Err("it took longer than 1.5 seconds".into()));
            }
        }
        WM_TIMER if wp == TIMER_UPDATE && !state.is_null() => {
            SetTimer(hwnd, TIMER_UPDATE, 6 * 60 * 60 * 1000, None);
            check_update(false);
        }
        WM_CLOSE => exit_app(),
        WM_DESTROY => {
            tray(NIM_DELETE, "");
            UnregisterHotKey(hwnd, HOTKEY_ID);
            PostQuitMessage(0);
        }
        m if !state.is_null() && m == (*state).taskbar_msg => tray(NIM_ADD, "PieAll"),
        _ => return DefWindowProcW(hwnd, msg, wp, lp),
    }
    0
}

unsafe fn mkfont(h: i32, weight: i32) -> HFONT {
    CreateFontW(
        -sc(h),
        0,
        0,
        0,
        weight,
        0,
        0,
        0,
        DEFAULT_CHARSET as u32,
        OUT_DEFAULT_PRECIS as u32,
        CLIP_DEFAULT_PRECIS as u32,
        CLEARTYPE_QUALITY as u32,
        0,
        wide("Segoe UI").as_ptr(),
    )
}

unsafe fn is_ours_dialog(root: HWND) -> bool {
    let mut b = [0u16; 32];
    let n = GetClassNameW(root, b.as_mut_ptr(), 32) as usize;
    let s = String::from_utf16_lossy(&b[..n]);
    s.starts_with("PieAll") && s != C_PIE && s != C_MAIN
}

pub fn run(initial_paths: Vec<PathBuf>) -> Result<(), String> {
    unsafe {
        SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_SYSTEM_AWARE);
        let mutex = CreateMutexW(null(), 0, wide("Local\\PieAll.Single").as_ptr());
        if !mutex.is_null() && GetLastError() == ERROR_ALREADY_EXISTS {
            let h = FindWindowW(wide(C_MAIN).as_ptr(), null());
            if !h.is_null() {
                PostMessageW(h, WM_APP_SHOW, 0, 0);
            }
            return Ok(()); // shortcut: initial_paths are not forwarded to the running instance
        }

        let app_box = Box::new(App {
            actions: actions::catalog(),
            main: null_mut(),
            pie: null_mut(),
            dash: null_mut(),
            c_target: null_mut(),
            c_search: null_mut(),
            c_list: null_mut(),
            c_status: null_mut(),
            editor: null_mut(),
            edit_action: 0,
            edit_fields: Vec::new(),
            preview_wnd: null_mut(),
            preview: None,
            preview_note: String::new(),
            preview_gen: 0,
            restore_to: 0,
            paths: Vec::new(),
            text: None,
            source: 0,
            source_pid: 0,
            ring: Vec::new(),
            more: 0,
            sel: None,
            list_map: Vec::new(),
            resolving: false,
            gen: 0,
            busy: 0,
            update_busy: false,
            update_available: None,
            update_ready: None,
            c_update: null_mut(),
            c_update_status: null_mut(),
            pie_active: false,
            taskbar_msg: RegisterWindowMessageW(wide("TaskbarCreated").as_ptr()),
            dpi: GetDpiForSystem() as i32,
            font: null_mut(),
            font_bold: null_mut(),
            font_small: null_mut(),
            bg: CreateSolidBrush(BASE),
        });
        APP.store(Box::into_raw(app_box), Ordering::Relaxed);
        let a = app();
        (*a).font = mkfont(14, 400);
        (*a).font_bold = mkfont(14, 700);
        (*a).font_small = mkfont(11, 400);

        register(C_MAIN, Some(main_proc), null_mut(), false);
        register(C_PIE, Some(pie_proc), null_mut(), true);
        register(C_DASH, Some(dash_proc), (*a).bg, true);
        register(C_EDIT, Some(editor_proc), (*a).bg, true);
        register(C_RESULT, Some(result_proc), (*a).bg, true);
        register(C_PREVIEW, Some(preview_proc), null_mut(), true);

        (*a).main = CreateWindowExW(
            0,
            wide(C_MAIN).as_ptr(),
            wide("PieAll").as_ptr(),
            WS_OVERLAPPED,
            0,
            0,
            0,
            0,
            null_mut(),
            null_mut(),
            hinst(),
            null(),
        );
        if (*a).main.is_null() {
            return Err("could not create the main window".into());
        }
        let sz = pie_px();
        (*a).pie = CreateWindowExW(
            WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_ACCEPTFILES,
            wide(C_PIE).as_ptr(),
            wide("PieAll action ring").as_ptr(),
            WS_POPUP,
            0,
            0,
            sz,
            sz,
            null_mut(),
            null_mut(),
            hinst(),
            null(),
        );
        SetWindowRgn((*a).pie, CreateEllipticRgn(0, 0, sz + 1, sz + 1), 1);
        create_dashboard();
        tray(NIM_ADD, "PieAll");
        if updates::configured() { SetTimer((*a).main, TIMER_UPDATE, 10_000, None); }

        let hotkey_ok = RegisterHotKey(
            (*a).main,
            HOTKEY_ID,
            MOD_CONTROL | MOD_NOREPEAT,
            VK_OEM_3 as u32,
        ) != 0;
        if !initial_paths.is_empty() {
            (*app()).paths = initial_paths;
        }
        show_dashboard();
        if !hotkey_ok {
            status("Ctrl+` is already in use. Use the tray icon or this window instead.");
        }

        let mut m: MSG = zeroed();
        while GetMessageW(&mut m, null_mut(), 0, 0) > 0 {
            let root = GetAncestor(m.hwnd, GA_ROOT);
            if !root.is_null() && is_ours_dialog(root) && IsDialogMessageW(root, &m) != 0 {
                continue;
            }
            TranslateMessage(&m);
            DispatchMessageW(&m);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hit_regions() {
        assert_eq!(hit(0.0, 0.0, 6), Hit::Center);
        assert_eq!(hit(0.0, -100.0, 6), Hit::Wedge(0)); // straight up
        assert_eq!(hit(100.0, 50.0, 6), Hit::Wedge(2)); // ~117 deg clockwise
        assert_eq!(hit(-1.0, -100.0, 6), Hit::Wedge(0)); // wraps across 12 o'clock
        assert_eq!(hit(0.0, -200.0, 6), Hit::Outside);
        assert_eq!(hit(0.0, -100.0, 0), Hit::Outside);
    }

    #[test]
    fn image_ring_prefers_core_edits() {
        let mut ids = [
            "rotate_left",
            "frame",
            "compress",
            "flip_h",
            "convert",
            "resize",
            "crop",
        ];
        ids.sort_by_key(|i| ring_rank(i));
        assert_eq!(
            &ids[..5],
            &["convert", "crop", "resize", "frame", "compress"]
        );
    }

    #[test]
    fn wedge_outline_is_rounded_and_bounded() {
        let pts = wedge_points(0, 6, 160.0, 160.0, 1.0);
        assert!(pts.len() > 2 * (ARC_STEPS + 1));
        assert!(pts
            .iter()
            .all(|p| (0..=320).contains(&p.x) && (0..=320).contains(&p.y)));
    }
}
