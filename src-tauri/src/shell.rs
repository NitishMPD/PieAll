use crate::hold;
use base64::Engine;
use pieall::{actions, platform};
use serde::Serialize;
use std::{
    collections::BTreeMap,
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, AtomicI32, AtomicU64, Ordering},
        Mutex,
    },
};
use tauri::{AppHandle, Emitter, Manager, WebviewWindow};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};
use tauri_plugin_updater::{Update, UpdaterExt};
use windows_sys::Win32::{
    Foundation::{HWND, POINT},
    Graphics::Gdi::*,
    System::Threading::{AttachThreadInput, GetCurrentThreadId},
    UI::WindowsAndMessaging::*,
};

type Result<T> = std::result::Result<T, String>;

#[derive(Default)]
struct Session {
    revision: u64,
    paths: Vec<PathBuf>,
    text: String,
    /// Set when `text` was copied out of the source window's selection; holds the
    /// user's clipboard so it can be restored after the result is pasted back.
    selection: Option<platform::ClipboardSnapshot>,
    source: usize,
    source_pid: u32,
    operation: Option<&'static str>,
    ticket: u64,
    editor: Option<String>,
    status: String,
    result: Option<String>,
    release: Option<Update>,
    download: Option<Vec<u8>>,
    update_message: String,
    progress: u8,
}

#[derive(Serialize)]
struct FieldView {
    key: &'static str,
    label: &'static str,
    default: &'static str,
}
#[derive(Serialize)]
struct ActionView {
    id: &'static str,
    title: &'static str,
    group: &'static str,
    eligible: bool,
    fields: Vec<FieldView>,
}
#[derive(Serialize)]
struct UpdateView {
    configured: bool,
    version: Option<String>,
    notes: String,
    ready: bool,
    message: String,
    progress: u8,
}
#[derive(Serialize)]
struct View {
    revision: u64,
    target: String,
    names: Vec<String>,
    kind: &'static str,
    busy: Option<&'static str>,
    editor: Option<String>,
    status: String,
    result: Option<String>,
    actions: Vec<ActionView>,
    update: UpdateView,
}

impl Session {
    fn idle(&self) -> Result<()> {
        if self.operation.is_some() || self.editor.is_some() {
            Err("Finish the current action or close its editor first.".into())
        } else {
            Ok(())
        }
    }
    fn replace(&mut self, paths: Vec<PathBuf>, text: String) {
        self.revision += 1;
        self.paths = paths;
        self.text = text;
        self.selection = None;
        self.result = None;
    }
    fn view(&self) -> View {
        let images = !self.paths.is_empty()
            && self.paths.iter().all(|p| {
                p.extension().is_some_and(|e| {
                    matches!(
                        e.to_string_lossy().to_ascii_lowercase().as_str(),
                        "png" | "jpg" | "jpeg" | "webp"
                    )
                })
            });
        let names: Vec<_> = self
            .paths
            .iter()
            .map(|p| {
                p.file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned()
            })
            .collect();
        let target = match names.len() {
            0 if !self.text.is_empty() => format!(
                "{} · {} characters",
                if self.selection.is_some() {
                    "Selected text"
                } else {
                    "Clipboard text"
                },
                self.text.chars().count()
            ),
            0 if self.source != 0 => "Active window".into(),
            0 => "Choose your files".into(),
            1 => names[0].clone(),
            n => format!("{n} selected files"),
        };
        View {
            revision: self.revision,
            target,
            names,
            kind: if images {
                "images"
            } else if !self.paths.is_empty() {
                "files"
            } else if !self.text.is_empty() {
                "text"
            } else if self.source != 0 {
                "window"
            } else {
                "empty"
            },
            busy: self.operation,
            editor: self.editor.clone(),
            status: self.status.clone(),
            result: self.result.clone(),
            actions: actions::catalog()
                .into_iter()
                .map(|a| ActionView {
                    eligible: actions::eligible(&a, &self.paths, !self.text.is_empty())
                        && (a.input != actions::InputKind::Window || self.source != 0),
                    id: a.id,
                    title: a.title,
                    group: a.group,
                    fields: a
                        .fields
                        .into_iter()
                        .map(|f| FieldView {
                            key: f.key,
                            label: f.label,
                            default: f.default,
                        })
                        .collect(),
                })
                .collect(),
            update: UpdateView {
                configured: updates_configured(),
                version: self.release.as_ref().map(|r| r.version.clone()),
                notes: self
                    .release
                    .as_ref()
                    .map(|r| r.body.clone().unwrap_or_default())
                    .unwrap_or_default(),
                ready: self.download.is_some(),
                message: self.update_message.clone(),
                progress: self.progress,
            },
        }
    }
}
fn state(app: &AppHandle) -> tauri::State<'_, Mutex<Session>> {
    app.state::<Mutex<Session>>()
}
fn notify(app: &AppHandle) {
    let _ = app.emit("session-changed", ());
}
fn local(window: &WebviewWindow) -> Result<()> {
    if matches!(window.label(), "dashboard" | "ring") {
        Ok(())
    } else {
        Err("Unknown surface.".into())
    }
}
fn dashboard_only(window: &WebviewWindow) -> Result<()> {
    if window.label() == "dashboard" {
        Ok(())
    } else {
        Err("Open the action browser for this command.".into())
    }
}
fn message(app: &AppHandle, text: impl Into<String>) {
    state(app).lock().unwrap().status = text.into();
    notify(app);
}

