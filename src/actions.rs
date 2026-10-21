use crate::output;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashSet},
    fs::{self, File},
    io::{BufRead, Read, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
};
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum InputKind {
    Files,
    Images,
    Text,
    Window,
}
#[derive(Clone)]
pub struct Field {
    pub key: &'static str,
    pub label: &'static str,
    pub default: &'static str,
    pub options: &'static [(&'static str, &'static str)],
}
#[derive(Clone)]
pub struct Action {
    pub id: &'static str,
    pub title: &'static str,
    pub group: &'static str,
    pub input: InputKind,
    pub fields: Vec<Field>,
}
fn field(key: &'static str, label: &'static str, default: &'static str) -> Field {
    Field {
        key,
        label,
        default,
        options: &[],
    }
}
fn choice(
    key: &'static str,
    label: &'static str,
    default: &'static str,
    options: &'static [(&'static str, &'static str)],
) -> Field {
    Field {
        key,
        label,
        default,
        options,
    }
}
pub fn catalog() -> Vec<Action> {
    use InputKind::*;
    let mut a = Vec::new();
    for (id, title) in [
        ("rotate_left", "Rotate left"),
        ("rotate_right", "Rotate right"),
        ("flip_h", "Flip horizontally"),
        ("flip_v", "Flip vertically"),
        ("grayscale", "Grayscale image"),
        ("strip_metadata", "Strip image metadata"),
        ("image_details", "Inspect image details"),
        ("palette", "Extract color palette"),
        ("copy_pixels", "Copy image pixels"),
    ] {
        a.push(Action {
            id,
            title,
            group: "Images",
            input: Images,
            fields: vec![],
        });
    }
    a.push(Action {
        id: "contact_sheet",
        title: "Create contact sheet",
        group: "Images",
        input: Images,
        fields: vec![
            field("columns", "Columns (1–8)", "4"),
            field("cell", "Thumbnail cell size (px)", "240"),
        ],
    });
    a.push(Action {
        id: "images_pdf",
        title: "Images to PDF",
        group: "PDF",
        input: Images,
        fields: vec![
            choice(
                "page_size",
                "Page size",
                "a4",
                &[("a4", "A4"), ("letter", "Letter"), ("image", "Fit image")],
            ),
            choice(
                "orientation",
                "Orientation",
                "portrait",
                &[("portrait", "Portrait"), ("landscape", "Landscape")],
            ),
            field("margin", "Page margin (points)", "0"),
            field("quality", "Image quality (1–100)", "90"),
        ],
    });
    for (id, title, fields) in [
        (
            "convert",
            "Convert image format",
            vec![
                choice(
                    "format",
                    "Output format",
                    "png",
                    &[("png", "PNG"), ("jpg", "JPEG"), ("webp", "WebP (lossless)")],
                ),
                field("quality", "JPEG quality (1–100)", "92"),
                field("background", "JPEG background HEX", "FFFFFF"),
            ],
        ),
        (
            "resize",
            "Resize image",
            vec![
                choice(
                    "mode",
                    "Resize by",
                    "bounds",
                    &[
                        ("bounds", "Fit within dimensions"),
                        ("percent", "Percentage"),
                        ("longest", "Longest side"),
                    ],
                ),
                field("width", "Maximum width (px)", "1200"),
                field("height", "Maximum height (px)", "1200"),
                field("percent", "Scale (1–100%)", "50"),
                field("longest", "Longest side (px)", "1200"),
            ],
        ),
        (
            "crop",
            "Crop image",
            vec![
                field("x", "Left (px)", "0"),
                field("y", "Top (px)", "0"),
                field("width", "Width (px)", "512"),
                field("height", "Height (px)", "512"),
            ],
        ),
        (
            "compress",
            "Compress image to JPEG",
            vec![
                field("quality", "JPEG quality 1–100", "80"),
                field("background", "Flatten alpha HEX", "FFFFFF"),
            ],
        ),
        (
            "frame",
            "Add framed background",
            vec![
                field("padding", "Padding (px)", "64"),
                field("background", "Background HEX", "F3EBDD"),
                field("corners", "Corner radius (px)", "24"),
                choice(
                    "style",
                    "Background style",
                    "solid",
                    &[("solid", "Solid color"), ("gradient", "Diagonal gradient")],
                ),
                field("secondary", "Gradient end HEX", "D3B8A0"),
                choice(
                    "ratio",
                    "Canvas ratio",
                    "natural",
                    &[
                        ("natural", "Natural"),
                        ("16:9", "16:9"),
                        ("4:5", "4:5"),
                        ("1:1", "Square"),
                    ],
                ),
                field("shadow", "Shadow softness (0–64 px)", "0"),
            ],
        ),
    ] {
        a.push(Action {
            id,
            title,
            group: "Images",
            input: Images,
            fields,
        });
    }
    for (id, title) in [
        ("copy_paths", "Copy paths"),
        ("reveal", "Reveal in Explorer"),
        ("terminal", "Open terminal here"),
        ("duplicate", "Duplicate files"),
        ("sha256", "Calculate SHA-256"),
        ("folder_size", "Calculate folder size"),
        ("find_duplicates", "Find duplicate files"),
        ("create_zip", "Create ZIP"),
        ("extract_zip", "Extract ZIP to new folder"),
        ("list_zip", "List ZIP contents"),
        ("test_zip", "Test ZIP integrity"),
    ] {
        a.push(Action {
            id,
            title,
            group: if id.contains("zip") {
                "Archives"
            } else {
                "Files"
            },
            input: Files,
            fields: if id == "create_zip" {
                vec![
                    choice(
                        "method",
                        "Compression method",
                        "deflate",
                        &[
                            ("deflate", "Deflate (smaller)"),
                            ("store", "Store (faster)"),
                        ],
                    ),
                    field("level", "Compression level (0–9)", "6"),
                ]
            } else {
                vec![]
            },
        });
    }
    for (id, title) in [
        ("upper", "UPPERCASE text"),
        ("lower", "lowercase text"),
        ("title", "Title Case text"),
        ("trim", "Trim whitespace"),
        ("sort", "Sort lines"),
        ("dedupe", "Remove duplicate lines"),
        ("count", "Count text"),
        ("json_format", "Format JSON"),
        ("json_minify", "Minify JSON"),
        ("url_encode", "URL encode component"),
        ("url_decode", "URL decode component"),
        ("base64_encode", "Base64 encode"),
        ("base64_decode", "Base64 decode"),
    ] {
        a.push(Action {
            id,
            title,
            group: "Text",
            input: Text,
            fields: vec![],
        });
    }
    for (id, title) in [
        ("always_top", "Toggle always on top"),
        ("center_window", "Center window"),
        ("fit_window", "Fit window to work area"),
        ("window_details", "Inspect window details"),
    ] {
        a.push(Action {
            id,
            title,
            group: "Window",
            input: Window,
            fields: vec![],
        });
    }
    for (id, title) in [
        ("pdf_details", "Inspect PDF details"),
        ("pdf_extract", "Extract PDF pages"),
        ("pdf_split", "Split PDF into pages"),
        ("pdf_rotate", "Rotate PDF pages"),
        ("pdf_compress", "Compress PDF streams"),
        ("pdf_strip_metadata", "Remove PDF document metadata"),
        ("pdf_text", "PDF to selectable text"),
    ] {
        let mut fields = Vec::new();
        if matches!(id, "pdf_extract" | "pdf_split" | "pdf_rotate" | "pdf_text") {
            fields.push(field("pages", "Pages (all, 1-3,5)", "all"));
        }
        if id == "pdf_rotate" {
            fields.push(choice(
                "angle",
                "Rotate clockwise",
                "90",
                &[("90", "90°"), ("180", "180°"), ("270", "270°")],
            ));
        }
        a.push(Action {
            id,
            title,
            group: "PDF",
            input: Files,
            fields,
        });
    }
    for (id, title, mut fields) in [
        (
            "video_convert",
            "Convert video format",
            vec![choice(
                "format",
                "Output format",
                "mp4",
                &[
                    ("mp4", "MP4"),
                    ("mkv", "MKV"),
                    ("mov", "MOV"),
                    ("webm", "WebM"),
                ],
            )],
        ),
        (
            "video_compress",
            "Compress video",
            vec![field("quality", "CRF quality (0 best, 51 smallest)", "28")],
        ),
        (
            "video_resize",
            "Resize video",
            vec![
                field("width", "Maximum width (px)", "1280"),
                field("height", "Maximum height (px)", "720"),
            ],
        ),
        (
            "video_rotate",
            "Rotate video",
            vec![choice(
                "angle",
                "Rotate clockwise",
                "90",
                &[("90", "90°"), ("180", "180°"), ("270", "270°")],
            )],
        ),
        ("video_mute", "Remove video audio", vec![]),
        (
            "video_trim",
            "Trim video",
            vec![
                field("start", "Start (seconds)", "0"),
                field("duration", "Duration (seconds)", "10"),
            ],
        ),
        (
            "video_gif",
            "Video to GIF",
            vec![
                field("width", "Width (px)", "480"),
                field("fps", "Frames per second (1–30)", "12"),
            ],
        ),
        (
            "video_frame",
            "Extract video frame",
            vec![field("start", "Time (seconds)", "0")],
        ),
        ("video_audio", "Extract video audio", vec![]),
        ("audio_convert", "Convert audio format", vec![]),
        (
            "audio_trim",
            "Trim audio",
            vec![
                field("start", "Start (seconds)", "0"),
                field("duration", "Duration (seconds)", "10"),
            ],
        ),
        (
            "audio_compress",
            "Compress audio to MP3",
            vec![field("bitrate", "Bitrate (32–320 kbps)", "128")],
        ),
        ("audio_normalize", "Normalize audio loudness", vec![]),
        (
            "audio_volume",
            "Change audio volume",
            vec![field("gain", "Volume multiplier (0–10)", "1.5")],
        ),
        ("media_details", "Inspect media details", vec![]),
    ] {
        if matches!(
            id,
            "video_audio" | "audio_convert" | "audio_normalize" | "audio_volume"
        ) {
            fields.insert(
                0,
                choice(
                    "format",
                    "Output format",
                    "mp3",
                    &[
                        ("mp3", "MP3"),
                        ("wav", "WAV (PCM)"),
                        ("flac", "FLAC"),
                        ("m4a", "M4A"),
                        ("ogg", "Ogg Vorbis"),
                        ("opus", "Opus"),
                    ],
                ),
            );
        }
        a.push(Action {
            id,
            title,
            group: if id.starts_with("audio_") {
                "Audio"
            } else {
                "Video"
            },
            input: Files,
            fields,
        });
    }
    for (id, title) in [
        ("download_video", "Download video from URL"),
        ("download_audio", "Download audio from URL"),
    ] {
        a.push(Action {
            id,
            title,
            group: "Browser",
            input: Text,
            fields: vec![],
        });
    }
    a
}
pub fn media_action(id: &str) -> bool {
    id.starts_with("video_") || id.starts_with("audio_") || id == "media_details"
}
pub fn missing_engine(id: &str, engines: &crate::engines::Engines) -> Option<&'static str> {
    if crate::downloads::is_download(id) {
        return ["yt-dlp", "ffmpeg", "ffprobe"]
            .iter()
            .find(|name| !engines.contains_key(**name))
            .map(|_| "Choose yt-dlp, FFmpeg and FFprobe in Tool engines.");
    }
    if !media_action(id) {
        return None;
    }
    if !engines.contains_key("ffprobe") {
        return Some("Choose FFprobe in Tool engines.");
    }
    if id != "media_details" && !engines.contains_key("ffmpeg") {
        return Some("Choose FFmpeg in Tool engines.");
    }
    None
}
pub fn eligible(a: &Action, paths: &[PathBuf], text: bool) -> bool {
    match a.input {
        InputKind::Window => true,
        InputKind::Text => text,
        InputKind::Images => {
            if a.id == "copy_pixels" && paths.len() != 1 {
                return false;
            }
            !paths.is_empty()
                && paths.iter().all(|p| {
                    p.is_file()
                        && p.extension().is_some_and(|e| {
                            matches!(
                                e.to_string_lossy().to_ascii_lowercase().as_str(),
                                "png" | "jpg" | "jpeg" | "webp"
                            )
                        })
                })
        }
        InputKind::Files => {
            if media_action(a.id) {
                return !paths.is_empty()
                    && paths.iter().all(|p| {
                        p.is_file() && crate::media_tools::is_media(p, a.id.starts_with("video_"))
                    });
            }
            if a.id.starts_with("pdf_") {
                return !paths.is_empty()
                    && paths.iter().all(|p| {
                        p.is_file() && p.extension().is_some_and(|e| e.eq_ignore_ascii_case("pdf"))
                    });
            }
            if matches!(a.id, "folder_size" | "find_duplicates") {
                return !paths.is_empty() && paths.iter().all(|p| p.is_dir());
            }
            !paths.is_empty()
                && paths.iter().all(|p| {
                    if a.id.ends_with("zip") && a.id != "create_zip" {
                        p.is_file() && p.extension().is_some_and(|e| e.eq_ignore_ascii_case("zip"))
                    } else if matches!(a.id, "sha256" | "duplicate") {
                        p.is_file()
                    } else {
                        p.exists()
                    }
                })
        }
    }
}
#[derive(Serialize, Deserialize)]
struct Request {
    id: String,
    paths: Vec<PathBuf>,
    text: String,
    params: BTreeMap<String, String>,
    #[serde(default)]
    engines: crate::engines::Engines,
    #[serde(default)]
    download: Option<crate::downloads::Settings>,
}
pub fn execute(
    id: &str,
    paths: &[PathBuf],
    text: &str,
    params: &BTreeMap<String, String>,
) -> Result<String, String> {
    execute_controlled(
        id,
        paths,
        text,
        params,
        &std::sync::atomic::AtomicBool::new(false),
        |_| {},
    )
}
pub fn execute_controlled(
    id: &str,
    paths: &[PathBuf],
    text: &str,
    params: &BTreeMap<String, String>,
    cancelled: &std::sync::atomic::AtomicBool,
    progress: impl FnMut(crate::jobs::Progress),
) -> Result<String, String> {
    execute_with_engines(
        id,
        paths,
        text,
        params,
        &crate::engines::Engines::new(),
        cancelled,
        progress,
    )
}
pub fn execute_with_engines(
    id: &str,
    paths: &[PathBuf],
    text: &str,
    params: &BTreeMap<String, String>,
    engines: &crate::engines::Engines,
    cancelled: &std::sync::atomic::AtomicBool,
    progress: impl FnMut(crate::jobs::Progress),
) -> Result<String, String> {
    let request = Request {
        id: id.into(),
        paths: paths.into(),
        text: text.into(),
        params: params.clone(),
        engines: engines.clone(),
        download: None,
    };
    // Launchers start apps that must outlive the call: the sandboxed worker's job
    // kills every descendant when it exits, and a child holding the worker's pipes
    // would stall its reply. They only spawn a process, so they run here.
    if matches!(id, "terminal" | "reveal") {
        return run(&request);
    }
    execute_request(request, cancelled, progress)
}
pub fn execute_download(
    id: &str,
    url: &str,
    settings: &crate::downloads::Settings,
    engines: &crate::engines::Engines,
    cancelled: &std::sync::atomic::AtomicBool,
    progress: impl FnMut(crate::jobs::Progress),
) -> Result<String, String> {
    let request = Request {
        id: id.into(),
        paths: vec![],
        text: url.into(),
        params: BTreeMap::new(),
        engines: engines.clone(),
        download: Some(settings.clone()),
    };
    execute_request(request, cancelled, progress)
}
fn execute_request(
    request: Request,
    cancelled: &std::sync::atomic::AtomicBool,
    mut progress: impl FnMut(crate::jobs::Progress),
) -> Result<String, String> {
    let mut bytes = serde_json::to_vec(&request).map_err(|e| e.to_string())?;
    if bytes.len() > 3 * 1024 * 1024 {
        return Err("Input exceeds worker message limit.".into());
    }
    let mut cmd = Command::new(std::env::current_exe().map_err(|e| e.to_string())?);
    cmd.arg("--worker")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000);
    }
    let mut child = cmd.spawn().map_err(|e| e.to_string())?;
    #[cfg(windows)]
    let containment = match crate::jobs::WorkerJob::attach(&child) {
        Ok(job) => job,
        Err(e) => {
            let _ = child.kill();
            let _ = child.wait();
            return Err(e);
        }
    };
    bytes.push(b'\n');
    let mut stdin = child.stdin.take().ok_or("Worker input unavailable")?;
    let write_result = stdin.write_all(&bytes);
    if let Err(e) = write_result {
        let _ = child.kill();
        let _ = child.wait();
        return Err(e.to_string());
    }
    let stdout = child.stdout.take().ok_or("Worker output unavailable")?;
    let stderr = child.stderr.take().ok_or("Worker progress unavailable")?;
    let (send, updates) = std::sync::mpsc::sync_channel(32);
    let progress_reader = std::thread::spawn(move || {
        let mut input = std::io::BufReader::new(stderr);
        loop {
            let mut line = String::new();
            if input.by_ref().take(4096).read_line(&mut line).unwrap_or(0) == 0 {
                break;
            }
            if let Ok(p) = serde_json::from_str::<crate::jobs::Progress>(&line) {
                let _ = send.try_send(p);
            }
        }
    });
    let reader = std::thread::spawn(move || {
        let mut data = Vec::new();
        stdout
            .take(1024 * 1024)
            .read_to_end(&mut data)
            .map(|_| data)
    });
    let seconds = if media_action(&request.id) || crate::downloads::is_download(&request.id) {
        3600
    } else {
        120
    };
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(seconds);
    let mut cancel_sent = None;
    let status = loop {
        for update in updates.try_iter() {
            progress(update);
        }
        if cancelled.load(std::sync::atomic::Ordering::SeqCst) && cancel_sent.is_none() {
            let _ = stdin.write_all(b"cancel\n");
            cancel_sent = Some(std::time::Instant::now());
        }
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) => {}
            Err(e) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(e.to_string());
            }
        }
        if std::time::Instant::now() > deadline
            || cancel_sent.is_some_and(|t| t.elapsed().as_secs() >= 5)
        {
            #[cfg(windows)]
            containment.terminate();
            let _ = child.kill();
            let _ = child.wait();
            let _ = reader.join();
            let _ = progress_reader.join();
            if cancel_sent.is_some() {
                return Err("Cancelled. Previously completed outputs are kept. A slow codec was stopped; temporary files may remain.".into());
            }
            return Err(format!("Job exceeded its {seconds}-second limit. Check the destination for any completed outputs."));
        }
        std::thread::sleep(std::time::Duration::from_millis(25));
    };
    let _ = progress_reader.join();
    for update in updates.try_iter() {
        progress(update);
    }
    let data = reader
        .join()
        .map_err(|_| "Worker reader stopped")?
        .map_err(|e| e.to_string())?;
    if !status.success() {
        return Err("Worker stopped unexpectedly; original files are unchanged. Check destination for any completed outputs.".into());
    }
    serde_json::from_slice::<Result<String, String>>(&data)
        .map_err(|_| "Invalid worker response".to_string())?
}
pub fn worker_main() {
    crate::jobs::worker_start();
    let result = (|| {
        let mut data = Vec::new();
        let mut input = std::io::BufReader::new(std::io::stdin());
        input
            .by_ref()
            .take(3 * 1024 * 1024 + 1)
            .read_until(b'\n', &mut data)
            .map_err(|e| e.to_string())?;
        if data.len() > 3 * 1024 * 1024 {
            return Err("Input too large".into());
        }
        let r: Request = serde_json::from_slice(&data).map_err(|e| e.to_string())?;
        std::thread::spawn(move || {
            let mut command = String::new();
            if input.take(32).read_line(&mut command).is_ok() && command.trim() == "cancel" {
                crate::jobs::cancel();
            }
        });
        run(&r)
    })();
    let result = result.and_then(|s| {
        if s.len() > 512 * 1024 {
            Err("Result exceeds the 512 KiB display limit.".into())
        } else {
            Ok(s)
        }
    });
    if let Ok(bytes) = serde_json::to_vec(&result) {
        let _ = std::io::stdout().write_all(&bytes);
    }
}
fn number(p: &BTreeMap<String, String>, key: &str, default: u32, max: u32) -> Result<u32, String> {
    let n = p
        .get(key)
        .map(|s| s.parse::<u32>())
        .transpose()
        .map_err(|_| format!("{key} must be a whole number"))?
        .unwrap_or(default);
    if n > max {
        return Err(format!("{key} must be at most {max}"));
    }
    Ok(n)
}
fn color(p: &BTreeMap<String, String>) -> Result<[u8; 3], String> {
    let s = p
        .get("background")
        .map(|s| s.trim_start_matches('#'))
        .unwrap_or("FFFFFF");
    if s.len() != 6 || !s.is_ascii() {
        return Err("Background must contain six HEX digits.".into());
    }
    let n = u32::from_str_radix(s, 16).map_err(|_| "Invalid HEX color")?;
    Ok([(n >> 16) as u8, (n >> 8) as u8, n as u8])
}
fn run(r: &Request) -> Result<String, String> {
    crate::jobs::check()?;
    let action = catalog()
        .into_iter()
        .find(|a| a.id == r.id)
        .ok_or("Unknown action")?;
    if !eligible(&action, &r.paths, !r.text.is_empty()) {
        return Err("Action does not support every selected target.".into());
    }
    if let Some(reason) = missing_engine(&r.id, &r.engines) {
        return Err(reason.into());
    }
    if r.paths.len() > 256 {
        return Err("Select at most 256 files.".into());
    }
    if action.input == InputKind::Text {
        if crate::downloads::is_download(&r.id) {
            return crate::downloads::run(
                &r.id,
                &r.text,
                r.download
                    .as_ref()
                    .ok_or("Open Browser tools to set up downloads.")?,
                &r.engines,
            );
        }
        return text_action(&r.id, &r.text);
    }
    if action.input == InputKind::Window {
        let raw = r
            .params
            .get("source_hwnd")
            .ok_or("No source window captured")?
            .parse()
            .map_err(|_| "Invalid source window")?;
        let expected = r
            .params
            .get("source_pid")
            .ok_or("Missing source process identity")?
            .parse::<u32>()
            .map_err(|_| "Invalid source process identity")?;
        let mut actual = 0u32;
        unsafe {
            windows_sys::Win32::UI::WindowsAndMessaging::GetWindowThreadProcessId(
                raw as windows_sys::Win32::Foundation::HWND,
                &mut actual,
            );
        }
        if expected == 0 || actual != expected {
            return Err("Source window changed or closed. Invoke PieAll again.".into());
        }
        return crate::platform::window_action(&r.id, raw);
    }
    if r.id == "copy_paths" {
        let s = r
            .paths
            .iter()
            .map(|p| p.to_string_lossy().into_owned())
            .collect::<Vec<_>>()
            .join("\r\n");
        crate::platform::set_clipboard_text(&s)?;
        return Ok(format!("Copied {} paths.", r.paths.len()));
    }
    if r.id == "create_zip" {
        return crate::archive_tools::create(&r.paths, &r.params);
    }
    if r.id == "contact_sheet" {
        return contact_sheet(&r.paths, &r.params);
    }
    if r.id == "images_pdf" {
        return images_pdf(&r.paths, &r.params);
    }
    crate::jobs::check()?;
    let mut report = Vec::new();
    let mut succeeded = 0;
    for (index, path) in r.paths.iter().enumerate() {
        crate::jobs::check()?;
        crate::jobs::progress(
            index,
            r.paths.len(),
            path.file_name().unwrap_or_default().to_string_lossy(),
        );
        let result = (|| {
            output::plain_path(path)?;
            if action.input == InputKind::Images {
                image_action(&r.id, path, &r.params)
            } else if r.id.starts_with("pdf_") {
                crate::pdf_tools::run_with_progress(&r.id, path, &r.params, index, r.paths.len())
            } else if media_action(&r.id) {
                crate::media_tools::run(&r.id, path, &r.params, &r.engines, index, r.paths.len())
            } else if matches!(r.id.as_str(), "list_zip" | "test_zip" | "extract_zip") {
                archive_action(&r.id, path, |fraction, item| {
                    crate::jobs::progress_fraction(index, r.paths.len(), item, Some(fraction));
                })
            } else {
                file_action(&r.id, path)
            }
        })();
        match result {
            Ok(s) => {
                succeeded += 1;
                report.push(s);
            }
            Err(e) => report.push(format!("FAILED {}: {e}", path.display())),
        }
    }
    crate::jobs::check()?;
    crate::jobs::progress(r.paths.len(), r.paths.len(), "Finished");
    let summary = format!(
        "Completed {succeeded} of {} targets.\r\n\r\n{}",
        r.paths.len(),
        report.join("\r\n\r\n")
    );
    if succeeded == 0 {
        Err(summary)
    } else {
        Ok(summary)
    }
}
fn load_image(path: &Path) -> Result<image::DynamicImage, String> {
    crate::jobs::check()?;
    if fs::metadata(path).map_err(|e| e.to_string())?.len() > 128 * 1024 * 1024 {
        return Err("Image file exceeds 128 MiB.".into());
    }
    let mut reader = image::ImageReader::open(path)
        .map_err(|e| e.to_string())?
        .with_guessed_format()
        .map_err(|e| e.to_string())?;
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(16384);
    limits.max_image_height = Some(16384);
    limits.max_alloc = Some(256 * 1024 * 1024);
    reader.limits(limits);
    let orientation = reader
        .into_decoder()
        .map_err(|e| e.to_string())
        .and_then(|mut d| {
            use image::ImageDecoder;
            d.orientation().map_err(|e| e.to_string())
        })?;
    let mut reader = image::ImageReader::open(path)
        .map_err(|e| e.to_string())?
        .with_guessed_format()
        .map_err(|e| e.to_string())?;
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(16384);
    limits.max_image_height = Some(16384);
    limits.max_alloc = Some(256 * 1024 * 1024);
    reader.limits(limits);
    let mut img = reader.decode().map_err(|e| e.to_string())?;
    img.apply_orientation(orientation);
    if img.width() as u64 * img.height() as u64 > 40_000_000 {
        return Err("Image exceeds 40 megapixels.".into());
    }
    Ok(img)
}
fn image_action(id: &str, path: &Path, p: &BTreeMap<String, String>) -> Result<String, String> {
    use image::GenericImageView;
    let mut img = load_image(path)?;
    let (w, h) = img.dimensions();
    // Edits keep the source format; only Convert and Compress choose another one.
    let mut ext = match path
        .extension()
        .map(|e| e.to_string_lossy().to_ascii_lowercase())
        .as_deref()
    {
        Some("jpg" | "jpeg") => "jpg",
        Some("webp") => "webp",
        _ => "png",
    };
    let mut label = match id {
        "rotate_left" | "rotate_right" => "rotated".to_string(),
        "flip_h" | "flip_v" => "flipped".into(),
        "strip_metadata" => "clean".into(),
        "crop" => "cropped".into(),
        "frame" => "framed".into(),
        "compress" => "compressed".into(),
        "convert" => String::new(),
        other => other.into(),
    };
    match id {
        "copy_pixels" => {
            let rgba = img.to_rgba8();
            let mut pixels = rgba.into_raw();
            for pixel in pixels.chunks_exact_mut(4) {
                pixel.swap(0, 2);
            }
            crate::platform::set_clipboard_image(w, h, &pixels)?;
            return Ok("Copied decoded image pixels to the clipboard.".into());
        }
        "image_details" => {
            return Ok(format!(
                "{}\n{} × {} pixels (orientation normalized)\n{} bytes\nColor: {:?}",
                path.display(),
                w,
                h,
                fs::metadata(path).map_err(|e| e.to_string())?.len(),
                img.color()
            ))
        }
        "palette" => {
            let small = img.thumbnail(96, 96).to_rgb8();
            let mut bins = BTreeMap::<[u8; 3], usize>::new();
            for px in small.pixels() {
                let key = [
                    px[0] / 32 * 32 + 16,
                    px[1] / 32 * 32 + 16,
                    px[2] / 32 * 32 + 16,
                ];
                *bins.entry(key).or_default() += 1;
            }
            let mut bins: Vec<_> = bins.into_iter().collect();
            bins.sort_by_key(|(_, n)| std::cmp::Reverse(*n));
            return Ok(bins
                .iter()
                .take(8)
                .map(|(c, _)| format!("#{:02X}{:02X}{:02X}", c[0], c[1], c[2]))
                .collect::<Vec<_>>()
                .join("\r\n"));
        }
        "rotate_left" => img = img.rotate270(),
        "rotate_right" => img = img.rotate90(),
        "flip_h" => img = img.fliph(),
        "flip_v" => img = img.flipv(),
        "grayscale" => img = img.grayscale(),
        "strip_metadata" => {}
        "resize" => {
            let mode = p.get("mode").map_or("bounds", String::as_str);
            let (nw, nh) = if p.contains_key("scale") || mode == "percent" {
                let scale = number(
                    p,
                    if p.contains_key("scale") {
                        "scale"
                    } else {
                        "percent"
                    },
                    50,
                    100,
                )?;
                if scale == 0 {
                    return Err("Scale must be 1–100%.".into());
                }
                (
                    (w as u64 * scale as u64 / 100).max(1) as u32,
                    (h as u64 * scale as u64 / 100).max(1) as u32,
                )
            } else if mode == "longest" {
                let side = number(p, "longest", 1200, 16384)?;
                (side, side)
            } else if mode == "bounds" {
                (
                    number(p, "width", 1200, 16384)?,
                    number(p, "height", 1200, 16384)?,
                )
            } else {
                return Err("Choose bounds, percent or longest resize mode.".into());
            };
            if nw == 0 || nh == 0 {
                return Err("Dimensions must be positive.".into());
            }
            img = if p.contains_key("scale") || mode == "percent" {
                img.resize_exact(nw, nh, image::imageops::FilterType::Lanczos3)
            } else {
                img.resize(nw.min(w), nh.min(h), image::imageops::FilterType::Lanczos3)
            };
            label = format!("{}x{}", img.width(), img.height());
        }
        "crop" => {
            let x = number(p, "x", 0, w)?;
            let y = number(p, "y", 0, h)?;
            let cw = number(p, "width", 512, w)?;
            let ch = number(p, "height", 512, h)?;
            if cw == 0
                || ch == 0
                || x.checked_add(cw).is_none_or(|n| n > w)
                || y.checked_add(ch).is_none_or(|n| n > h)
            {
                return Err("Crop must be positive and inside the image.".into());
            }
            img = img.crop_imm(x, y, cw, ch);
        }
        "convert" => {
            ext = match p.get("format").map(|s| s.as_str()).unwrap_or("png") {
                "png" => "png",
                "jpg" | "jpeg" => "jpg",
                "webp" => "webp",
                _ => return Err("Choose png, jpg or webp.".into()),
            };
        }
        // Transparent images stay PNG so compression never paints over their alpha.
        "compress" => {
            ext = if img.color().has_alpha() && img.to_rgba8().pixels().any(|px| px[3] < 255) {
                "png"
            } else {
                "jpg"
            }
        }
        "frame" => {
            img = crate::image_tools::frame(img, p)?;
        }
        _ => return Err("Unsupported image action".into()),
    }
    let mut stage = output::Staged::new(path.parent().ok_or("No parent folder")?)?;
    if ext == "jpg" {
        let bg = color(p)?;
        let rgba = img.to_rgba8();
        let rgb = image::RgbImage::from_fn(img.width(), img.height(), |x, y| {
            let c = rgba.get_pixel(x, y);
            let a = c[3] as u32;
            image::Rgb(std::array::from_fn(|i| {
                ((c[i] as u32 * a + bg[i] as u32 * (255 - a) + 127) / 255) as u8
            }))
        });
        let quality = number(p, "quality", if id == "compress" { 80 } else { 92 }, 100)?;
        if quality == 0 {
            return Err("Quality must be 1–100.".into());
        }
        image::codecs::jpeg::JpegEncoder::new_with_quality(&mut stage.file, quality as u8)
            .encode_image(&rgb)
            .map_err(|e| e.to_string())?;
    } else if ext == "png" && id == "compress" {
        use image::codecs::png::{CompressionType, FilterType, PngEncoder};
        img.write_with_encoder(PngEncoder::new_with_quality(
            &mut stage.file,
            CompressionType::Best,
            FilterType::Adaptive,
        ))
        .map_err(|e| e.to_string())?;
    } else {
        img.write_to(
            &mut stage.file,
            if ext == "webp" {
                image::ImageFormat::WebP
            } else {
                image::ImageFormat::Png
            },
        )
        .map_err(|e| e.to_string())?;
    }
    let before = fs::metadata(path).map_err(|e| e.to_string())?.len();
    let after = stage.file.metadata().map_err(|e| e.to_string())?.len();
    if id == "compress" && after >= before {
        return Ok(format!(
            "Skipped {}: it is already as small as this setting makes it.",
            path.display()
        ));
    }
    let saved = stage.publish(path, &label, ext)?;
    if id == "compress" {
        return Ok(format!(
            "Saved {} ({} → {}, {}% smaller)",
            saved.display(),
            human_size(before),
            human_size(after),
            100 - after * 100 / before.max(1)
        ));
    }
    Ok(format!("Saved {}", saved.display()))
}
pub fn human_size(bytes: u64) -> String {
    match bytes {
        0..1024 => format!("{bytes} B"),
        1024..1_048_576 => format!("{:.0} KB", bytes as f64 / 1024.0),
        _ => format!("{:.1} MB", bytes as f64 / 1_048_576.0),
    }
}
fn file_action(id: &str, path: &Path) -> Result<String, String> {
    match id {
        "folder_size" | "find_duplicates" => folder_report(id, path),
        "reveal" => {
            let arg = if path.is_dir() {
                path.as_os_str().to_os_string()
            } else {
                let mut a = std::ffi::OsString::from("/select,");
                a.push(path);
                a
            };
            Command::new("explorer.exe")
                .arg(arg)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .map_err(|e| e.to_string())?;
            Ok("Opened Explorer.".into())
        }
        "terminal" => {
            let dir = if path.is_dir() {
                path
            } else {
                path.parent().ok_or("No parent folder")?
            };
            // ShellExecute gives cmd a console of its own (std's Command would hand it
            // PieAll's stdio). It must run in the app process, not the worker: see
            // execute_with_engines.
            use std::os::windows::ffi::OsStrExt;
            let directory: Vec<u16> = dir.as_os_str().encode_wide().chain(Some(0)).collect();
            let program: Vec<u16> = "cmd.exe".encode_utf16().chain(Some(0)).collect();
            let opened = unsafe {
                windows_sys::Win32::UI::Shell::ShellExecuteW(
                    std::ptr::null_mut(),
                    std::ptr::null(),
                    program.as_ptr(),
                    std::ptr::null(),
                    directory.as_ptr(),
                    windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL,
                )
            };
            if opened as usize <= 32 {
                return Err("Windows could not open a terminal in this folder.".into());
            }
            Ok(format!("Opened terminal in {}", dir.display()))
        }
        "duplicate" => {
            let mut input = File::open(path).map_err(|e| e.to_string())?;
            if input.metadata().map_err(|e| e.to_string())?.len() > 2 * 1024 * 1024 * 1024 {
                return Err("Preview copy limit is 2 GiB.".into());
            }
            let mut stage = output::Staged::new(path.parent().ok_or("No parent")?)?;
            std::io::copy(&mut input, &mut stage.file).map_err(|e| e.to_string())?;
            Ok(format!(
                "Saved {}",
                stage
                    .publish_named(
                        path.parent().ok_or("No parent")?,
                        &format!(
                            "{} - Copy",
                            path.file_stem().unwrap_or_default().to_string_lossy()
                        ),
                        path.extension().and_then(|e| e.to_str()).unwrap_or("bin")
                    )?
                    .display()
            ))
        }
        "sha256" => {
            use sha2::{Digest, Sha256};
            let mut file = File::open(path).map_err(|e| e.to_string())?;
            let mut hash = Sha256::new();
            let mut buf = [0u8; 65536];
            loop {
                crate::jobs::check()?;
                let n = file.read(&mut buf).map_err(|e| e.to_string())?;
                if n == 0 {
                    break;
                }
                hash.update(&buf[..n]);
            }
            Ok(format!("{}\n{:x}", path.display(), hash.finalize()))
        }
        _ => Err("Unsupported file action".into()),
    }
}
struct TempDir(PathBuf);
impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn archive_action(
    id: &str,
    path: &Path,
    mut progress: impl FnMut(f64, &str),
) -> Result<String, String> {
    progress(0., "Reading ZIP");
    let f = File::open(path).map_err(|e| e.to_string())?;
    if f.metadata().map_err(|e| e.to_string())?.len() > 1536 * 1024 * 1024 {
        return Err("ZIP input exceeds 1536 MiB.".into());
    }
    let mut zip = zip::ZipArchive::new(f).map_err(|e| e.to_string())?;
    if zip.len() > 10000 {
        return Err("ZIP exceeds 10,000 entries.".into());
    }
    let mut total = 0u64;
    let mut names = HashSet::new();
    for i in 0..zip.len() {
        crate::jobs::check()?;
        let entry = zip.by_index(i).map_err(|e| e.to_string())?;
        let relative = output::safe_archive_name(entry.name())?;
        if entry.is_symlink() || entry.unix_mode().is_some_and(|m| m & 0o170000 == 0o120000) {
            return Err("Archive links are unsupported.".into());
        }
        let key = relative.to_string_lossy().to_lowercase();
        if !names.insert(key) {
            return Err("Duplicate or case-colliding ZIP entry.".into());
        }
        total = total.checked_add(entry.size()).ok_or("ZIP size overflow")?;
        if total > 1536 * 1024 * 1024
            || entry.size() > 768 * 1024 * 1024
            || entry.size()
                > entry
                    .compressed_size()
                    .saturating_mul(3000)
                    .max(1024 * 1024)
        {
            return Err("ZIP exceeds expansion safety limits.".into());
        }
    }
    if id == "list_zip" {
        let mut lines = Vec::new();
        for i in 0..zip.len() {
            crate::jobs::check()?;
            let entry = zip.by_index(i).map_err(|e| e.to_string())?;
            lines.push(format!("{}  ({} bytes)", entry.name(), entry.size()));
            if lines.iter().map(String::len).sum::<usize>() > 256 * 1024 {
                return Err("Archive listing exceeds display limit.".into());
            }
        }
        return Ok(lines.join("\r\n"));
    }
    let parent = path.parent().ok_or("No parent")?;
    let _parent_guards = output::lock_parent(parent)?;
    let mut temp = None;
    if id == "extract_zip" {
        for n in 0..100 {
            let p = parent.join(format!(".pieall-extract-{}-{n}", std::process::id()));
            match fs::create_dir(&p) {
                Ok(()) => {
                    temp = Some(TempDir(p));
                    break;
                }
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(e) => return Err(e.to_string()),
            }
        }
        if temp.is_none() {
            return Err("Cannot reserve extraction folder.".into());
        }
    }
    let mut actual_total = 0u64;
    let mut last_progress = std::time::Instant::now();
    let mut buffer = [0u8; 65536];
    for i in 0..zip.len() {
        crate::jobs::check()?;
        let mut entry = zip.by_index(i).map_err(|e| e.to_string())?;
        if entry.is_dir() {
            if let Some(t) = &temp {
                fs::create_dir_all(t.0.join(output::safe_archive_name(entry.name())?))
                    .map_err(|e| e.to_string())?;
            }
            continue;
        }
        let limit = entry.size();
        let mut out: Box<dyn Write> = if let Some(t) = &temp {
            let dest = t.0.join(output::safe_archive_name(entry.name())?);
            fs::create_dir_all(dest.parent().ok_or("No archive parent")?)
                .map_err(|e| e.to_string())?;
            Box::new(
                fs::OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(dest)
                    .map_err(|e| e.to_string())?,
            )
        } else {
            Box::new(std::io::sink())
        };
        let name = entry.name().to_string();
        let mut input = entry.by_ref().take(limit + 1);
        let mut copied = 0u64;
        loop {
            crate::jobs::check()?;
            let n = input.read(&mut buffer).map_err(|e| e.to_string())?;
            if n == 0 {
                break;
            }
            copied += n as u64;
            if copied > limit {
                return Err("ZIP entry expanded beyond its declared size.".into());
            }
            out.write_all(&buffer[..n]).map_err(|e| e.to_string())?;
            if last_progress.elapsed() >= std::time::Duration::from_millis(100) {
                progress((actual_total + copied) as f64 / total.max(1) as f64, &name);
                last_progress = std::time::Instant::now();
            }
        }
        if copied != limit {
            return Err("ZIP entry expanded beyond its declared size.".into());
        }
        actual_total += copied;
        if actual_total > 1536 * 1024 * 1024 {
            return Err("ZIP expansion limit exceeded.".into());
        }
    }
    progress(1., "Finishing ZIP");
    if let Some(t) = temp {
        let stem = path.file_stem().unwrap_or_default().to_string_lossy();
        for n in 0..10000 {
            let suffix = if n == 0 {
                String::new()
            } else {
                format!(" ({})", n + 1)
            };
            // Named after the archive, like Windows "Extract all".
            let dest = parent.join(format!("{stem}{suffix}"));
            if dest.exists() {
                continue;
            }
            output::plain_path(parent)?;
            let published = crate::jobs::publish(|| match fs::rename(&t.0, &dest) {
                Ok(()) => Ok(true),
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => Ok(false),
                Err(e) => Err(e.to_string()),
            })?;
            if published {
                return Ok(format!("Extracted to {}", dest.display()));
            }
        }
        return Err("Too many extraction name collisions.".into());
    }
    Ok(format!(
        "Verified {} ZIP entries; {} bytes.",
        zip.len(),
        actual_total
    ))
}
fn text_action(id: &str, text: &str) -> Result<String, String> {
    use base64::Engine;
    if text.len() > 2 * 1024 * 1024 {
        return Err("Text exceeds 2 MiB.".into());
    }
    Ok(match id{
 "upper"=>text.to_uppercase(),"lower"=>text.to_lowercase(),"title"=>text.split_whitespace().map(|s|{let mut c=s.chars();c.next().map(|v|v.to_uppercase().collect::<String>()+&c.as_str().to_lowercase()).unwrap_or_default()}).collect::<Vec<_>>().join(" "),
 "trim"=>text.lines().map(str::trim).collect::<Vec<_>>().join("\r\n"),"sort"=>{let mut v:Vec<_>=text.lines().collect();v.sort();v.join("\r\n")},"dedupe"=>{let mut seen=HashSet::new();text.lines().filter(|s|seen.insert(*s)).collect::<Vec<_>>().join("\r\n")},
 "count"=>format!("Unicode scalar values: {}\nWords (whitespace-separated): {}\nLines: {}\nUTF-8 bytes: {}",text.chars().count(),text.split_whitespace().count(),text.lines().count(),text.len()),
 "json_format"|"json_minify"=>{let value:serde_json::Value=serde_json::from_str(text).map_err(|e|e.to_string())?;if id=="json_format"{serde_json::to_string_pretty(&value)}else{serde_json::to_string(&value)}.map_err(|e|e.to_string())?},
 "url_encode"=>text.bytes().map(|b|if b.is_ascii_alphanumeric()||b"-._~".contains(&b){(b as char).to_string()}else{format!("%{b:02X}")}).collect(),
 "url_decode"=>{let bytes=text.as_bytes();let mut out=Vec::new();let mut i=0;while i<bytes.len(){if bytes[i]==b'%'{if i+2>=bytes.len(){return Err("Incomplete percent escape".into());}let s=std::str::from_utf8(&bytes[i+1..i+3]).map_err(|_|"Invalid escape")?;out.push(u8::from_str_radix(s,16).map_err(|_|"Invalid percent escape")?);i+=3;}else{out.push(bytes[i]);i+=1;}}String::from_utf8(out).map_err(|_|"Decoded URL component is not UTF-8 text")?},
 "base64_encode"=>base64::engine::general_purpose::STANDARD.encode(text),"base64_decode"=>{let bytes=base64::engine::general_purpose::STANDARD.decode(text.trim()).map_err(|e|e.to_string())?;let s=String::from_utf8(bytes).map_err(|_|"Binary Base64 output is unsupported in text mode")?;if s.chars().any(|c|c.is_control()&&!matches!(c,'\n'|'\r'|'\t')){return Err("Decoded value contains binary control characters.".into());}s},_=>return Err("Unknown text action".into())})
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn text_roundtrips() {
        for value in ["a+b / café", "hello\nworld", "😀"] {
            let encoded = text_action("url_encode", value).unwrap();
            assert_eq!(text_action("url_decode", &encoded).unwrap(), value);
            let encoded = text_action("base64_encode", value).unwrap();
            assert_eq!(text_action("base64_decode", &encoded).unwrap(), value);
        }
        assert!(text_action("url_decode", "%QZ").is_err());
        assert!(text_action("json_format", "{bad}").is_err());
    }
    #[test]
    fn catalog_unique() {
        let c = catalog();
        let mut ids = HashSet::new();
        for a in c {
            assert!(ids.insert(a.id));
        }
    }
    #[test]
    fn image_safety_and_output() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("a.png");
        image::RgbaImage::from_pixel(8, 6, image::Rgba([255, 0, 0, 100]))
            .save(&p)
            .unwrap();
        let original = fs::read(&p).unwrap();
        let msg = image_action("rotate_left", &p, &BTreeMap::new()).unwrap();
        assert!(msg.contains("a-rotated.png"), "{msg}");
        assert_eq!(fs::read(&p).unwrap(), original);
        let mut params = BTreeMap::new();
        params.insert("width".into(), "999".into());
        assert!(image_action("crop", &p, &params).is_err());
    }
    #[test]
    fn compress_and_scale_name_outputs_beside_the_source() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("photo.png");
        let noisy = image::RgbImage::from_fn(64, 48, |x, y| {
            image::Rgb([((x * 7) ^ (y * 13)) as u8, (x * y) as u8, (x + y * 3) as u8])
        });
        image::DynamicImage::ImageRgb8(noisy).save(&p).unwrap();
        let mut params = BTreeMap::new();
        params.insert("quality".into(), "50".into());
        let msg = image_action("compress", &p, &params).unwrap();
        assert!(msg.contains("photo-compressed.jpg"), "{msg}");
        let mut params = BTreeMap::new();
        params.insert("scale".into(), "50".into());
        image_action("resize", &p, &params).unwrap();
        let half = image::open(d.path().join("photo-32x24.png")).unwrap();
        assert_eq!((half.width(), half.height()), (32, 24));
        // Transparent pixels must survive compression.
        let t = d.path().join("logo.png");
        image::RgbaImage::from_fn(64, 64, |x, _| image::Rgba([x as u8 * 4, 0, 0, x as u8 * 4]))
            .save(&t)
            .unwrap();
        let msg = image_action("compress", &t, &BTreeMap::new()).unwrap();
        assert!(!msg.contains(".jpg"), "{msg}");
    }
    #[test]
    fn archive_roundtrip() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("a.txt");
        fs::write(&p, "hello").unwrap();
        crate::archive_tools::create(&[p], &BTreeMap::new()).unwrap();
        let z = d.path().join("a.zip");
        assert!(archive_action("test_zip", &z, |_, _| {})
            .unwrap()
            .contains("Verified"));
        archive_action("extract_zip", &z, |_, _| {}).unwrap();
        assert_eq!(
            fs::read_to_string(d.path().join("a/a.txt")).unwrap(),
            "hello"
        );
    }
    #[test]
    fn extraction_reports_progress_and_preserves_chunked_and_empty_entries() {
        let d = tempfile::tempdir().unwrap();
        let z = d.path().join("progress.zip");
        let data: Vec<u8> = (0..200_000).map(|i| (i % 251) as u8).collect();
        let mut zip = zip::ZipWriter::new(File::create(&z).unwrap());
        let options = zip::write::SimpleFileOptions::default();
        zip.start_file("large.bin", options).unwrap();
        zip.write_all(&data).unwrap();
        zip.start_file("empty.txt", options).unwrap();
        zip.add_directory("empty-dir/", options).unwrap();
        zip.finish().unwrap();
        let mut updates = Vec::new();
        archive_action("extract_zip", &z, |fraction, _| updates.push(fraction)).unwrap();
        assert_eq!(updates.first(), Some(&0.));
        assert_eq!(updates.last(), Some(&1.));
        assert!(updates.windows(2).all(|p| p[0] <= p[1]));
        assert!(updates
            .iter()
            .all(|f| f.is_finite() && (0. ..=1.).contains(f)));
        assert_eq!(fs::read(d.path().join("progress/large.bin")).unwrap(), data);
        assert_eq!(
            fs::metadata(d.path().join("progress/empty.txt"))
                .unwrap()
                .len(),
            0
        );
        assert!(d.path().join("progress/empty-dir").is_dir());

        let empty = d.path().join("empty.zip");
        zip::ZipWriter::new(File::create(&empty).unwrap())
            .finish()
            .unwrap();
        updates.clear();
        archive_action("extract_zip", &empty, |fraction, _| updates.push(fraction)).unwrap();
        assert_eq!(updates, [0., 1.]);
    }
}
fn contact_sheet(paths: &[PathBuf], params: &BTreeMap<String, String>) -> Result<String, String> {
    let columns = number(params, "columns", 4, 8)?;
    let cell = number(params, "cell", 240, 1000)?;
    if columns == 0 || cell < 32 {
        return Err("Use 1–8 columns and a cell size of 32–1000 px.".into());
    }
    let rows = (paths.len() as u32).div_ceil(columns);
    let width = columns * cell;
    let height = rows * cell;
    if width as u64 * height as u64 > 40_000_000 {
        return Err("Contact sheet exceeds 40 megapixels.".into());
    }
    let mut sheet = image::RgbaImage::from_pixel(width, height, image::Rgba([243, 235, 221, 255]));
    for (i, path) in paths.iter().enumerate() {
        crate::jobs::check()?;
        crate::jobs::progress(
            i,
            paths.len(),
            path.file_name().unwrap_or_default().to_string_lossy(),
        );
        output::plain_path(path)?;
        let img = load_image(path)?.thumbnail(cell - 16, cell - 16);
        let x = (i as u32 % columns) * cell + (cell - img.width()) / 2;
        let y = (i as u32 / columns) * cell + (cell - img.height()) / 2;
        image::imageops::overlay(&mut sheet, &img, x as i64, y as i64);
    }
    let source = &paths[0];
    let mut stage = output::Staged::new(source.parent().ok_or("No parent")?)?;
    image::DynamicImage::ImageRgba8(sheet)
        .write_to(&mut stage.file, image::ImageFormat::Png)
        .map_err(|e| e.to_string())?;
    Ok(format!(
        "Saved {}",
        stage.publish(source, "contact-sheet", "png")?.display()
    ))
}
fn images_pdf(paths: &[PathBuf], params: &BTreeMap<String, String>) -> Result<String, String> {
    // Raster-only PDF: fixed object identities, byte offsets and JPEG streams, no user-supplied PDF syntax.
    if paths.len() > 64 {
        return Err("Images-to-PDF supports at most 64 images.".into());
    }
    let page_size = params.get("page_size").map_or("a4", String::as_str);
    let orientation = params.get("orientation").map_or("portrait", String::as_str);
    if !matches!(page_size, "a4" | "letter" | "image")
        || !matches!(orientation, "portrait" | "landscape")
    {
        return Err("Invalid page size or orientation.".into());
    }
    let margin = number(params, "margin", 0, 144)? as f64;
    let quality = number(params, "quality", 90, 100)?;
    if quality == 0 {
        return Err("Quality must be 1–100.".into());
    }
    let source = &paths[0];
    let mut stage = output::Staged::new(source.parent().ok_or("No parent")?)?;
    let mut offsets = vec![0u64; 3 + paths.len() * 3];
    use std::io::Seek;
    stage
        .file
        .write_all(b"%PDF-1.4\n%\xE2\xE3\xCF\xD3\n")
        .map_err(|e| e.to_string())?;
    fn object(file: &mut File, offsets: &mut [u64], id: usize, body: &[u8]) -> Result<(), String> {
        use std::io::Seek;
        offsets[id] = file.stream_position().map_err(|e| e.to_string())?;
        writeln!(file, "{id} 0 obj").map_err(|e| e.to_string())?;
        file.write_all(body).map_err(|e| e.to_string())?;
        file.write_all(b"\nendobj\n").map_err(|e| e.to_string())?;
        Ok(())
    }
    object(
        &mut stage.file,
        &mut offsets,
        1,
        b"<< /Type /Catalog /Pages 2 0 R >>",
    )?;
    let kids = (0..paths.len())
        .map(|i| format!("{} 0 R", 3 + i * 3))
        .collect::<Vec<_>>()
        .join(" ");
    object(
        &mut stage.file,
        &mut offsets,
        2,
        format!("<< /Type /Pages /Count {} /Kids [{kids}] >>", paths.len()).as_bytes(),
    )?;
    for (i, path) in paths.iter().enumerate() {
        crate::jobs::check()?;
        crate::jobs::progress(
            i,
            paths.len(),
            path.file_name().unwrap_or_default().to_string_lossy(),
        );
        output::plain_path(path)?;
        let img = load_image(path)?;
        let rgba = img.to_rgba8();
        let rgb = image::RgbImage::from_fn(img.width(), img.height(), |x, y| {
            let c = rgba.get_pixel(x, y);
            let a = c[3] as u32;
            image::Rgb(std::array::from_fn(|i| {
                ((c[i] as u32 * a + 255 * (255 - a) + 127) / 255) as u8
            }))
        });
        let mut jpeg = Vec::new();
        image::codecs::jpeg::JpegEncoder::new_with_quality(&mut jpeg, quality as u8)
            .encode_image(&rgb)
            .map_err(|e| e.to_string())?;
        let page = 3 + i * 3;
        let image_id = page + 1;
        let content_id = page + 2;
        let (mut pw, mut ph) = match page_size {
            "letter" => (612.0, 792.0),
            "image" => (
                img.width() as f64 * (14000. / img.width().max(img.height()) as f64).min(1.)
                    + margin * 2.0,
                img.height() as f64 * (14000. / img.width().max(img.height()) as f64).min(1.)
                    + margin * 2.0,
            ),
            _ => (595.0, 842.0),
        };
        if page_size != "image" && orientation == "landscape" {
            std::mem::swap(&mut pw, &mut ph);
        }
        let scale = ((pw - margin * 2.0) / img.width() as f64)
            .min((ph - margin * 2.0) / img.height() as f64);
        let w = img.width() as f64 * scale;
        let h = img.height() as f64 * scale;
        object(&mut stage.file,&mut offsets,page,format!("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {pw:.3} {ph:.3}] /Resources << /XObject << /Im0 {image_id} 0 R >> >> /Contents {content_id} 0 R >>").as_bytes())?;
        let mut body=format!("<< /Type /XObject /Subtype /Image /Width {} /Height {} /ColorSpace /DeviceRGB /BitsPerComponent 8 /Filter /DCTDecode /Length {} >>\nstream\n",img.width(),img.height(),jpeg.len()).into_bytes();
        body.extend_from_slice(&jpeg);
        body.extend_from_slice(b"\nendstream");
        object(&mut stage.file, &mut offsets, image_id, &body)?;
        let content = format!(
            "q {w:.3} 0 0 {h:.3} {:.3} {:.3} cm /Im0 Do Q",
            (pw - w) / 2.0,
            (ph - h) / 2.0
        );
        object(
            &mut stage.file,
            &mut offsets,
            content_id,
            format!(
                "<< /Length {} >>\nstream\n{content}\nendstream",
                content.len()
            )
            .as_bytes(),
        )?;
        if stage.file.stream_position().map_err(|e| e.to_string())? > 512 * 1024 * 1024 {
            return Err("PDF output exceeds 512 MiB.".into());
        }
    }
    let start = stage.file.stream_position().map_err(|e| e.to_string())?;
    write!(
        stage.file,
        "xref\n0 {}\n0000000000 65535 f \n",
        offsets.len()
    )
    .map_err(|e| e.to_string())?;
    for offset in offsets.iter().skip(1) {
        writeln!(stage.file, "{offset:010} 00000 n ").map_err(|e| e.to_string())?;
    }
    write!(
        stage.file,
        "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{start}\n%%EOF\n",
        offsets.len()
    )
    .map_err(|e| e.to_string())?;
    Ok(format!(
        "Saved {} (configured pages, white background, selection order)",
        stage
            .publish_named(
                source.parent().ok_or("No parent")?,
                &source.file_stem().unwrap_or_default().to_string_lossy(),
                "pdf",
            )?
            .display()
    ))
}
fn folder_report(id: &str, path: &Path) -> Result<String, String> {
    use sha2::{Digest, Sha256};
    let mut stack = vec![(path.to_path_buf(), 0)];
    let mut files = Vec::new();
    let mut total = 0u64;
    let mut skipped = 0;
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
    while let Some((dir, depth)) = stack.pop() {
        crate::jobs::check()?;
        if depth > 64 || std::time::Instant::now() > deadline {
            return Err("Folder scan reached its depth or 30-second time limit.".into());
        }
        output::plain_path(&dir)?;
        let entries = match fs::read_dir(&dir) {
            Ok(v) => v,
            Err(_) => {
                skipped += 1;
                continue;
            }
        };
        for entry in entries {
            let entry = match entry {
                Ok(v) => v,
                Err(_) => {
                    skipped += 1;
                    continue;
                }
            };
            let p = entry.path();
            if output::plain_path(&p).is_err() {
                skipped += 1;
                continue;
            }
            let m = match fs::symlink_metadata(&p) {
                Ok(v) => v,
                Err(_) => {
                    skipped += 1;
                    continue;
                }
            };
            if m.is_dir() {
                stack.push((p, depth + 1));
            } else if m.is_file() {
                total = total.checked_add(m.len()).ok_or("Size overflow")?;
                files.push((p, m.len()));
            }
            if files.len() + stack.len() > 100_000 {
                return Err("Folder scan exceeds 100,000 entries.".into());
            }
        }
    }
    if id == "folder_size" {
        return Ok(format!("{}\n{} files\n{} bytes (logical, hard links counted separately)\n{} inaccessible/link items skipped",path.display(),files.len(),total,skipped));
    }
    let mut groups = BTreeMap::<u64, Vec<PathBuf>>::new();
    for (p, size) in files {
        groups.entry(size).or_default().push(p);
    }
    let mut hashes = BTreeMap::<String, Vec<PathBuf>>::new();
    let mut hashed = 0u64;
    for group in groups.values().filter(|g| g.len() > 1) {
        for p in group {
            crate::jobs::check()?;
            if std::time::Instant::now() > deadline {
                return Err("Duplicate scan reached its 30-second time limit.".into());
            }
            output::plain_path(p)?;
            let mut f = File::open(p).map_err(|e| e.to_string())?;
            let mut hash = Sha256::new();
            let mut buf = [0u8; 65536];
            loop {
                let n = f.read(&mut buf).map_err(|e| e.to_string())?;
                if n == 0 {
                    break;
                }
                hashed += n as u64;
                if hashed > 2 * 1024 * 1024 * 1024 || std::time::Instant::now() > deadline {
                    return Err("Duplicate scan reached its 2 GiB or 30-second limit.".into());
                }
                hash.update(&buf[..n]);
            }
            hashes
                .entry(format!("{:x}", hash.finalize()))
                .or_default()
                .push(p.clone());
        }
    }
    let mut report =
        format!("List only; no files changed. Skipped inaccessible/link items: {skipped}\n");
    for group in hashes.values().filter(|g| g.len() > 1) {
        report.push_str("\nMatching SHA-256:\n");
        for p in group {
            crate::jobs::check()?;
            report.push_str(&format!("{}\n", p.display()));
        }
        if report.len() > 256 * 1024 {
            return Err("Duplicate report exceeds display limit.".into());
        }
    }
    if !hashes.values().any(|g| g.len() > 1) {
        report.push_str("No duplicate content found.");
    }
    Ok(report)
}
#[cfg(test)]
mod boundary_tests {
    use super::*;
    #[test]
    fn pdf_page_controls_and_resize_modes_preserve_original() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("source.png");
        image::RgbaImage::from_pixel(101, 67, image::Rgba([255, 0, 0, 255]))
            .save(&path)
            .unwrap();
        let original = fs::read(&path).unwrap();
        image_action(
            "resize",
            &path,
            &BTreeMap::from([
                ("mode".into(), "percent".into()),
                ("percent".into(), "50".into()),
            ]),
        )
        .unwrap();
        let resized = image::open(dir.path().join("source-50x33.png")).unwrap();
        assert_eq!((resized.width(), resized.height()), (50, 33));
        image_action(
            "resize",
            &path,
            &BTreeMap::from([
                ("mode".into(), "longest".into()),
                ("longest".into(), "20".into()),
            ]),
        )
        .unwrap();
        let resized = image::open(dir.path().join("source-20x13.png")).unwrap();
        assert_eq!((resized.width(), resized.height()), (20, 13));
        images_pdf(
            std::slice::from_ref(&path),
            &BTreeMap::from([
                ("page_size".into(), "letter".into()),
                ("orientation".into(), "landscape".into()),
                ("margin".into(), "36".into()),
            ]),
        )
        .unwrap();
        let pdf = fs::read(dir.path().join("source.pdf")).unwrap();
        assert!(String::from_utf8_lossy(&pdf).contains("/MediaBox [0 0 792.000 612.000]"));
        let before = fs::read_dir(dir.path()).unwrap().count();
        assert!(images_pdf(
            std::slice::from_ref(&path),
            &BTreeMap::from([("quality".into(), "0".into())])
        )
        .is_err());
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), before);
        assert_eq!(fs::read(path).unwrap(), original);
    }
    #[test]
    fn new_tools_require_all_compatible_inputs_and_explicit_engines() {
        let dir = tempfile::tempdir().unwrap();
        let pdf = dir.path().join("a.pdf");
        let video = dir.path().join("b.mp4");
        let audio = dir.path().join("c.mp3");
        for path in [&pdf, &video, &audio] {
            fs::write(path, []).unwrap();
        }
        let catalog = catalog();
        let find = |id| catalog.iter().find(|a| a.id == id).unwrap();
        assert!(eligible(
            find("pdf_rotate"),
            std::slice::from_ref(&pdf),
            false
        ));
        assert!(!eligible(find("pdf_rotate"), &[pdf, video.clone()], false));
        assert!(eligible(
            find("video_convert"),
            std::slice::from_ref(&video),
            false
        ));
        assert!(!eligible(
            find("video_convert"),
            std::slice::from_ref(&audio),
            false
        ));
        assert!(eligible(find("audio_convert"), &[audio, video], false));
        assert!(missing_engine("video_convert", &crate::engines::Engines::new()).is_some());
        assert!(missing_engine("pdf_rotate", &crate::engines::Engines::new()).is_none());
        assert_eq!(catalog.len(), 68);
    }
    #[test]
    fn rejects_malicious_zip() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("bad.zip");
        let mut w = zip::ZipWriter::new(File::create(&p).unwrap());
        w.start_file("../outside.txt", zip::write::SimpleFileOptions::default())
            .unwrap();
        w.write_all(b"unsafe").unwrap();
        w.finish().unwrap();
        assert!(archive_action("extract_zip", &p, |_, _| {}).is_err());
        assert!(!d.path().join("outside.txt").exists());
    }
    #[test]
    fn pdf_xrefs_and_contact_sheet() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("input.png");
        image::RgbaImage::from_pixel(16, 8, image::Rgba([0, 100, 200, 255]))
            .save(&p)
            .unwrap();
        images_pdf(std::slice::from_ref(&p), &BTreeMap::new()).unwrap();
        let bytes = fs::read(d.path().join("input.pdf")).unwrap();
        let text = String::from_utf8_lossy(&bytes);
        assert!(text.contains("/Count 1"));
        let marker = b"\nxref\n";
        let xref = bytes
            .windows(marker.len())
            .position(|w| w == marker)
            .unwrap()
            + 1;
        let tail = String::from_utf8_lossy(&bytes[xref..]);
        let lines: Vec<_> = tail.lines().collect();
        for (id, line) in lines.iter().skip(3).take(5).enumerate() {
            let at = line[..10].parse::<usize>().unwrap();
            assert!(bytes[at..].starts_with(format!("{} 0 obj", id + 1).as_bytes()));
        }
        assert!(tail.contains(&format!("startxref\n{xref}\n")));
        contact_sheet(std::slice::from_ref(&p), &BTreeMap::new()).unwrap();
        let sheet = image::open(d.path().join("input-contact-sheet.png")).unwrap();
        assert_eq!((sheet.width(), sheet.height()), (960, 240));
    }
    #[test]
    fn folder_scan_is_read_only() {
        let d = tempfile::tempdir().unwrap();
        fs::write(d.path().join("one"), "same").unwrap();
        fs::write(d.path().join("two"), "same").unwrap();
        assert!(folder_report("folder_size", d.path())
            .unwrap()
            .contains("8 bytes"));
        let report = folder_report("find_duplicates", d.path()).unwrap();
        assert!(report.contains("Matching SHA-256"));
        assert_eq!(fs::read_dir(d.path()).unwrap().count(), 2);
    }
    #[test]
    fn eligibility_uses_every_target() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("a.png");
        let q = d.path().join("b.txt");
        fs::write(&p, []).unwrap();
        fs::write(&q, []).unwrap();
        let action = catalog().into_iter().find(|a| a.id == "crop").unwrap();
        assert!(!eligible(&action, &[p.clone(), q], false));
        let action = catalog()
            .into_iter()
            .find(|a| a.id == "copy_pixels")
            .unwrap();
        assert!(!eligible(&action, &[p.clone(), p], false));
    }
    #[test]
    fn invalid_parameters_do_not_write() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("a.png");
        image::RgbaImage::new(4, 4).save(&p).unwrap();
        let mut params = BTreeMap::new();
        params.insert("width".into(), "0".into());
        assert!(image_action("resize", &p, &params).is_err());
        params.insert("background".into(), "bad".into());
        assert!(image_action("frame", &p, &params).is_err());
        assert_eq!(fs::read_dir(d.path()).unwrap().count(), 1);
    }
}
