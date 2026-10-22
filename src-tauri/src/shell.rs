use crate::hold;
use base64::Engine;
use pieall::{actions, platform, shortcuts};
use serde::Serialize;
use std::{
    collections::BTreeMap,
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, AtomicI32, AtomicU64, Ordering},
        Arc, Mutex,
    },
};
use tauri::{AppHandle, Emitter, Manager, WebviewWindow};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};
use tauri_plugin_updater::{Update, UpdaterExt};
use windows_sys::Win32::{
    Foundation::{HWND, POINT, RECT},
    Graphics::{
        Dwm::{DwmSetWindowAttribute, DWMWA_BORDER_COLOR, DWMWA_COLOR_NONE},
        Gdi::*,
    },
    System::Threading::{AttachThreadInput, GetCurrentThreadId},
    UI::HiDpi::{GetDpiForMonitor, GetDpiForWindow, MDT_EFFECTIVE_DPI},
    UI::WindowsAndMessaging::*,
};

type Result<T> = std::result::Result<T, String>;

const INVOCATION: &str = "Ctrl+Backquote";
fn set_shortcuts_enabled(app: &AppHandle, enabled: bool) -> Result<()> {
    let session = state(app);
    let session = session.lock().unwrap();
    // The operation lease also covers temporary Ctrl+` release and restoration.
    session.idle()?;
    if enabled {
        if !app.global_shortcut().is_registered(INVOCATION) {
            app.global_shortcut()
                .register(INVOCATION)
                .map_err(|e| format!("Could not enable Ctrl+`: {e}"))?;
        }
    } else if app.global_shortcut().is_registered(INVOCATION) {
        app.global_shortcut()
            .unregister(INVOCATION)
            .map_err(|e| format!("Could not disable Ctrl+`: {e}"))?;
    }
    hold::set_enabled(enabled);
    Ok(())
}

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
    source_app: Option<&'static shortcuts::Profile>,
    operation: Option<&'static str>,
    ticket: u64,
    editor: Option<String>,
    status: String,
    result: Option<String>,
    release: Option<Update>,
    download: Option<Vec<u8>>,
    update_message: String,
    updating: bool,
    progress: u8,
    jobs: BTreeMap<u64, Job>,
    engines: pieall::engines::Engines,
}

struct Job {
    view: JobView,
    report: Option<String>,
    cancel: Option<Arc<AtomicBool>>,
    dismissed: bool,
}
impl Job {
    fn running(&self) -> bool {
        matches!(self.view.state, "running" | "cancelling")
    }
}

#[derive(Clone, Serialize)]
struct JobView {
    #[serde(skip)]
    action: String,
    id: u64,
    title: String,
    detail: String,
    completed: usize,
    total: usize,
    state: &'static str,
    fraction: Option<f64>,
    output: Option<PathBuf>,
    has_report: bool,
}

#[derive(Serialize)]
struct FieldView {
    key: &'static str,
    label: &'static str,
    default: &'static str,
    options: &'static [(&'static str, &'static str)],
}
#[derive(Serialize)]
struct ActionView {
    id: &'static str,
    title: &'static str,
    group: &'static str,
    eligible: bool,
    fields: Vec<FieldView>,
    shortcut: Option<&'static str>,
    category: Option<&'static str>,
    priority: Option<usize>,
    unavailable: Option<&'static str>,
}
#[derive(Serialize)]
struct UpdateView {
    busy: bool,
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
    app: Option<&'static str>,
    /// The captured window is always-on-top, so its pin slice offers Unpin.
    pinned: bool,
    busy: Option<&'static str>,
    editor: Option<String>,
    status: String,
    result: Option<String>,
    actions: Vec<ActionView>,
    update: UpdateView,
    job: Option<JobView>,
    jobs: Vec<JobView>,
}

impl Session {
    fn browser_source(&self) -> bool {
        self.source != 0
            && self
                .source_app
                .is_some_and(|p| p.sets.iter().any(|s| s == "browser" || s == "firefox"))
    }
    fn cancel_capture(&mut self) {
        if self.operation == Some("capture") {
            self.ticket += 1;
            self.operation = None;
        }
    }
    fn idle(&self) -> Result<()> {
        if self.operation.is_some() || self.editor.is_some() {
            Err("Finish the current action or close its editor first.".into())
        } else {
            Ok(())
        }
    }
    fn cancel_job(&mut self, id: u64) -> Result<()> {
        let job = self
            .jobs
            .get_mut(&id)
            .filter(|j| j.running())
            .ok_or("This job has already finished.")?;
        if let Some(cancel) = &job.cancel {
            cancel.store(true, Ordering::SeqCst);
        }
        job.view.state = "cancelling";
        Ok(())
    }
    fn finish_job(&mut self, ticket: u64, revision: u64, id: &str, result: &Result<String>) {
        if self.revision == revision && self.ticket == ticket {
            match result {
                Ok(report) => {
                    self.result = Some(report.clone());
                    self.status = "Action complete. Original files preserved.".into();
                }
                Err(error) => self.status = error.clone(),
            }
        }
        if let Some(job) = self.jobs.get_mut(&ticket) {
            job.cancel = None;
            job.report = Some(result.as_ref().map_or_else(Clone::clone, Clone::clone));
            let job = &mut job.view;
            job.has_report = true;
            job.output = result
                .as_ref()
                .ok()
                .filter(|_| pieall::downloads::is_download(id))
                .and_then(|report| download_output(report));
            job.fraction = None;
            job.state = match result {
                Ok(report) if report.contains("FAILED ") => "partial",
                Ok(_) => "complete",
                Err(e) if e.starts_with("Cancelled.") => "cancelled",
                Err(_) => "failed",
            };
            if result.is_ok() {
                job.completed = job.total;
            }
            job.detail = result.as_ref().map_or_else(
                |e| e.clone(),
                |r| r.lines().next().unwrap_or("Complete").into(),
            );
        }
    }
    fn quiescent(&self) -> Result<()> {
        self.idle()?;
        if self.updating || self.jobs.values().any(Job::running) {
            return Err("Wait for running jobs to finish or cancel them first.".into());
        }
        Ok(())
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
                    self.source_app
                        .map_or("Selected text", |profile| profile.name.as_str())
                } else {
                    "Clipboard text"
                },
                self.text.chars().count()
            ),
            0 if self.source != 0 => self
                .source_app
                .map_or("Active window", |p| p.name.as_str())
                .into(),
            0 => "Choose your files".into(),
            1 => names[0].clone(),
            n => format!("{n} selected files"),
        };
        View {
            job: self.jobs.values().next_back().map(|j| j.view.clone()),
            jobs: self.jobs.values().rev().map(|j| j.view.clone()).collect(),
            revision: self.revision,
            target,
            app: self.source_app.map(|p| p.id.as_str()),
            pinned: self.source != 0
                && unsafe { GetWindowLongPtrW(self.source as _, GWL_EXSTYLE) }
                    & WS_EX_TOPMOST as isize
                    != 0,
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
                    eligible: (actions::eligible(&a, &self.paths, !self.text.is_empty())
                        || (pieall::downloads::is_download(a.id) && self.browser_source()))
                        && (a.input != actions::InputKind::Window || self.source != 0),
                    id: a.id,
                    title: a.title,
                    group: a.group,
                    shortcut: None,
                    category: None,
                    priority: None,
                    unavailable: None,
                    fields: a
                        .fields
                        .into_iter()
                        .map(|f| FieldView {
                            key: f.key,
                            label: f.label,
                            default: f.default,
                            options: f.options,
                        })
                        .collect(),
                })
                .chain(self.source_app.into_iter().flat_map(|profile| {
                    shortcuts::for_profile(&profile.id).map(|a| ActionView {
                        id: &a.id,
                        title: &a.title,
                        group: "Apps",
                        eligible: true,
                        fields: vec![],
                        shortcut: Some(&a.keys),
                        category: Some(&a.category),
                        priority: a.priority,
                        unavailable: None,
                    })
                }))
                .collect(),
            update: UpdateView {
                busy: self.updating,
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

// Background jobs own their lifetime independently of the interactive selection lease.
struct JobGuard {
    app: AppHandle,
    id: u64,
}
impl Drop for JobGuard {
    fn drop(&mut self) {
        let state = state(&self.app);
        let mut s = state.lock().unwrap();
        if let Some(job) = s.jobs.get_mut(&self.id) {
            if job.running() {
                job.view.state = "failed";
                job.view.detail = "The action stopped unexpectedly. Try again.".into();
            }
            job.cancel = None;
        }
        drop(s);
        notify(&self.app);
    }
}

struct UpdateGuard {
    app: AppHandle,
    install: Option<Lease>,
}
impl Drop for UpdateGuard {
    fn drop(&mut self) {
        state(&self.app).lock().unwrap().updating = false;
        drop(self.install.take());
        notify(&self.app);
    }
}

static ENGINE_SETUP: Mutex<()> = Mutex::new(());
fn engine_setup(cancel: &AtomicBool) -> Result<std::sync::MutexGuard<'static, ()>> {
    loop {
        if cancel.load(Ordering::SeqCst) {
            return Err("Cancelled. Tool setup stopped.".into());
        }
        match ENGINE_SETUP.try_lock() {
            Ok(guard) => return Ok(guard),
            Err(std::sync::TryLockError::WouldBlock) => {
                std::thread::sleep(std::time::Duration::from_millis(50))
            }
            Err(_) => return Err("Tool setup stopped unexpectedly. Restart PieAll.".into()),
        }
    }
}