// A lease releases on every success, error, panic, or cancelled blocking task.
struct Lease {
    app: AppHandle,
    ticket: u64,
}
impl Lease {
    fn acquire(app: &AppHandle, operation: &'static str) -> Result<Self> {
        let s = state(app);
        let mut s = s.lock().unwrap();
        s.idle()?;
        s.ticket += 1;
        s.operation = Some(operation);
        Ok(Self {
            app: app.clone(),
            ticket: s.ticket,
        })
    }
}
impl Drop for Lease {
    fn drop(&mut self) {
        let s = state(&self.app);
        let mut s = s.lock().unwrap();
        if s.ticket == self.ticket {
            s.operation = None;
        }
        drop(s);
        notify(&self.app);
    }
}

#[tauri::command]
fn session(window: WebviewWindow, app: AppHandle) -> Result<View> {
    local(&window)?;
    Ok(state(&app).lock().unwrap().view())
}

fn show_dashboard(app: &AppHandle) -> Result<()> {
    // Through hide_ring so RING_OPEN clears; otherwise the next hold only "closes" it.
    let _ = hide_ring(app, false);
    let window = app
        .get_webview_window("dashboard")
        .ok_or("Action browser unavailable")?;
    window.show().map_err(|e| e.to_string())?;
    window.unminimize().map_err(|e| e.to_string())?;
    window.set_focus().map_err(|e| e.to_string())
}

// The ring and toast are shown with raw Win32 calls (Tauri's show() always
// activates), so they must be hidden the same way: Tauri would believe they were
// never visible and skip the hide.
fn show_inactive(window: &WebviewWindow) {
    if let Ok(hwnd) = window.hwnd() {
        unsafe {
            ShowWindow(hwnd.0 as _, SW_SHOWNOACTIVATE);
            SetWindowPos(
                hwnd.0 as _,
                HWND_TOPMOST,
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
            );
        }
    }
}
fn hide_raw(window: &WebviewWindow) {
    if let Ok(hwnd) = window.hwnd() {
        unsafe { ShowWindow(hwnd.0 as _, SW_HIDE) };
    }
}

/// Exit animation length; the window hides only after its content has faded out,
/// so the next show never flashes a stale frame.
const OUTRO_MS: u64 = 110;
static RING_OPEN: AtomicBool = AtomicBool::new(false);
static RING_GENERATION: AtomicU64 = AtomicU64::new(0);
/// Physical screen point of the last ring's center; results appear there.
static RING_X: AtomicI32 = AtomicI32::new(i32::MIN);
static RING_Y: AtomicI32 = AtomicI32::new(i32::MIN);

fn hide_ring(app: &AppHandle, restore: bool) -> Result<()> {
    if !RING_OPEN.swap(false, Ordering::SeqCst) {
        return Ok(());
    }
    let generation = RING_GENERATION.fetch_add(1, Ordering::SeqCst) + 1;
    let source = {
        let s = state(app);
        let mut s = s.lock().unwrap();
        if s.operation == Some("capture") {
            s.ticket += 1;
            s.operation = None;
            s.status = "Selection cancelled. Invoke again to select a target.".into();
        }
        (s.source, s.source_pid)
    };
    let _ = app.emit_to("ring", "ring-close", ());
    // Focus goes back at once so typing resumes while the ring fades.
    if restore && source.0 != 0 {
        unsafe {
            let mut pid = 0;
            GetWindowThreadProcessId(source.0 as _, &mut pid);
            if pid != 0 && pid == source.1 {
                SetForegroundWindow(source.0 as _);
            }
        }
    }
    if let Some(ring) = app.get_webview_window("ring") {
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(OUTRO_MS));
            if RING_GENERATION.load(Ordering::SeqCst) == generation {
                hide_raw(&ring);
            }
        });
    }
    notify(app);
    Ok(())
}

fn ring_visible(_app: &AppHandle) -> bool {
    RING_OPEN.load(Ordering::SeqCst)
}

/// Takes focus even when another app is foreground: a keyboard hook, unlike a
/// registered hotkey, does not grant PieAll the right to activate its windows.
unsafe fn force_foreground(hwnd: HWND) {
    let foreground = GetForegroundWindow();
    let theirs = GetWindowThreadProcessId(foreground, std::ptr::null_mut());
    let ours = GetCurrentThreadId();
    let attached = theirs != 0 && theirs != ours && AttachThreadInput(ours, theirs, 1) != 0;
    BringWindowToTop(hwnd);
    SetForegroundWindow(hwnd);
    if attached {
        AttachThreadInput(ours, theirs, 0);
    }
}

fn focus_ring(app: &AppHandle) {
    let Some(ring) = app.get_webview_window("ring") else {
        return;
    };
    let _ = app.run_on_main_thread(move || {
        if let Ok(hwnd) = ring.hwnd() {
            unsafe { force_foreground(hwnd.0 as _) };
        }
        let _ = ring.set_focus();
    });
}

