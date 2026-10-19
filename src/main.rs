#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod ui;
fn main() {
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
    if std::env::args().any(|a| a == "--worker") {
        pieall::actions::worker_main();
        return;
    }
    let paths = std::env::args_os()
        .skip(1)
        .map(|p| {
            let p = std::path::PathBuf::from(p);
            if p.is_absolute() {
                p
            } else {
                std::env::current_dir()
                    .map(|base| base.join(&p))
                    .unwrap_or(p)
            }
        })
        .collect();
    if let Err(e) = ui::run(paths) {
        pieall::platform::show_error(&e);
    }
}
use pieall::{actions, platform};
