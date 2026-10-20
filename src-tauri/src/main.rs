#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod hold;
mod shell;

fn main() {
    // Dispatch helpers before Tauri, so a codec/selection worker creates no WebView.
    if std::env::args().nth(1).as_deref() == Some("--capture") {
        let result = std::env::args()
            .nth(2)
            .and_then(|s| s.parse::<usize>().ok())
            .ok_or_else(|| "Invalid capture window".to_string())
            .and_then(pieall::platform::capture_selection_direct);
        if let Ok(bytes) = serde_json::to_vec(&result) {
            use std::io::Write;
            let _ = std::io::stdout().write_all(&bytes);
        }
        return;
    }
    if std::env::args().nth(1).as_deref() == Some("--worker") {
        pieall::actions::worker_main();
        return;
    }
    shell::run();
}