/// Shows the ring centered on the cursor without activating it, so the source
/// window keeps focus until its selection has been read.
fn place_ring(app: &AppHandle, held: bool) -> Result<()> {
    let ring = app.get_webview_window("ring").ok_or("Ring unavailable")?;
    let mut point = POINT { x: 0, y: 0 };
    unsafe {
        GetCursorPos(&mut point);
    }
    let scale = ring
        .monitor_from_point(point.x as f64, point.y as f64)
        .map_err(|e| e.to_string())?
        .map_or(1.0, |m| m.scale_factor());
    let size = (440.0 * scale).round() as i32;
    let mut info: MONITORINFO = unsafe { std::mem::zeroed() };
    info.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
    unsafe {
        GetMonitorInfoW(MonitorFromPoint(point, MONITOR_DEFAULTTONEAREST), &mut info);
    }
    let work = info.rcWork;
    let x = (point.x - size / 2).clamp(work.left, (work.right - size).max(work.left));
    let y = (point.y - size / 2).clamp(work.top, (work.bottom - size).max(work.top));
    ring.set_size(tauri::LogicalSize::new(440.0, 440.0))
        .map_err(|e| e.to_string())?;
    ring.set_position(tauri::PhysicalPosition::new(x, y))
        .map_err(|e| e.to_string())?;
    RING_X.store(x + size / 2, Ordering::SeqCst);
    RING_Y.store(y + size / 2, Ordering::SeqCst);
    RING_GENERATION.fetch_add(1, Ordering::SeqCst);
    RING_OPEN.store(true, Ordering::SeqCst);
    let _ = app.emit_to("ring", "ring-opened", held);
    let hwnd = ring.hwnd().map_err(|e| e.to_string())?.0 as HWND;
    unsafe {
        // Undecorated windows keep caption/frame styles. Once a window region is set,
        // Windows paints its classic inactive caption, which showed as a pale crescent
        // above the top slice. Strip them before clipping.
        let style = GetWindowLongPtrW(hwnd, GWL_STYLE);
        let bare = style & !((WS_CAPTION | WS_THICKFRAME) as isize);
        if bare != style {
            SetWindowLongPtrW(hwnd, GWL_STYLE, bare);
            SetWindowPos(
                hwnd,
                std::ptr::null_mut(),
                0,
                0,
                0,
                0,
                SWP_FRAMECHANGED | SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
            );
        }
        // Clip the native hit surface too; transparent corners must not eat desktop clicks.
        let inset = (17.0 * scale).round() as i32;
        let region = CreateEllipticRgn(inset, inset, size - inset, size - inset);
        if !region.is_null() && SetWindowRgn(hwnd, region, 1) == 0 {
            DeleteObject(region);
        }
    }
    show_inactive(&ring);
    Ok(())
}

/// Reports where the cursor was when the held key came up, in ring viewBox units.
fn release_ring(app: &AppHandle) {
    let Some(ring) = app.get_webview_window("ring") else {
        return;
    };
    if !RING_OPEN.load(Ordering::SeqCst) {
        return;
    }
    let mut point = POINT { x: 0, y: 0 };
    unsafe {
        GetCursorPos(&mut point);
    }
    if let (Ok(origin), Ok(scale)) = (ring.outer_position(), ring.scale_factor()) {
        let x = (point.x - origin.x) as f64 / scale;
        let y = (point.y - origin.y) as f64 / scale;
        let _ = app.emit_to("ring", "ring-release", (x, y));
    }
    // The web side decides what the release does (run a slice or close), but the
    // ring must never outlive the key. Once capture is done and the web side has
    // had its turn, close whatever ring is still the one this release belonged to.
    let generation = RING_GENERATION.load(Ordering::SeqCst);
    let app = app.clone();
    std::thread::spawn(move || {
        for _ in 0..40 {
            if state(&app).lock().unwrap().operation != Some("capture") {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        std::thread::sleep(std::time::Duration::from_millis(400));
        if RING_GENERATION.load(Ordering::SeqCst) == generation {
            let _ = hide_ring(&app, true);
        }
    });
}

/// Opens the ring for whatever is under the user's hands: the Explorer or desktop
/// selection, else selected text in the foreground app, else that app's window.
fn invoke_ring(app: &AppHandle, held: bool) -> Result<()> {
    if ring_visible(app) {
        return hide_ring(app, true);
    }
    {
        let s = state(app);
        s.lock().unwrap().idle()?;
    }
    let lease = Lease::acquire(app, "capture")?;
    let source = unsafe { GetForegroundWindow() } as usize;
    let own_window = ["dashboard", "ring", "toast"]
        .iter()
        .filter_map(|label| app.get_webview_window(label))
        .any(|w| w.hwnd().is_ok_and(|h| h.0 as usize == source));
    let source = if own_window { 0 } else { source };
    {
        let s = state(app);
        let mut s = s.lock().unwrap();
        s.replace(Vec::new(), String::new());
        s.source = source;
        s.source_pid = 0;
        if source != 0 {
            unsafe {
                GetWindowThreadProcessId(source as _, &mut s.source_pid);
            }
        }
        s.status = "Reading selection…".into();
    }
    notify(app);
    place_ring(app, held)?;
    let shell = source != 0 && platform::is_shell_window(source);
    let wants_text = source != 0 && !shell && !platform::is_terminal_window(source);
    if !wants_text {
        // Explorer is read over COM, which does not need its window focused.
        focus_ring(app);
    }
    let owner = app
        .get_webview_window("ring")
        .and_then(|w| w.hwnd().ok())
        .map_or(0, |h| h.0 as usize);
    let app = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let result = if shell {
            platform::capture_selection_from(source).map(|paths| (paths, None))
        } else if wants_text {
            Ok((Vec::new(), platform::copy_selection(owner)))
        } else {
            Ok((Vec::new(), None))
        };
        let current = {
            let s = state(&app);
            let mut s = s.lock().unwrap();
            if s.ticket != lease.ticket {
                return;
            }
            match result {
                Ok((paths, text)) => {
                    match text {
                        Some((text, snapshot)) => {
                            s.replace(paths, text);
                            s.selection = Some(snapshot);
                        }
                        None => s.replace(paths, String::new()),
                    }
                    s.status = "Choose an action. Your originals stay untouched.".into();
                }
                Err(error) => {
                    s.status = format!("{error} Window actions remain available.");
                }
            }
            true
        };
        drop(lease);
        if current && wants_text {
            focus_ring(&app);
        }
    });
    Ok(())
}

#[tauri::command]
fn surface(window: WebviewWindow, app: AppHandle, action: String) -> Result<()> {
    local(&window)?;
    match action.as_str() {
        "dashboard" => show_dashboard(&app),
        "ring" => {
            state(&app).lock().unwrap().idle()?;
            place_ring(&app, false)?;
            focus_ring(&app);
            Ok(())
        }
        "dismiss" => {
            if window.label() == "ring" {
                hide_ring(&app, true)
            } else {
                window.hide().map_err(|e| e.to_string())
            }
        }
        _ => Err("Unknown window action.".into()),
    }
}