#[tauri::command]
fn session(window: WebviewWindow, app: AppHandle) -> Result<View> {
    local(&window)?;
    Ok(state(&app).lock().unwrap().view())
}

/// Hiding the dashboard abandons its open editor; otherwise every later invoke is refused as busy.
fn close_window(app: &AppHandle, window: &tauri::Window) -> Result<()> {
    if window.label() == "dashboard" && state(app).lock().unwrap().editor.take().is_some() {
        notify(app);
    }
    window.hide().map_err(|e| e.to_string())
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
const OUTRO_MS: u64 = 60;
static RING_OPEN: AtomicBool = AtomicBool::new(false);
/// Selection reads still running. A cancelled read keeps going until its Ctrl+C
/// settles and the clipboard is restored; keys sent before then could interleave.
static CAPTURING: AtomicU64 = AtomicU64::new(0);
struct Capturing;
impl Capturing {
    fn start() -> Self {
        CAPTURING.fetch_add(1, Ordering::SeqCst);
        Self
    }
    /// Waits (bounded) for every selection read to finish touching the clipboard.
    fn wait() {
        for _ in 0..200 {
            if CAPTURING.load(Ordering::SeqCst) == 0 {
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
    }
}
impl Drop for Capturing {
    fn drop(&mut self) {
        CAPTURING.fetch_sub(1, Ordering::SeqCst);
    }
}
static RING_GENERATION: AtomicU64 = AtomicU64::new(0);
/// Generation of the ring that last gained focus. Focus loss only closes that ring;
/// Windows delivers a hidden ring's focus loss late, after the next one has opened.
static RING_FOCUSED: AtomicU64 = AtomicU64::new(u64::MAX);
/// The backtick is physically down. While held, only its release closes the ring.
static RING_HELD: AtomicBool = AtomicBool::new(false);
/// Generation of the ring whose hold is confirmed (not a typed backtick). Text
/// capture sends Ctrl+C, so it waits for this.
static RING_CONFIRMED: AtomicU64 = AtomicU64::new(u64::MAX);
/// Physical screen point of the last ring; selects the monitor for notices.
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
    let generation = RING_GENERATION.load(Ordering::SeqCst);
    let _ = app.run_on_main_thread(move || {
        // Never focus a ring that closed meanwhile: set_focus would show it again
        // behind PieAll's back, leaving a ring nothing can close.
        if !RING_OPEN.load(Ordering::SeqCst) || RING_GENERATION.load(Ordering::SeqCst) != generation
        {
            return;
        }
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
    // Native calls only: Tauri's window getters/setters hop to the main thread,
    // which delayed the ring and could show it before a queued move landed.
    let monitor = unsafe { MonitorFromPoint(point, MONITOR_DEFAULTTONEAREST) };
    let (mut dpi_x, mut dpi_y) = (96u32, 96u32);
    unsafe {
        GetDpiForMonitor(monitor, MDT_EFFECTIVE_DPI, &mut dpi_x, &mut dpi_y);
    }
    let scale = dpi_x as f64 / 96.0;
    let size = (440.0 * scale).round() as i32;
    let mut info: MONITORINFO = unsafe { std::mem::zeroed() };
    info.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
    unsafe {
        GetMonitorInfoW(monitor, &mut info);
    }
    let work = info.rcWork;
    let x = (point.x - size / 2).clamp(work.left, (work.right - size).max(work.left));
    let y = (point.y - size / 2).clamp(work.top, (work.bottom - size).max(work.top));
    let hwnd = ring.hwnd().map_err(|e| e.to_string())?.0 as HWND;
    unsafe {
        SetWindowPos(hwnd, HWND_TOPMOST, x, y, size, size, SWP_NOACTIVATE);
    }
    RING_X.store(x + size / 2, Ordering::SeqCst);
    RING_Y.store(y + size / 2, Ordering::SeqCst);
    RING_GENERATION.fetch_add(1, Ordering::SeqCst);
    RING_OPEN.store(true, Ordering::SeqCst);
    // Click-mode opens clear a stale hold, so focus loss can still close them.
    RING_HELD.store(held, Ordering::SeqCst);
    let _ = app.emit_to("ring", "ring-opened", held);
    unsafe {
        // Undecorated windows can keep caption/frame styles, which Windows paints as a
        // pale band once a window region is set. Strip them before clipping.
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
        // The region is in window coordinates but the content starts at the client
        // origin, so center it on the client area; any leftover frame stays clipped.
        let mut client = POINT { x: 0, y: 0 };
        let mut frame: RECT = std::mem::zeroed();
        ClientToScreen(hwnd, &mut client);
        GetWindowRect(hwnd, &mut frame);
        let (dx, dy) = (client.x - frame.left, client.y - frame.top);
        let inset = (17.0 * scale).round() as i32;
        let region =
            CreateEllipticRgn(dx + inset, dy + inset, dx + size - inset, dy + size - inset);
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
        // Already closed as far as PieAll knows; make sure nothing is still on screen.
        hide_raw(&ring);
        return;
    }
    // Native calls, no main-thread hop: the release should act immediately.
    if let Ok(hwnd) = ring.hwnd() {
        let hwnd = hwnd.0 as HWND;
        let mut point = POINT { x: 0, y: 0 };
        let scale = unsafe {
            GetCursorPos(&mut point);
            ScreenToClient(hwnd, &mut point);
            GetDpiForWindow(hwnd).max(96) as f64 / 96.0
        };
        let (x, y) = (point.x as f64 / scale, point.y as f64 / scale);
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

/// True over apps where ` is a menu key, not text: a shortcut profile that never
/// reads selected text (Figma, Explorer and the desktop, Adobe apps). Terminals
/// are excluded because shells use ` as an escape character.
fn instant_ring() -> bool {
    let hwnd = unsafe { GetForegroundWindow() } as usize;
    if hwnd == 0 || platform::is_terminal_window(hwnd) {
        return false;
    }
    let mut pid = 0;
    unsafe { GetWindowThreadProcessId(hwnd as _, &mut pid) };
    platform::window_executable(hwnd, pid)
        .ok()
        .and_then(|exe| shortcuts::for_executable(&exe))
        .is_some_and(|profile| !profile.capture_text)
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
        s.source_app = None;
        if source != 0 {
            unsafe {
                GetWindowThreadProcessId(source as _, &mut s.source_pid);
            }
            s.source_app = platform::window_profile(source, s.source_pid)
                .ok()
                .flatten();
        }
        s.status = "Reading selection…".into();
    }
    notify(app);
    place_ring(app, held)?;
    let shell = source != 0 && platform::is_shell_window(source);
    let skip_text = state(app)
        .lock()
        .unwrap()
        .source_app
        .is_some_and(|p| !p.capture_text);
    let wants_text = source != 0 && !shell && !skip_text && !platform::is_terminal_window(source);
    // While ` is held the ring never takes focus: the app keeps it, so shortcuts
    // and results land without a focus round-trip, and the hook handles Escape.
    if !wants_text && !held {
        // Explorer is read over COM, which does not need its window focused.
        focus_ring(app);
    }
    let owner = app
        .get_webview_window("ring")
        .and_then(|w| w.hwnd().ok())
        .map_or(0, |h| h.0 as usize);
    let generation = RING_GENERATION.load(Ordering::SeqCst);
    let app = app.clone();
    // Reserve before scheduling: an immediate action must also wait for a queued capture.
    let capturing = wants_text.then(Capturing::start);
    tauri::async_runtime::spawn_blocking(move || {
        let _capturing = capturing;
        // A held ` may still turn out to be typing; never send Ctrl+C for that.
        // A tap closes the ring, which bumps the ticket and ends the wait.
        if held && wants_text {
            let ticket = || state(&app).lock().unwrap().ticket;
            for _ in 0..400 {
                if ticket() != lease.ticket {
                    return;
                }
                if RING_CONFIRMED.load(Ordering::SeqCst) == generation {
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(5));
            }
        }
        let result = if shell {
            platform::capture_selection_from(source).map(|paths| (paths, None))
        } else if wants_text {
            let ticket = lease.ticket;
            Ok((
                Vec::new(),
                platform::copy_selection(owner, || state(&app).lock().unwrap().ticket != ticket),
            ))
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
                    s.status = s.source_app.map_or_else(
                        || "Choose an action. Your originals stay untouched.".into(),
                        |profile| format!("Choose an action for {}.", profile.name),
                    );
                }
                Err(error) => {
                    s.status = format!("{error} Window actions remain available.");
                }
            }
            true
        };
        drop(lease);
        if current && wants_text && !held {
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
        "search" => {
            show_dashboard(&app)?;
            app.emit_to("dashboard", "search-actions", ())
                .map_err(|e| e.to_string())
        }
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
                close_window(&app, &window.as_ref().window())
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
        s.source_app = None;
        s.status = "Selection ready. Choose an action.".into();
        Ok(())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[derive(Serialize)]
struct EngineView {
    name: &'static str,
    path: Option<String>,
    sha256: Option<String>,
}
fn settings_path(app: &AppHandle) -> Result<PathBuf> {
    Ok(app
        .path()
        .app_local_data_dir()
        .map_err(|e| e.to_string())?
        .join("settings.sqlite3"))
}
fn download_defaults(app: &AppHandle) -> Result<pieall::downloads::Settings> {
    pieall::downloads::load(
        &settings_path(app)?,
        &app.path().download_dir().map_err(|e| e.to_string())?,
    )
}
#[tauri::command]
fn download_settings(window: WebviewWindow, app: AppHandle) -> Result<pieall::downloads::Settings> {
    dashboard_only(&window)?;
    download_defaults(&app)
}
#[tauri::command]
async fn configure_downloads(
    window: WebviewWindow,
    app: AppHandle,
    preferences: pieall::downloads::Preferences,
    choose_folder: bool,
) -> Result<pieall::downloads::Settings> {
    dashboard_only(&window)?;
    preferences.validate()?;
    let lease = Lease::acquire(&app, "download setup")?;
    notify(&app);
    tauri::async_runtime::spawn_blocking(move || {
        let _lease = lease;
        let mut settings = download_defaults(&app)?;
        settings.preferences = preferences;
        if choose_folder {
            let Some(folder) = platform::pick_download_folder()? else {
                return download_defaults(&app);
            };
            settings.destination = folder;
        }
        pieall::downloads::store(&settings_path(&app)?, &settings)?;
        Ok(settings)
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
async fn download_url(
    window: WebviewWindow,
    app: AppHandle,
    id: String,
    url: String,
    preferences: pieall::downloads::Preferences,
) -> Result<()> {
    dashboard_only(&window)?;
    if !pieall::downloads::is_download(&id) {
        return Err("Unknown download action.".into());
    }
    preferences.validate()?;
    let url = pieall::web_proxy::web_url(&url)?.to_string();
    let revision = {
        let state = state(&app);
        let mut s = state.lock().unwrap();
        s.idle()?;
        s.replace(vec![], url);
        s.source = 0;
        s.source_pid = 0;
        s.source_app = None;
        s.revision
    };
    let params = BTreeMap::from([
        ("quality".into(), preferences.quality),
        ("container".into(), preferences.container),
        ("audio".into(), preferences.audio),
    ]);
    run_action(window, app, id, revision, params).await
}
#[tauri::command]
fn tool_engines(window: WebviewWindow, app: AppHandle) -> Result<Vec<EngineView>> {
    dashboard_only(&window)?;
    let s = state(&app);
    let s = s.lock().unwrap();
    Ok(pieall::engines::NAMES
        .iter()
        .map(|name| {
            let engine = s.engines.get(*name);
            EngineView {
                name,
                path: engine.map(|e| e.path.to_string_lossy().into_owned()),
                sha256: engine.map(|e| e.sha256.clone()),
            }
        })
        .collect())
}
#[tauri::command]
async fn setup_tools(window: WebviewWindow, app: AppHandle) -> Result<()> {
    dashboard_only(&window)?;
    let revision = state(&app).lock().unwrap().revision;
    run_action(window, app, "setup_tools".into(), revision, BTreeMap::new()).await
}
#[tauri::command]
async fn configure_engine(
    window: WebviewWindow,
    app: AppHandle,
    name: String,
    remove: bool,
) -> Result<()> {
    dashboard_only(&window)?;
    if !pieall::engines::NAMES.contains(&name.as_str()) {
        return Err("Unknown engine.".into());
    }
    let lease = Lease::acquire(&app, "engine setup")?;
    notify(&app);
    tauri::async_runtime::spawn_blocking(move || {
        let _lease = lease;
        let _setup = ENGINE_SETUP
            .try_lock()
            .map_err(|_| "Tools are being set up. Change custom engines after setup finishes.")?;
        let engine = if remove {
            None
        } else {
            let paths = platform::pick_engine()?;
            let Some(path) = paths.first() else {
                return Ok(());
            };
            Some(pieall::engines::Engine::select(&name, path)?)
        };
        pieall::engines::store(&settings_path(&app)?, &name, engine.as_ref())?;
        let s = state(&app);
        let mut s = s.lock().unwrap();
        match engine {
            Some(engine) => {
                s.engines.insert(name.clone(), engine);
            }
            None => {
                s.engines.remove(&name);
            }
        }
        s.status = format!(
            "{} {}.",
            name,
            if remove { "removed" } else { "configured" }
        );
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
    if id == "setup_tools" {
        dashboard_only(&window)?;
    }
    if id.starts_with("app:") {
        if !params.is_empty() {
            return Err("App shortcuts do not accept parameters.".into());
        }
        return run_app_shortcut(window, app, id, revision).await;
    }
    if params.len() > 16 || params.iter().any(|(k, v)| k.len() > 64 || v.len() > 256) {
        return Err("Action parameters exceed limits.".into());
    }
    let download_settings = if pieall::downloads::is_download(&id) {
        let mut settings = download_defaults(&app)?;
        settings.preferences = settings.preferences.with_overrides(&params)?;
        Some(settings)
    } else {
        None
    };
    let from_ring = window.label() == "ring";
    let lease;
    let cancel = Arc::new(AtomicBool::new(false));
    let (paths, text, selection, source, browser_source) = {
        let s = state(&app);
        let mut s = s.lock().unwrap();
        if revision != s.revision {
            return Err("Selection changed. Choose the action again.".into());
        }
        let action = actions::catalog()
            .into_iter()
            .find(|a| a.id == id)
            .or_else(|| {
                (id == "setup_tools").then(|| actions::Action {
                    id: "setup_tools",
                    title: "Set up tools",
                    group: "Tools",
                    input: actions::InputKind::Text,
                    fields: vec![],
                })
            })
            .ok_or("Unknown action")?;
        let browser_source = pieall::downloads::is_download(&id) && s.browser_source();
        if !(actions::eligible(&action, &s.paths, !s.text.is_empty())
            || browser_source
            || id == "setup_tools")
            || (action.input == actions::InputKind::Window && s.source == 0)
        {
            return Err("This action does not support the selected target.".into());
        }
        // Window actions don't need the selection: a release mid-read runs at once.
        if action.input == actions::InputKind::Window || browser_source {
            s.cancel_capture();
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
        // Keep recent results bounded without dropping any active job.
        while s.jobs.values().filter(|j| !j.running()).count() >= 20 {
            let oldest = *s.jobs.iter().find(|(_, j)| !j.running()).unwrap().0;
            s.jobs.remove(&oldest);
        }
        let job = JobView {
            action: id.clone(),
            id: s.ticket,
            title: action.title.into(),
            detail: "Starting…".into(),
            completed: 0,
            total: s.paths.len().max(1),
            state: "running",
            fraction: None,
            output: None,
            has_report: false,
        };
        let ticket = s.ticket;
        s.jobs.insert(
            ticket,
            Job {
                view: job,
                report: None,
                cancel: Some(cancel.clone()),
                dismissed: false,
            },
        );
        let selection = if from_ring { s.selection.take() } else { None };
        (
            s.paths.clone(),
            s.text.clone(),
            selection,
            s.source,
            browser_source.then_some((s.source, s.source_pid)),
        )
    };
    let guard = JobGuard {
        app: app.clone(),
        id: lease.ticket,
    };
    let job_revision = revision;
    // Only actions that touch foreground input/clipboard keep the short interactive lease.
    let input_action =
        matches!(id.as_str(), "copy_paths" | "copy_pixels") || params.contains_key("source_hwnd");
    notify(&app);
    if from_ring {
        hide_ring(&app, true)?;
        // Most actions finish before a progress notice would be worth seeing.
        let slow = app.clone();
        let ticket = lease.ticket;
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(700));
            if shows_progress(&slow, ticket) {
                show_job_toast(&slow, ticket);
            }
        });
    } else if let Some((source, _)) = browser_source {
        window.hide().map_err(|e| e.to_string())?;
        unsafe {
            SetForegroundWindow(source as _);
        }
    } else {
        show_dashboard(&app)?;
    }
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = guard;
        let mut input_lease = Some(lease);
        Capturing::wait();
        let started = std::time::Instant::now();
        let ticket = _guard.id;
        let progress = |p: pieall::jobs::Progress| {
            {
                let state = state(&app);
                let mut s = state.lock().unwrap();
                let status = if let Some(job) = s.jobs.get_mut(&ticket).map(|j| &mut j.view) {
                    job.completed = p.completed.min(p.total);
                    job.total = p.total;
                    job.detail = p.item;
                    job.fraction = p
                        .fraction
                        .filter(|f| f.is_finite())
                        .map(|f| f.clamp(0., 1.));
                    Some(format!(
                        "{} · {} of {} · {}",
                        job.title, job.completed, job.total, job.detail
                    ))
                } else {
                    None
                };
                if let Some(status) =
                    status.filter(|_| s.ticket == ticket && s.revision == job_revision)
                {
                    s.status = status;
                }
            }
            notify(&app);
            if started.elapsed().as_millis() >= 700 && shows_progress(&app, ticket) {
                show_job_toast(&app, ticket);
            }
        };
        let result = (|| {
            if cancel.load(Ordering::SeqCst) {
                return Err("Cancelled. Nothing was started.".into());
            }
            let text = match browser_source {
                Some((hwnd, pid)) => platform::copy_browser_url(hwnd, pid)?,
                None => text,
            };
            if pieall::downloads::is_download(&id) {
                pieall::web_proxy::web_url(&text)?;
            }
            if !input_action {
                drop(input_lease.take());
            }
            let mut engines = state(&app).lock().unwrap().engines.clone();
            let needs_engines = pieall::downloads::is_download(&id)
                || actions::media_action(&id)
                || id == "setup_tools";
            let setup = if needs_engines {
                Some(engine_setup(&cancel)?)
            } else {
                None
            };
            if needs_engines {
                engines = state(&app).lock().unwrap().engines.clone();
            }
            pieall::engine_install::ensure(
                &settings_path(&app)?,
                &id,
                &mut engines,
                &cancel,
                &progress,
            )?;
            if needs_engines {
                state(&app).lock().unwrap().engines = engines.clone();
            }
            drop(setup);
            if id == "setup_tools" {
                Ok("Tools ready. Video, audio and downloads are available.".into())
            } else if let Some(settings) = download_settings {
                actions::execute_download(&id, &text, &settings, &engines, &cancel, progress)
            } else {
                actions::execute_with_engines(
                    &id, &paths, &text, &params, &engines, &cancel, progress,
                )
            }
        })();
        {
            let s = state(&app);
            let mut s = s.lock().unwrap();
            s.finish_job(ticket, job_revision, &id, &result);
        }
        let dismissed = state(&app)
            .lock()
            .unwrap()
            .jobs
            .get(&ticket)
            .is_some_and(|j| j.dismissed);
        drop(input_lease.take());
        if from_ring {
            finish_in_place(&app, &id, result, selection, source, ticket, job_revision);
        } else if !from_ring && !dismissed {
            show_job_toast(&app, ticket);
        }
    });
    Ok(())
}

#[derive(Serialize)]
struct ShortcutLibrary {
    profiles: &'static [shortcuts::Profile],
    actions: &'static [shortcuts::Shortcut],
}
#[tauri::command]
fn set_trigger_key(window: WebviewWindow, vk: u32) -> Result<()> {
    dashboard_only(&window)?;
    hold::set_trigger(vk)
}
#[tauri::command]
fn autostart_enabled(window: WebviewWindow) -> Result<bool> {
    dashboard_only(&window)?;
    crate::autostart::enabled()
}
#[tauri::command]
fn set_autostart(window: WebviewWindow, enabled: bool) -> Result<()> {
    dashboard_only(&window)?;
    crate::autostart::set_enabled(enabled)
}
#[tauri::command]
fn shortcut_library(window: WebviewWindow) -> Result<ShortcutLibrary> {
    dashboard_only(&window)?;
    let catalog = shortcuts::catalog();
    Ok(ShortcutLibrary {
        profiles: &catalog.profiles,
        actions: &catalog.actions,
    })
}

async fn run_app_shortcut(
    window: WebviewWindow,
    app: AppHandle,
    id: String,
    revision: u64,
) -> Result<()> {
    let action = shortcuts::find(&id).ok_or("Unknown app shortcut")?;
    let (source, pid, lease) = {
        let state = state(&app);
        let mut s = state.lock().unwrap();
        if revision != s.revision {
            return Err("Selection changed. Invoke PieAll again.".into());
        }
        // App shortcuts don't need the selection: a release mid-read runs at once.
        s.cancel_capture();
        s.idle()?;
        if s.source_app
            .is_none_or(|profile| profile.id != action.profile)
            || s.source == 0
        {
            return Err("Invoke PieAll from the app this shortcut belongs to.".into());
        }
        // The action ID selects bundled keys. IPC cannot supply keys, HWND, PID or app identity.
        if platform::window_profile(s.source, s.source_pid)?.is_none_or(|p| p.id != action.profile)
        {
            return Err("The source app changed. Invoke PieAll again.".into());
        }
        s.ticket += 1;
        s.operation = Some("shortcut");
        s.result = None;
        s.status = format!("Sending {}…", action.title);
        (
            s.source,
            s.source_pid,
            Lease {
                app: app.clone(),
                ticket: s.ticket,
            },
        )
    };
    notify(&app);
    hide_ring(&app, true)?;
    if window.label() == "dashboard" {
        window.hide().map_err(|error| error.to_string())?;
        unsafe {
            SetForegroundWindow(source as _);
        }
    }
    tauri::async_runtime::spawn_blocking(move || {
        let _lease = lease;
        Capturing::wait();
        let result = dispatch_app_shortcut(&app, source, pid, action);
        let (tone, status) = match result {
            Ok(()) => (
                "ok",
                format!("Shortcut sent: {} · {}", action.title, action.keys),
            ),
            Err(error) => ("error", error),
        };
        {
            let state = state(&app);
            let mut s = state.lock().unwrap();
            // A shortcut can change the selection or document. Never reuse its old captured content.
            s.replace(Vec::new(), String::new());
            s.status = status.clone();
        }
        toast(&app, tone, status, "");
    })
    .await
    .map_err(|error| error.to_string())
}

fn dispatch_app_shortcut(
    app: &AppHandle,
    source: usize,
    pid: u32,
    action: &shortcuts::Shortcut,
) -> Result<()> {
    let collision = shortcuts::uses_invocation_shortcut(action)
        && app.global_shortcut().is_registered(INVOCATION);
    if !collision {
        return platform::send_app_shortcut(source, pid, action);
    }
    app.global_shortcut()
        .unregister(INVOCATION)
        .map_err(|e| format!("Could not release PieAll's shortcut: {e}"))?;
    let result = platform::send_app_shortcut(source, pid, action);
    // RegisterHotKey would swallow injected Ctrl+` too. Keep it free while input is dispatched.
    std::thread::sleep(std::time::Duration::from_millis(80));
    let restore = if hold::enabled() && !app.global_shortcut().is_registered(INVOCATION) {
        app.global_shortcut().register(INVOCATION)
    } else {
        Ok(())
    };
    match (result, restore) {
        (result, Ok(())) => result,
        (Ok(()), Err(error)) => Err(format!(
            "Shortcut sent, but Ctrl+` could not be restored: {error}. Hold ` or use the tray."
        )),
        (Err(error), Err(restore)) => Err(format!(
            "{error} Ctrl+` could not be restored: {restore}. Hold ` or use the tray."
        )),
    }
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
    id: u64,
    tone: &'static str,
    title: String,
    detail: String,
    job: Option<JobView>,
}
static TOAST_GENERATION: AtomicU64 = AtomicU64::new(0);
static TOAST_GATE: std::sync::Mutex<Option<u64>> = std::sync::Mutex::new(None);

#[tauri::command]
fn dismiss_toast(window: WebviewWindow, app: AppHandle, id: u64, job: Option<u64>) -> Result<()> {
    if window.label() != "toast" {
        return Err("Unknown surface.".into());
    }
    let mut current_job = TOAST_GATE.lock().unwrap();
    // Progress can advance the generation between rendering and clicking Close.
    // The job identity remains stable; an old notice cannot dismiss a different job.
    if TOAST_GENERATION.load(Ordering::SeqCst) == id || job.is_some() && *current_job == job {
        TOAST_GENERATION.fetch_add(1, Ordering::SeqCst);
        if let Some(job) = job {
            if let Some(job) = state(&app).lock().unwrap().jobs.get_mut(&job) {
                job.dismissed = true;
            }
        }
        hide_raw(&window);
        *current_job = None;
    }
    Ok(())
}

#[tauri::command]
fn open_download_location(window: WebviewWindow, app: AppHandle, id: u64) -> Result<()> {
    if !matches!(window.label(), "dashboard" | "toast") {
        return Err("Unknown surface.".into());
    }
    let path = {
        let s = state(&app);
        let s = s.lock().unwrap();
        s.jobs
            .get(&id)
            .map(|j| &j.view)
            .filter(|j| j.state == "complete" && pieall::downloads::is_download(&j.action))
            .and_then(|j| j.output.clone())
            .ok_or("This download is no longer available.")?
    };
    pieall::output::plain_path(&path)?;
    actions::execute("reveal", &[path], "", &BTreeMap::new())?;
    Ok(())
}

#[tauri::command]
fn job_report(window: WebviewWindow, app: AppHandle, id: u64) -> Result<String> {
    dashboard_only(&window)?;
    state(&app)
        .lock()
        .unwrap()
        .jobs
        .get(&id)
        .and_then(|j| j.report.clone())
        .ok_or("This result is no longer available.")
        .map_err(Into::into)
}

#[tauri::command]
fn cancel_job(window: WebviewWindow, app: AppHandle, id: u64) -> Result<()> {
    if !matches!(window.label(), "dashboard" | "toast") {
        return Err("Unknown surface.".into());
    }
    let state = state(&app);
    let mut s = state.lock().unwrap();
    s.cancel_job(id)?;
    s.status = "Cancelling… Completed outputs will be kept.".into();
    drop(s);
    notify(&app);
    show_job_toast(&app, id);
    Ok(())
}

/// Progress UI is for real progress work: several files, or one file that reports
/// how far along it is (media, PDF). One-shot actions (open a terminal, copy a
/// path) only ever get the result notice, however long the call took.
fn shows_progress(app: &AppHandle, ticket: u64) -> bool {
    let s = state(app);
    let s = s.lock().unwrap();
    s.jobs.get(&ticket).is_some_and(|job| {
        let j = &job.view;
        j.id == ticket
            && j.state == "running"
            && j.completed < j.total
            && (j.total > 1
                || j.fraction.is_some()
                || pieall::downloads::is_download(&j.action)
                || actions::media_action(&j.action)
                || j.action == "setup_tools")
    })
}

fn show_job_toast(app: &AppHandle, ticket: u64) {
    let s = state(app);
    let s = s.lock().unwrap();
    let Some(job) = s
        .jobs
        .get(&ticket)
        .filter(|j| !j.dismissed)
        .map(|j| &j.view)
    else {
        return;
    };
    let job = job.clone();
    drop(s);
    let tone = match job.state {
        "running" | "cancelling" => "busy",
        "failed" | "partial" => "error",
        "cancelled" => "info",
        _ => "ok",
    };
    show_notice(app, tone, job.title.clone(), job.detail.clone(), Some(job));
}

/// A nonactivating notice with explicit dismiss and cancellation controls.
fn toast(app: &AppHandle, tone: &'static str, title: impl Into<String>, detail: impl Into<String>) {
    show_notice(app, tone, title.into(), detail.into(), None);
}
fn dashboard_active(app: &AppHandle) -> bool {
    app.get_webview_window("dashboard")
        .and_then(|w| w.hwnd().ok())
        .is_some_and(|hwnd| std::ptr::eq(unsafe { GetForegroundWindow() }, hwnd.0))
}
fn hide_job_notice(app: &AppHandle) {
    let mut current = TOAST_GATE.lock().unwrap();
    if current.take().is_some() {
        TOAST_GENERATION.fetch_add(1, Ordering::SeqCst);
        if let Some(window) = app.get_webview_window("toast") {
            hide_raw(&window);
        }
    }
}
fn notice_corner(work: RECT, width: i32, height: i32, margin: i32) -> (i32, i32) {
    (
        (work.right - width - margin).max(work.left),
        (work.bottom - height - margin).max(work.top),
    )
}
fn show_notice(
    app: &AppHandle,
    mut tone: &'static str,
    mut title: String,
    mut detail: String,
    mut job: Option<JobView>,
) {
    let mut current_job = TOAST_GATE.lock().unwrap();
    if let Some(snapshot) = &job {
        let s = state(app);
        let s = s.lock().unwrap();
        let Some(latest) = s
            .jobs
            .get(&snapshot.id)
            .filter(|j| !j.dismissed)
            .map(|j| &j.view)
        else {
            return;
        };
        tone = match latest.state {
            "running" | "cancelling" => "busy",
            "failed" | "partial" => "error",
            "cancelled" => "info",
            _ => "ok",
        };
        title = latest.title.clone();
        detail = latest.detail.clone();
        job = Some(latest.clone());
    }
    if tone == "busy" && current_job.is_some() && job.as_ref().map(|j| j.id) != *current_job {
        return;
    }
    let Some(window) = app.get_webview_window("toast") else {
        return;
    };
    let has_output = job.as_ref().is_some_and(|j| j.output.is_some());
    if job.is_some() && dashboard_active(app) {
        TOAST_GENERATION.fetch_add(1, Ordering::SeqCst);
        *current_job = None;
        hide_raw(&window);
        return;
    }
    let same_job = job.as_ref().is_some_and(|j| *current_job == Some(j.id));
    let generation = TOAST_GENERATION.fetch_add(1, Ordering::SeqCst) + 1;
    *current_job = job.as_ref().map(|j| j.id);
    let _ = app.emit_to(
        "toast",
        "toast",
        ToastView {
            id: generation,
            tone,
            title,
            detail,
            job,
        },
    );
    // A progress tick for the toast already on screen only repaints it: moving and
    // resizing the window on every tick made the bar stutter.
    let visible = window
        .hwnd()
        .is_ok_and(|h| unsafe { IsWindowVisible(h.0 as _) } != 0);
    if same_job && visible && tone == "busy" {
        return;
    }
    let point = POINT {
        x: RING_X.load(Ordering::SeqCst),
        y: RING_Y.load(Ordering::SeqCst),
    };
    let monitor = if same_job {
        window
            .hwnd()
            .ok()
            .map(|h| unsafe { MonitorFromWindow(h.0 as _, MONITOR_DEFAULTTONEAREST) })
    } else {
        None
    }
    .unwrap_or_else(|| unsafe {
        MonitorFromPoint(
            if point.x == i32::MIN {
                POINT { x: 0, y: 0 }
            } else {
                point
            },
            MONITOR_DEFAULTTONEAREST,
        )
    });
    let mut info: MONITORINFO = unsafe { std::mem::zeroed() };
    info.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
    if unsafe { GetMonitorInfoW(monitor, &mut info) } == 0 {
        return;
    }
    let (mut dpi_x, mut dpi_y) = (96, 96);
    unsafe {
        GetDpiForMonitor(monitor, MDT_EFFECTIVE_DPI, &mut dpi_x, &mut dpi_y);
    }
    let scale = dpi_x as f64 / 96.0;
    let (width, height) = (400.0, 164.0);
    let (w, h) = (
        (width * scale).round() as i32,
        (height * scale).round() as i32,
    );
    let (x, y) = notice_corner(info.rcWork, w, h, (16.0 * scale).round() as i32);
    let _ = window.set_size(tauri::LogicalSize::new(width, height));
    let _ = window.set_position(tauri::PhysicalPosition::new(x, y));
    show_inactive(&window);
    if tone != "busy" {
        let linger = if has_output {
            8000
        } else if tone == "error" {
            3200
        } else {
            1400
        };
        let app = app.clone();
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(linger));
            let gate = TOAST_GATE.lock().unwrap();
            if TOAST_GENERATION.load(Ordering::SeqCst) != generation {
                return;
            }
            let _ = app.emit_to("toast", "toast-hide", ());
            drop(gate);
            std::thread::sleep(std::time::Duration::from_millis(OUTRO_MS + 60));
            let mut gate = TOAST_GATE.lock().unwrap();
            if TOAST_GENERATION.load(Ordering::SeqCst) == generation {
                hide_raw(&window);
                *gate = None;
            }
        });
    }
}

/// Actions whose answer is text to keep; from the ring it goes to the clipboard.
const REPORTS: &[&str] = &[
    "media_details",
    "pdf_details",
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

fn download_output(report: &str) -> Option<PathBuf> {
    let path = PathBuf::from(report.lines().next()?.strip_prefix("Saved ")?);
    path.is_absolute().then_some(path)
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
fn action_toast(
    app: &AppHandle,
    ticket: u64,
    tone: &'static str,
    title: impl Into<String>,
    detail: impl Into<String>,
) {
    {
        let s = state(app);
        let mut s = s.lock().unwrap();
        let Some(job) = s.jobs.get_mut(&ticket).map(|j| &mut j.view) else {
            return;
        };
        job.title = title.into();
        job.detail = detail.into();
        if tone == "error" && job.state != "cancelled" {
            job.state = "failed";
        }
    }
    show_job_toast(app, ticket);
}
fn finish_in_place(
    app: &AppHandle,
    id: &str,
    result: Result<String>,
    selection: Option<platform::ClipboardSnapshot>,
    source: usize,
    ticket: u64,
    revision: u64,
) {
    let report = match result {
        Ok(report) => report,
        Err(error) => {
            let (_, title, detail) = summarize(&error, true);
            action_toast(app, ticket, "error", title, detail);
            return;
        }
    };
    if pieall::downloads::is_download(id) {
        let (tone, title, detail) = summarize(&report, false);
        action_toast(app, ticket, tone, title, detail);
        return;
    }
    let _input = if REPORTS.contains(&id) || selection.is_some() {
        match Lease::acquire(app, "clipboard") {
            Ok(lease) if state(app).lock().unwrap().revision == revision => {
                Capturing::wait();
                Some(lease)
            }
            _ => {
                action_toast(
                    app,
                    ticket,
                    "info",
                    "Result ready",
                    "View the result in PieAll's jobs list.",
                );
                return;
            }
        }
    } else {
        None
    };
    if REPORTS.contains(&id) {
        let body = report
            .split_once("\r\n\r\n")
            .filter(|_| report.starts_with("Completed "))
            .map_or(report.as_str(), |(_, body)| body);
        let preview: String = body.lines().take(2).collect::<Vec<_>>().join(" · ");
        match platform::set_clipboard_text(body) {
            Ok(()) => action_toast(app, ticket, "ok", "Copied to clipboard", preview),
            Err(error) => action_toast(app, ticket, "error", "Couldn't copy the result", error),
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
            action_toast(
                app,
                ticket,
                "info",
                "Result copied",
                "Focus moved before PieAll could paste. Press Ctrl+V to paste it.",
            );
        }
        return;
    }
    let (tone, title, detail) = summarize(&report, false);
    action_toast(app, ticket, tone, title, detail);
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
    let _update = {
        let state = state(app);
        let mut s = state.lock().unwrap();
        if s.updating {
            return Err("An update operation is already running.".into());
        }
        let install = if action == "install" {
            s.quiescent()?;
            s.ticket += 1;
            s.operation = Some("update");
            Some(Lease {
                app: app.clone(),
                ticket: s.ticket,
            })
        } else {
            None
        };
        s.updating = true;
        UpdateGuard {
            app: app.clone(),
            install,
        }
    };
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
    s.source_app = None;
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
                    if event.state == ShortcutState::Pressed && hold::enabled() {
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
            shortcut_library,
            set_trigger_key,
            autostart_enabled,
            set_autostart,
            copy_result,
            preview,
            update_action,
            dismiss_toast,
            open_download_location,
            cancel_job,
            job_report,
            tool_engines,
            configure_engine,
            download_settings,
            configure_downloads,
            download_url,
            setup_tools
        ])
        .setup(|app| {
            match settings_path(app.handle()).and_then(|p| pieall::engines::load(&p)) {
                Ok(engines) => state(app.handle()).lock().unwrap().engines = engines,
                Err(error) => message(app.handle(), format!("Tool settings unavailable: {error}")),
            }
            // Parse and index bundled keymaps once, before the first hold invocation.
            let _ = shortcuts::catalog();
            let cwd = std::env::current_dir()?.to_string_lossy().into_owned();
            if let Err(error) =
                accept_paths(app.handle(), file_args(std::env::args().skip(1), &cwd))
            {
                message(app.handle(), error);
            }
            if let Err(error) = app.global_shortcut().register(INVOCATION) {
                message(
                    app.handle(),
                    format!("Ctrl + ` is unavailable: {error}. Hold ` or use the tray."),
                );
            }
            // Windows 11 draws an accent-colour border on an active undecorated window;
            // clipped to the ring's circle it showed as a pale arc above the top slice
            // whenever a click activated the ring. Turn that border off for good.
            for label in ["ring", "toast"] {
                if let Some(hwnd) = app.get_webview_window(label).and_then(|w| w.hwnd().ok()) {
                    let none: u32 = DWMWA_COLOR_NONE;
                    unsafe {
                        DwmSetWindowAttribute(
                            hwnd.0 as _,
                            DWMWA_BORDER_COLOR as u32,
                            &none as *const u32 as _,
                            std::mem::size_of::<u32>() as u32,
                        );
                    }
                }
            }
            if let Some(toast_window) = app.get_webview_window("toast") {
                let _ = toast_window.set_ignore_cursor_events(false);
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
            // A Start that fails (busy) may still be a typed backtick; report it only
            // once the hold is confirmed, so typing never pops error toasts.
            let refused = Mutex::new(None::<String>);
            let hook = hold::install(
                move |event| {
                    let result = match event {
                        hold::Hold::Start => {
                            RING_HELD.store(true, Ordering::SeqCst);
                            *refused.lock().unwrap() = invoke_ring(&handle, true).err();
                            Ok(())
                        }
                        hold::Hold::Confirm => {
                            RING_CONFIRMED
                                .store(RING_GENERATION.load(Ordering::SeqCst), Ordering::SeqCst);
                            refused.lock().unwrap().take().map_or(Ok(()), Err)
                        }
                        hold::Hold::Tap => {
                            refused.lock().unwrap().take();
                            // Typing, not a hold: vanish at once, no fade, focus untouched.
                            RING_HELD.store(false, Ordering::SeqCst);
                            let result = hide_ring(&handle, false);
                            if let Some(ring) = handle.get_webview_window("ring") {
                                hide_raw(&ring);
                            }
                            result
                        }
                        hold::Hold::End => {
                            RING_HELD.store(false, Ordering::SeqCst);
                            release_ring(&handle);
                            Ok(())
                        }
                        hold::Hold::Cancel => hide_ring(&handle, true),
                    };
                    if let Err(error) = result {
                        toast(&handle, "error", "PieAll is busy", error);
                    }
                },
                instant_ring,
            );
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
                "Show action ring",
                true,
                None::<&str>,
            )?;
            let use_shortcut = tauri::menu::CheckMenuItem::with_id(
                app,
                "use-shortcut",
                "Use shortcut",
                true,
                true,
                None::<&str>,
            )?;
            let exit =
                tauri::menu::MenuItem::with_id(app, "exit", "Exit PieAll", true, None::<&str>)?;
            let menu = tauri::menu::Menu::with_items(app, &[&open, &ring, &use_shortcut, &exit])?;
            let pixels = include_bytes!("../icons/tray.rgba").to_vec();
            tauri::tray::TrayIconBuilder::new()
                .icon(tauri::image::Image::new_owned(pixels, 32, 32))
                .tooltip("PieAll · hold ` over a file, text or window")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(move |app, event| {
                    let result = match event.id.as_ref() {
                        "open" => show_dashboard(app),
                        "ring" => invoke_ring(app, false),
                        "use-shortcut" => {
                            let result = set_shortcuts_enabled(app, !hold::enabled());
                            // Native check items toggle before the callback; undo on failure.
                            let checked = use_shortcut
                                .set_checked(hold::enabled())
                                .map_err(|e| e.to_string());
                            result.and(checked)
                        }
                        "exit" => {
                            let idle = state(app).lock().unwrap().quiescent();
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
            // Click-through logo splash over the desktop, independent of the hidden dashboard; it has no IPC and closes itself.
            let splash = tauri::WebviewWindowBuilder::new(
                app,
                "splash",
                tauri::WebviewUrl::App("index.html?surface=splash".into()),
            )
            .title("PieAll")
            .inner_size(320.0, 320.0)
            .center()
            .decorations(false)
            .transparent(true)
            .shadow(false)
            .resizable(false)
            .always_on_top(true)
            .skip_taskbar(true)
            .focused(false)
            .build()?;
            splash.set_ignore_cursor_events(true)?;
            std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_millis(2500));
                let _ = splash.destroy();
            });
            Ok(())
        })
        .on_window_event(|window, event| {
            let app = window.app_handle();
            match event {
                tauri::WindowEvent::CloseRequested { api, .. } => {
                    api.prevent_close();
                    let _ = close_window(app, window);
                }
                tauri::WindowEvent::Focused(true) if window.label() == "dashboard" => {
                    hide_job_notice(app);
                }
                tauri::WindowEvent::Focused(true) if window.label() == "ring" => {
                    if RING_OPEN.load(Ordering::SeqCst) {
                        RING_FOCUSED
                            .store(RING_GENERATION.load(Ordering::SeqCst), Ordering::SeqCst);
                    }
                }
                tauri::WindowEvent::Focused(false)
                    if window.label() == "ring"
                        && !RING_HELD.load(Ordering::SeqCst)
                        && RING_FOCUSED.load(Ordering::SeqCst)
                            == RING_GENERATION.load(Ordering::SeqCst) =>
                {
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
    #[test]
    fn notices_anchor_to_work_area_corner_including_negative_monitors() {
        assert_eq!(
            notice_corner(
                RECT {
                    left: 0,
                    top: 0,
                    right: 1920,
                    bottom: 1040
                },
                400,
                164,
                16
            ),
            (1504, 860)
        );
        assert_eq!(
            notice_corner(
                RECT {
                    left: -1920,
                    top: -100,
                    right: 0,
                    bottom: 940
                },
                600,
                246,
                24
            ),
            (-624, 670)
        );
        assert_eq!(
            notice_corner(
                RECT {
                    left: 0,
                    top: 0,
                    right: 300,
                    bottom: 100
                },
                400,
                164,
                16
            ),
            (0, 0)
        );
    }
    #[test]
    fn session_only_exposes_the_captured_apps_shortcuts() {
        let figma = shortcuts::for_executable("figma.exe").unwrap();
        let mut s = Session {
            source: 1,
            source_app: Some(figma),
            ..Session::default()
        };
        let view = s.view();
        assert_eq!(view.app, Some("figma"));
        let shortcuts: Vec<_> = view
            .actions
            .iter()
            .filter(|a| a.shortcut.is_some())
            .collect();
        assert_eq!(
            shortcuts.len(),
            pieall::shortcuts::for_profile("figma").count()
        );
        assert!(shortcuts
            .iter()
            .all(|a| a.eligible && a.id.starts_with("app:figma:")));
        s.source = 0;
        s.source_app = None;
        assert!(s.view().actions.iter().all(|a| a.shortcut.is_none()));
    }
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
    fn download_output_uses_only_the_saved_absolute_path() {
        assert_eq!(
            download_output(
                "Saved C:\\Downloads\\clip (2).mkv\r\nVideo title\r\nSource video: 1080p"
            ),
            Some(PathBuf::from("C:\\Downloads\\clip (2).mkv"))
        );
        assert_eq!(download_output("FAILED download"), None);
        assert_eq!(download_output("Saved relative.mkv"), None);
        assert_eq!(download_output(""), None);
    }
    #[test]
    fn download_reports_name_one_file_and_its_location() {
        let report = "Saved C:\\Downloads\\clip.mkv\r\nSource video: 1920×1080 · \"avc1\" · Source audio: \"opus\"\r\nFormat: mkv · Quality: best · Audio preference: original";
        let (tone, title, _) = summarize(report, false);
        assert_eq!((tone, title.as_str()), ("ok", "Saved clip.mkv"));
        assert_eq!(
            download_output(report),
            Some(PathBuf::from("C:\\Downloads\\clip.mkv"))
        );
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

    fn running_job(id: u64) -> Job {
        Job {
            view: JobView {
                action: "download_video".into(),
                id,
                title: format!("Download {id}"),
                detail: "Downloading".into(),
                completed: 0,
                total: 1,
                state: "running",
                fraction: Some(0.3),
                output: None,
                has_report: false,
            },
            report: None,
            cancel: Some(Arc::new(AtomicBool::new(false))),
            dismissed: false,
        }
    }

    #[test]
    fn concurrent_jobs_allow_new_context_and_cancel_independently() {
        let mut s = Session {
            ticket: 2,
            jobs: BTreeMap::from([(1, running_job(1)), (2, running_job(2))]),
            ..Session::default()
        };
        assert!(s.idle().is_ok());
        assert!(s.quiescent().is_err());
        let first_cancel = s.jobs[&1].cancel.clone().unwrap();
        let second_cancel = s.jobs[&2].cancel.clone().unwrap();
        s.cancel_job(1).unwrap();
        assert!(first_cancel.load(Ordering::SeqCst));
        assert!(!second_cancel.load(Ordering::SeqCst));
        assert_eq!(s.jobs[&2].view.state, "running");
        s.replace(vec![PathBuf::from("C:/another.png")], String::new());
        s.finish_job(1, 0, "download_video", &Err("Cancelled. Stopped.".into()));
        assert_eq!(s.jobs[&1].view.state, "cancelled");
        assert!(s.cancel_job(1).is_err());
        assert!(s.jobs[&2].running());
        assert_eq!(s.paths, vec![PathBuf::from("C:/another.png")]);
        assert!(s.result.is_none());
        assert!(s.idle().is_ok());
    }

    #[test]
    fn out_of_order_completion_retains_each_result_and_dismissal() {
        let mut s = Session {
            ticket: 2,
            jobs: BTreeMap::from([(1, running_job(1)), (2, running_job(2))]),
            ..Session::default()
        };
        s.jobs.get_mut(&1).unwrap().dismissed = true;
        s.finish_job(
            2,
            0,
            "download_video",
            &Ok("Saved C:\\Downloads\\second.mkv".into()),
        );
        s.finish_job(
            1,
            0,
            "download_video",
            &Ok("Saved C:\\Downloads\\first.mkv".into()),
        );
        assert_eq!(s.result.as_deref(), Some("Saved C:\\Downloads\\second.mkv"));
        assert_eq!(
            s.jobs[&1].view.output,
            Some(PathBuf::from("C:\\Downloads\\first.mkv"))
        );
        assert_eq!(
            s.jobs[&2].view.output,
            Some(PathBuf::from("C:\\Downloads\\second.mkv"))
        );
        assert!(s.jobs[&1].dismissed);
        assert!(!s.jobs[&2].dismissed);
        assert!(s.jobs.values().all(|j| j.cancel.is_none()));
        assert_eq!(s.view().jobs.len(), 2);
        assert!(s.quiescent().is_ok());
    }

    #[test]
    fn waiting_for_shared_engine_setup_is_cancellable() {
        let _setup = ENGINE_SETUP.lock().unwrap();
        let cancel = Arc::new(AtomicBool::new(false));
        let worker_cancel = cancel.clone();
        let waiter = std::thread::spawn(move || engine_setup(&worker_cancel).is_err());
        cancel.store(true, Ordering::SeqCst);
        assert!(waiter.join().unwrap());
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
