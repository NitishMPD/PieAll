pub mod actions;
pub mod app_icons;
pub mod archive_tools;
mod browser_capture;
pub mod downloads;
pub mod engine_install;
pub mod engine_process;
pub mod engines;
pub mod image_tools;
pub mod jobs;
pub mod media_tools;
pub mod output;
pub mod pdf_tools;
pub mod platform;
pub mod shortcuts;
#[cfg(feature = "legacy-ui")]
pub mod updates;
mod web_media;
pub use web_media::{
    media_thumbnail, scan as scan_media, scan_cancellable as scan_media_cancellable, single_video,
    MediaItem, MediaSelection,
};
pub mod web_proxy;