#[tauri::command]
async fn load_context(window: WebviewWindow, app: AppHandle, kind: String) -> Result<()> {
    dashboard_only(&window)?;
    state(&app).lock().unwrap().idle()?;
    let lease = Lease::acquire(&app, "selection")?;
    notify(&app);
    tauri::async_runtime::spawn_blocking(move || {
        let _lease = lease;
        let (paths, text) = match kind.as_str() {
            "files" => {
                let paths = platform::pick_files()?;
                if paths.is_empty() {
                    return Ok(());
                }
                (paths, String::new())
            }
            "clipboard" => (Vec::new(), platform::clipboard_text()?),
            _ => return Err("Unknown selection source.".into()),
        };
        if paths.len() > 256 {
            return Err("Select at most 256 files.".into());
        }
        let s = state(&app);
        let mut s = s.lock().unwrap();
        s.replace(paths, text);
        s.source = 0;
        s.source_pid = 0;
        s.status = "Selection ready. Choose an action.".into();
        Ok(())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
fn prepare(window: WebviewWindow, app: AppHandle, id: Option<String>, revision: u64) -> Result<()> {
    local(&window)?;
    let s = state(&app);
    let mut s = s.lock().unwrap();
    if s.operation.is_some() || revision != s.revision {
        return Err("Selection changed or an action is running.".into());
    }
    if let Some(ref id) = id {
        let action = s
            .view()
            .actions
            .into_iter()
            .find(|a| a.id == id)
            .ok_or("Unknown action")?;
        if !action.eligible {
            return Err("This action does not support the selected target.".into());
        }
    }
    s.editor = id;
    drop(s);
    notify(&app);
    if window.label() == "ring" {
        show_dashboard(&app)?;
    }
    Ok(())
}

#[tauri::command]
async fn run_action(
    window: WebviewWindow,
    app: AppHandle,
    id: String,
    revision: u64,
    mut params: BTreeMap<String, String>,
) -> Result<()> {
    local(&window)?;
    if params.len() > 16 || params.iter().any(|(k, v)| k.len() > 64 || v.len() > 256) {
        return Err("Action parameters exceed limits.".into());
    }
    let from_ring = window.label() == "ring";
    let lease;
    let (paths, text, title, selection, source) = {
        let s = state(&app);
        let mut s = s.lock().unwrap();
        if revision != s.revision {
            return Err("Selection changed. Choose the action again.".into());
        }
        let action = actions::catalog()
            .into_iter()
            .find(|a| a.id == id)
            .ok_or("Unknown action")?;
        if !actions::eligible(&action, &s.paths, !s.text.is_empty())
            || (action.input == actions::InputKind::Window && s.source == 0)
        {
            return Err("This action does not support the selected target.".into());
        }
        if s.operation.is_some() || s.editor.as_ref().is_some_and(|e| e != &id) {
            return Err("Finish the current action first.".into());
        }
        // Caller cannot replace the captured window identity through IPC parameters.
        params.remove("source_hwnd");
        params.remove("source_pid");
        if action.input == actions::InputKind::Window {
            params.insert("source_hwnd".into(), s.source.to_string());
            params.insert("source_pid".into(), s.source_pid.to_string());
        }
        s.ticket += 1;
        s.operation = Some("action");
        lease = Lease {
            app: app.clone(),
            ticket: s.ticket,
        };
        s.editor = None;
        s.result = None;
        s.status = format!("Running {}…", action.title);
        let selection = if from_ring { s.selection.take() } else { None };
        (
            s.paths.clone(),
            s.text.clone(),
            action.title,
            selection,
            s.source,
        )
    };
    notify(&app);
    if from_ring {
        hide_ring(&app, true)?;
        // Most actions finish before a progress notice would be worth seeing.
        let slow = app.clone();
        let ticket = lease.ticket;
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(700));
            let running = {
                let s = state(&slow);
                let s = s.lock().unwrap();
                s.operation == Some("action") && s.ticket == ticket
            };
            if running {
                toast(&slow, "busy", format!("{title}…"), "");
            }
        });
    } else {
        show_dashboard(&app)?;
    }
    tauri::async_runtime::spawn_blocking(move || {
        let _lease = lease;
        let result = actions::execute(&id, &paths, &text, &params);
        {
            let s = state(&app);
            let mut s = s.lock().unwrap();
            match &result {
                Ok(result) => {
                    s.result = Some(result.clone());
                    s.status = "Action complete. Original files preserved.".into();
                }
                Err(error) => {
                    s.status = error.clone();
                }
            }
        }
        if from_ring {
            finish_in_place(&app, &id, result, selection, source);
        }
    })
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
fn copy_result(window: WebviewWindow, app: AppHandle) -> Result<()> {
    dashboard_only(&window)?;
    let text = state(&app)
        .lock()
        .unwrap()
        .result
        .clone()
        .ok_or("No result to copy")?;
    platform::set_clipboard_text(&text)
}

#[tauri::command]
async fn preview(window: WebviewWindow, app: AppHandle, revision: u64) -> Result<Option<Preview>> {
    local(&window)?;
    let path = {
        let s = state(&app);
        let s = s.lock().unwrap();
        if revision != s.revision {
            return Err("Selection changed.".into());
        }
        s.paths.first().cloned()
    };
    tauri::async_runtime::spawn_blocking(move || {
        static PREVIEW_GATE: Mutex<()> = Mutex::new(());
        let _gate = PREVIEW_GATE
            .lock()
            .map_err(|_| "Preview decoder unavailable")?;
        let Some(path) = path else {
            return Ok(None);
        };
        if std::fs::metadata(&path).map_err(|e| e.to_string())?.len() > 64 * 1024 * 1024 {
            return Ok(None);
        }
        pieall::output::plain_path(&path)?;
        let mut reader = image::ImageReader::open(&path)
            .map_err(|e| e.to_string())?
            .with_guessed_format()
            .map_err(|e| e.to_string())?;
        let mut limits = image::Limits::default();
        limits.max_alloc = Some(128 * 1024 * 1024);
        limits.max_image_width = Some(16384);
        limits.max_image_height = Some(16384);
        reader.limits(limits);
        let orientation = reader
            .into_decoder()
            .map_err(|e| e.to_string())
            .and_then(|mut d| {
                use image::ImageDecoder;
                let orientation = d.orientation().map_err(|e| e.to_string())?;
                let mut image = image::DynamicImage::from_decoder(d).map_err(|e| e.to_string())?;
                image.apply_orientation(orientation);
                Ok(image)
            })?;
        let width = orientation.width();
        let height = orientation.height();
        let thumb = orientation.thumbnail(320, 320);
        let mut bytes = std::io::Cursor::new(Vec::new());
        thumb
            .write_to(&mut bytes, image::ImageFormat::Png)
            .map_err(|e| e.to_string())?;
        Ok(Some(Preview {
            width,
            height,
            url: format!(
                "data:image/png;base64,{}",
                base64::engine::general_purpose::STANDARD.encode(bytes.into_inner())
            ),
        }))
    })
    .await
    .map_err(|e| e.to_string())?
}

#[derive(Serialize)]
struct Preview {
    url: String,
    width: u32,
    height: u32,
}

#[derive(Serialize, Clone)]
struct ToastView {
    tone: &'static str,
    title: String,
    detail: String,
}

/// A small click-through pill where the ring was; it never takes focus.
fn toast(app: &AppHandle, tone: &'static str, title: impl Into<String>, detail: impl Into<String>) {
    static GENERATION: AtomicU64 = AtomicU64::new(0);
    let Some(window) = app.get_webview_window("toast") else {
        return;
    };
    let generation = GENERATION.fetch_add(1, Ordering::SeqCst) + 1;
    let _ = app.emit_to(
        "toast",
        "toast",
        ToastView {
            tone,
            title: title.into(),
            detail: detail.into(),
        },
    );
    let mut point = POINT {
        x: RING_X.load(Ordering::SeqCst),
        y: RING_Y.load(Ordering::SeqCst),
    };
    if point.x == i32::MIN {
        unsafe { GetCursorPos(&mut point) };
    }
    let mut info: MONITORINFO = unsafe { std::mem::zeroed() };
    info.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
    unsafe {
        GetMonitorInfoW(MonitorFromPoint(point, MONITOR_DEFAULTTONEAREST), &mut info);
    }
    let scale = window
        .monitor_from_point(point.x as f64, point.y as f64)
        .ok()
        .flatten()
        .map_or(1.0, |m| m.scale_factor());
    let (width, height) = (360.0, 64.0);
    let (w, h) = ((width * scale) as i32, (height * scale) as i32);
    let work = info.rcWork;
    let x = (point.x - w / 2).clamp(work.left, (work.right - w).max(work.left));
    let y = (point.y - h / 2).clamp(work.top, (work.bottom - h).max(work.top));
    let _ = window.set_size(tauri::LogicalSize::new(width, height));
    let _ = window.set_position(tauri::PhysicalPosition::new(x, y));
    show_inactive(&window);
    if tone != "busy" {
        let linger = if tone == "error" { 3200 } else { 1400 };
        let app = app.clone();
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(linger));
            if GENERATION.load(Ordering::SeqCst) != generation {
                return;
            }
            let _ = app.emit_to("toast", "toast-hide", ());
            std::thread::sleep(std::time::Duration::from_millis(OUTRO_MS + 60));
            if GENERATION.load(Ordering::SeqCst) == generation {
                hide_raw(&window);
            }
        });
    }
}

/// Actions whose answer is text to keep; from the ring it goes to the clipboard.
const REPORTS: &[&str] = &[
    "sha256",
    "image_details",
    "palette",
    "folder_size",
    "find_duplicates",
    "list_zip",
    "window_details",
    "count",
];

fn file_name(path: &str) -> String {
    std::path::Path::new(path)
        .file_name()
        .map_or_else(|| path.to_string(), |n| n.to_string_lossy().into_owned())
}

fn first_failure(lines: &[&str]) -> Option<String> {
    lines
        .iter()
        .find_map(|l| l.strip_prefix("FAILED "))
        .map(|l| l.split_once(": ").map_or(l, |(_, why)| why).to_string())
}

/// Turns a worker report into a short toast: what was saved, or why not.
fn summarize(report: &str, failed: bool) -> (&'static str, String, String) {
    let lines: Vec<&str> = report
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with("Completed "))
        .collect();
    let saved: Vec<(String, String)> = lines
        .iter()
        .filter_map(|l| {
            l.strip_prefix("Saved ")
                .or_else(|| l.strip_prefix("Extracted to "))
        })
        .map(|rest| match rest.rfind(" (") {
            Some(i) if rest[i..].contains('→') => (
                file_name(&rest[..i]),
                rest[i + 2..].trim_end_matches(')').to_string(),
            ),
            _ => (file_name(rest), String::new()),
        })
        .collect();
    let failures = lines.iter().filter(|l| l.starts_with("FAILED ")).count();
    let why = first_failure(&lines);
    match (saved.as_slice(), failures) {
        ([], 0) => {
            if let Some(skip) = lines.iter().find_map(|l| l.strip_prefix("Skipped ")) {
                let why = skip.split_once(": ").map_or(skip, |(_, w)| w);
                return ("info", "Nothing saved".into(), capitalize(why));
            }
            let first = lines.first().copied().unwrap_or("Done.").to_string();
            if failed {
                ("error", "Couldn't finish".into(), first)
            } else {
                ("ok", first, lines.get(1).copied().unwrap_or("").to_string())
            }
        }
        ([], _) => ("error", "Couldn't finish".into(), why.unwrap_or_default()),
        // "2.4 MB → 820 KB, 66% smaller" is trimmed to its point: "66% smaller".
        ([(name, extra)], 0) => (
            "ok",
            format!("Saved {name}"),
            extra.rsplit(", ").next().unwrap_or_default().to_string(),
        ),
        ([_, rest @ ..], 0) => (
            "ok",
            format!("Saved {} files", rest.len() + 1),
            String::new(),
        ),
        (done, _) => (
            "error",
            format!("Saved {} of {}", done.len(), done.len() + failures),
            why.unwrap_or_default(),
        ),
    }
}

fn capitalize(text: &str) -> String {
    let mut chars = text.chars();
    chars.next().map_or_else(String::new, |c| {
        c.to_uppercase().collect::<String>() + chars.as_str()
    })
}

/// Reports a ring action where the user is: pasted over their selection, copied, or toasted.
fn finish_in_place(
    app: &AppHandle,
    id: &str,
    result: Result<String>,
    selection: Option<platform::ClipboardSnapshot>,
    source: usize,
) {
    let report = match result {
        Ok(report) => report,
        Err(error) => {
            let (_, title, detail) = summarize(&error, true);
            toast(app, "error", title, detail);
            return;
        }
    };
    if REPORTS.contains(&id) {
        let body = report
            .split_once("\r\n\r\n")
            .filter(|_| report.starts_with("Completed "))
            .map_or(report.as_str(), |(_, body)| body);
        let preview: String = body.lines().take(2).collect::<Vec<_>>().join(" · ");
        match platform::set_clipboard_text(body) {
            Ok(()) => toast(app, "ok", "Copied to clipboard", preview),
            Err(error) => toast(app, "error", "Couldn't copy the result", error),
        }
        return;
    }
    if let Some(snapshot) = selection {
        let owner = app
            .get_webview_window("ring")
            .and_then(|w| w.hwnd().ok())
            .map_or(0, |h| h.0 as usize);
        if !matches!(
            platform::paste_text(source, &report, &snapshot, owner),
            Ok(true)
        ) {
            toast(
                app,
                "info",
                "Result copied",
                "Focus moved before PieAll could paste. Press Ctrl+V to paste it.",
            );
        }
        return;
    }
    let (tone, title, detail) = summarize(&report, false);
    toast(app, tone, title, detail);
}

fn updates_configured() -> bool {
    !cfg!(debug_assertions)
        && valid_update_key(option_env!("PIEALL_TAURI_PUBLIC_KEY").unwrap_or(""))
}

fn valid_update_key(key: &str) -> bool {
    base64::engine::general_purpose::STANDARD
        .decode(key.trim())
        .ok()
        .and_then(|bytes| String::from_utf8(bytes).ok())
        .is_some_and(|text| minisign_verify::PublicKey::decode(&text).is_ok())
}

fn confirm_version(available: &str, expected: Option<&str>) -> Result<()> {
    if expected != Some(available) {
        return Err("The update changed. Review and confirm its version again.".into());
    }
    Ok(())
}

#[tauri::command]
async fn update_action(
    window: WebviewWindow,
    app: AppHandle,
    action: String,
    expected_version: Option<String>,
) -> Result<()> {
    dashboard_only(&window)?;
    update(&app, &action, expected_version.as_deref()).await
}

async fn update(app: &AppHandle, action: &str, expected_version: Option<&str>) -> Result<()> {
    if !updates_configured() {
        return Err(
            "Updates require a signed release configured with PieAll's own public key.".into(),
        );
    }
    state(app).lock().unwrap().idle()?;
    let _lease = Lease::acquire(app, "update")?;
    state(app).lock().unwrap().update_message = match action {
        "check" => "Checking for updates…",
        "download" => "Downloading update…",
        "install" => "Verifying and starting the installer…",
        _ => "Unknown update action.",
    }
    .into();
    notify(app);
    let result: Result<()> = async {
        match action {
            "check" => {
                if state(app).lock().unwrap().download.is_some() { return Err("An update is already downloaded. Install it when convenient.".into()); }
                let release = app.updater_builder()
                    .pubkey(option_env!("PIEALL_TAURI_PUBLIC_KEY").unwrap_or(""))
                    .timeout(std::time::Duration::from_secs(30))
                    .build().map_err(|e| e.to_string())?.check().await.map_err(|e| e.to_string())?;
                if let Some(ref release) = release {
                    let version = semver::Version::parse(&release.version).map_err(|e| e.to_string())?;
                    let expected = format!("https://github.com/NitishMPD/PieAll/releases/download/v{version}/PieAll_{version}_x64-setup.exe");
                    if !version.pre.is_empty() || !version.build.is_empty() || release.download_url.as_str() != expected || release.body.as_ref().is_some_and(|notes| notes.len() > 128 * 1024) {
                        return Err("Invalid stable PieAll release metadata.".into());
                    }
                }
                let s = state(app); let mut s = s.lock().unwrap();
                s.update_message = if release.is_some() { "Update available. Read the notes, then download.".into() } else { "You are up to date.".into() };
                s.release = release;
            }
            "download" => {
                let release = state(app).lock().unwrap().release.clone().ok_or("Check for an update first")?;
                state(app).lock().unwrap().progress = 0;
                let client = reqwest::Client::builder().https_only(true)
                    .timeout(std::time::Duration::from_secs(600)).build().map_err(|e| e.to_string())?;
                let mut response = client.get(release.download_url.clone()).send().await.map_err(|e| e.to_string())?.error_for_status().map_err(|e| e.to_string())?;
                let total = response.content_length();
                const MAX: u64 = 256 * 1024 * 1024;
                if total.is_some_and(|n| n > MAX) { return Err("Installer exceeds 256 MiB.".into()); }
                let mut bytes = Vec::new();
                while let Some(chunk) = response.chunk().await.map_err(|e| e.to_string())? {
                    if (bytes.len() as u64).saturating_add(chunk.len() as u64) > MAX { return Err("Installer exceeds 256 MiB.".into()); }
                    bytes.extend_from_slice(&chunk);
                    let n = total.filter(|n| *n > 0).map_or(0, |n| ((bytes.len() as u64).saturating_mul(100) / n).min(99) as u8);
                    let s = state(app); let mut s = s.lock().unwrap();
                    if s.progress != n { s.progress = n; drop(s); notify(app); }
                }
                if bytes.is_empty() || total.is_some_and(|n| n != bytes.len() as u64) { return Err("Installer download was incomplete.".into()); }
                release.verify(&bytes).map_err(|e| e.to_string())?;
                // Bounded transport; Tauri verifies both the installer and its signed version.
                let s = state(app); let mut s = s.lock().unwrap();
                s.download = Some(bytes); s.progress = 100;
                s.update_message = "Signature verified. Ready to install.".into();
            }
            "install" => {
                let (release, bytes) = {
                    let s = state(app); let mut s = s.lock().unwrap();
                    let release = s.release.clone().ok_or("Check for an update first")?;
                    confirm_version(&release.version, expected_version)?;
                    let bytes = s.download.take().ok_or("Download an update first")?;
                    (release, bytes)
                };
                // Keep the lease through the native installer handoff; plugin owns Windows exit.
                tauri::async_runtime::spawn_blocking(move || release.install(bytes).map_err(|e| e.to_string())).await.map_err(|e| e.to_string())??;
            }
            _ => return Err("Unknown update action.".into()),
        }
        Ok(())
    }.await;
    if let Err(ref error) = result {
        state(app).lock().unwrap().update_message = error.clone();
    }
    result
}

fn accept_paths(app: &AppHandle, paths: Vec<PathBuf>) -> Result<()> {
    if paths.is_empty() {
        return Ok(());
    }
    if paths.len() > 256 {
        return Err("Select at most 256 files.".into());
    }
    let s = state(app);
    let mut s = s.lock().unwrap();
    s.idle()?;
    s.replace(paths, String::new());
    s.source = 0;
    s.source_pid = 0;
    s.status = "Files ready. Choose an action.".into();
    drop(s);
    notify(app);
    Ok(())
}

fn file_args(args: impl Iterator<Item = String>, cwd: &str) -> Vec<PathBuf> {
    args.filter(|arg| !arg.starts_with('-'))
        .map(|arg| {
            let p = PathBuf::from(arg);
            if p.is_absolute() {
                p
            } else {
                PathBuf::from(cwd).join(p)
            }
        })
        .collect()
}

pub fn run() {
    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(Mutex::new(Session {
            status: "Select something anywhere in Windows, then hold ` to open the ring.".into(),
            update_message: "Updates require a configured signed release.".into(),
            ..Session::default()
        }))
        .plugin(tauri_plugin_single_instance::init(|app, args, cwd| {
            if let Err(error) = accept_paths(app, file_args(args.into_iter().skip(1), &cwd)) {
                message(app, error);
            }
            let _ = show_dashboard(app);
        }))
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _, event| {
                    if event.state == ShortcutState::Pressed {
                        if let Err(error) = invoke_ring(app, false) {
                            message(app, error);
                        }
                    }
                })
                .build(),
        )
        .invoke_handler(tauri::generate_handler![
            session,
            surface,
            load_context,
            prepare,
            run_action,
            copy_result,
            preview,
            update_action
        ])
        .setup(|app| {
            let cwd = std::env::current_dir()?.to_string_lossy().into_owned();
            if let Err(error) =
                accept_paths(app.handle(), file_args(std::env::args().skip(1), &cwd))
            {
                message(app.handle(), error);
            }
            if let Err(error) = app.global_shortcut().register("Ctrl+Backquote") {
                message(
                    app.handle(),
                    format!("Ctrl + ` is unavailable: {error}. Hold ` or use the tray."),
                );
            }
            if let Some(toast_window) = app.get_webview_window("toast") {
                let _ = toast_window.set_ignore_cursor_events(true);
                if let Ok(hwnd) = toast_window.hwnd() {
                    unsafe {
                        let style = GetWindowLongPtrW(hwnd.0 as _, GWL_EXSTYLE);
                        SetWindowLongPtrW(
                            hwnd.0 as _,
                            GWL_EXSTYLE,
                            style | (WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW) as isize,
                        );
                    }
                }
            }
            let handle = app.handle().clone();
            let hook = hold::install(move |event| {
                let result = match event {
                    hold::Hold::Start => invoke_ring(&handle, true),
                    hold::Hold::End => {
                        release_ring(&handle);
                        Ok(())
                    }
                };
                if let Err(error) = result {
                    toast(&handle, "error", "PieAll is busy", error);
                }
            });
            if let Err(error) = hook {
                let startup = app.handle().clone();
                std::thread::spawn(move || {
                    // Let the toast surface load before its first message.
                    std::thread::sleep(std::time::Duration::from_millis(1500));
                    toast(
                        &startup,
                        "error",
                        "Hold ` is unavailable",
                        format!("{error} Use Ctrl + ` instead."),
                    );
                });
            }
            let open =
                tauri::menu::MenuItem::with_id(app, "open", "Open PieAll", true, None::<&str>)?;
            let ring = tauri::menu::MenuItem::with_id(
                app,
                "ring",
                "Show action ring (hold `)",
                true,
                None::<&str>,
            )?;
            let exit =
                tauri::menu::MenuItem::with_id(app, "exit", "Exit PieAll", true, None::<&str>)?;
            let menu = tauri::menu::Menu::with_items(app, &[&open, &ring, &exit])?;
            let pixels = include_bytes!("../icons/tray.rgba").to_vec();
            tauri::tray::TrayIconBuilder::new()
                .icon(tauri::image::Image::new_owned(pixels, 32, 32))
                .tooltip("PieAll · hold ` over a file, text or window")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| {
                    let result = match event.id.as_ref() {
                        "open" => show_dashboard(app),
                        "ring" => invoke_ring(app, false),
                        "exit" => {
                            let idle = state(app).lock().unwrap().idle();
                            if idle.is_ok() {
                                app.exit(0);
                            }
                            idle
                        }
                        _ => Ok(()),
                    };
                    if let Err(error) = result {
                        message(app, error);
                        let _ = show_dashboard(app);
                    }
                })
                .on_tray_icon_event(|tray, event| {
                    if matches!(
                        event,
                        tauri::tray::TrayIconEvent::Click {
                            button: tauri::tray::MouseButton::Left,
                            button_state: tauri::tray::MouseButtonState::Up,
                            ..
                        }
                    ) {
                        let _ = show_dashboard(tray.app_handle());
                    }
                })
                .build(app)?;
            if updates_configured() {
                let app = app.handle().clone();
                std::thread::spawn(move || {
                    std::thread::sleep(std::time::Duration::from_secs(10));
                    loop {
                        let app = app.clone();
                        tauri::async_runtime::spawn(async move {
                            let should_check = {
                                let s = state(&app);
                                let s = s.lock().unwrap();
                                s.idle().is_ok() && s.release.is_none() && s.download.is_none()
                            };
                            if should_check {
                                let _ = update(&app, "check", None).await;
                            }
                        });
                        std::thread::sleep(std::time::Duration::from_secs(6 * 60 * 60));
                    }
                });
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            let app = window.app_handle();
            match event {
                tauri::WindowEvent::CloseRequested { api, .. } => {
                    api.prevent_close();
                    let _ = window.hide();
                }
                tauri::WindowEvent::Focused(false) if window.label() == "ring" => {
                    let _ = hide_ring(app, false);
                }
                tauri::WindowEvent::DragDrop(tauri::DragDropEvent::Drop { paths, .. }) => {
                    if let Err(error) = accept_paths(app, paths.clone()) {
                        message(app, error);
                    }
                    if window.label() == "ring" {
                        let _ = show_dashboard(app);
                    }
                }
                _ => {}
            }
        });
    if let Err(error) = builder.run(tauri::generate_context!()) {
        platform::show_error(&error.to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn update_install_requires_the_reviewed_version_and_valid_key() {
        assert!(confirm_version("0.1.42", Some("0.1.42")).is_ok());
        assert!(confirm_version("0.1.42", Some("0.1.41")).is_err());
        assert!(confirm_version("0.1.42", None).is_err());
        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../../scripts/fixtures/release-signature.json"
        ))
        .unwrap();
        assert!(valid_update_key(fixture["key"].as_str().unwrap()));
        assert!(!valid_update_key("invalid"));
        assert!(!valid_update_key(""));
    }
    #[test]
    fn summaries_name_the_saved_file_or_the_failure() {
        let (tone, title, detail) = summarize(
            "Completed 1 of 1 targets.\r\n\r\nSaved C:\\Pics\\cat-compressed (2).jpg (2.0 MB → 700 KB, 66% smaller)",
            false,
        );
        assert_eq!(tone, "ok");
        assert_eq!(title, "Saved cat-compressed (2).jpg");
        assert_eq!(detail, "66% smaller");
        let (tone, title, _) = summarize(
            "Completed 2 of 2 targets.\r\n\r\nSaved C:\\a-rotated.png\r\n\r\nSaved C:\\b-rotated.png",
            false,
        );
        assert_eq!((tone, title.as_str()), ("ok", "Saved 2 files"));
        let (tone, title, detail) = summarize(
            "Completed 0 of 1 targets.\r\n\r\nFAILED C:\\x.png: Image exceeds 40 megapixels.",
            true,
        );
        assert_eq!(tone, "error");
        assert_eq!(title, "Couldn't finish");
        assert_eq!(detail, "Image exceeds 40 megapixels.");
        let (tone, title, _) = summarize(
            "Completed 1 of 1 targets.\r\n\r\nSkipped C:\\x.jpg: it is already as small as this setting makes it.",
            false,
        );
        assert_eq!((tone, title.as_str()), ("info", "Nothing saved"));
    }
    #[test]
    fn replacement_clears_previous_context_and_results() {
        let mut s = Session {
            text: "secret".into(),
            result: Some("old result".into()),
            ..Session::default()
        };
        s.replace(vec![PathBuf::from("C:/sample.png")], String::new());
        assert_eq!(s.revision, 1);
        assert!(s.text.is_empty());
        assert!(s.result.is_none());
        s.replace(Vec::new(), "new text".into());
        assert_eq!(s.revision, 2);
        assert!(s.paths.is_empty());
        assert_eq!(s.view().kind, "text");
    }
    #[test]
    fn empty_session_disables_window_actions_and_busy_editor_blocks_changes() {
        let mut s = Session::default();
        assert!(s.view().actions.iter().all(|a| !a.eligible));
        s.source = 123;
        assert_eq!(s.view().actions.iter().filter(|a| a.eligible).count(), 4);
        s.editor = Some("crop".into());
        assert!(s.idle().is_err());
        s.editor = None;
        s.operation = Some("update");
        assert!(s.idle().is_err());
    }
}
