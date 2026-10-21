//! Explicit single-URL downloads through pinned engines and a public-only proxy.
use crate::{
    engines::{Engine, Engines},
    jobs, output,
};
use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    io::{Read, Write},
    path::{Path, PathBuf},
    time::Duration,
};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Preferences {
    pub quality: String,
    pub container: String,
    pub audio: String,
}
impl Default for Preferences {
    fn default() -> Self {
        Self {
            quality: "best".into(),
            container: "mkv".into(),
            audio: "original".into(),
        }
    }
}
impl Preferences {
    pub fn validate(&self) -> Result<(), String> {
        if !["best", "2160", "1440", "1080", "720", "480"].contains(&self.quality.as_str())
            || !["mkv", "mp4", "webm"].contains(&self.container.as_str())
            || !["original", "mp3", "m4a", "opus", "flac", "wav"].contains(&self.audio.as_str())
        {
            return Err("Choose a listed download quality and format.".into());
        }
        Ok(())
    }
    pub fn with_overrides(&self, values: &BTreeMap<String, String>) -> Result<Self, String> {
        let mut p = self.clone();
        for (key, value) in values {
            match key.as_str() {
                "quality" => p.quality = value.clone(),
                "container" => p.container = value.clone(),
                "audio" => p.audio = value.clone(),
                _ => return Err("Unknown download option.".into()),
            }
        }
        p.validate()?;
        Ok(p)
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Settings {
    pub destination: PathBuf,
    pub preferences: Preferences,
}
pub fn load(database: &Path, default_folder: &Path) -> Result<Settings, String> {
    let db = crate::engines::database(database)?;
    let value: Option<String> = db
        .query_row("SELECT value FROM download_settings WHERE id=1", [], |r| {
            r.get(0)
        })
        .optional()
        .map_err(|e| e.to_string())?;
    let settings: Settings = match value {
        Some(value) => serde_json::from_str(&value).map_err(|_| "Download settings are invalid")?,
        None => Settings {
            destination: default_folder.to_owned(),
            preferences: Preferences::default(),
        },
    };
    settings.preferences.validate()?;
    Ok(settings)
}
pub fn store(database: &Path, settings: &Settings) -> Result<(), String> {
    settings.preferences.validate()?;
    output::plain_path(&settings.destination)?;
    if !settings.destination.is_dir() {
        return Err("Choose a download folder.".into());
    }
    let db = crate::engines::database(database)?;
    db.execute("INSERT INTO download_settings(id,value) VALUES(1,?1) ON CONFLICT(id) DO UPDATE SET value=excluded.value", params![serde_json::to_string(settings).map_err(|e| e.to_string())?]).map_err(|e| e.to_string())?;
    Ok(())
}
pub fn is_download(id: &str) -> bool {
    matches!(id, "download_video" | "download_audio")
}
fn engine<'a>(engines: &'a Engines, name: &str) -> Result<&'a Engine, String> {
    engines
        .get(name)
        .ok_or_else(|| format!("Choose {name} in Tool engines first."))
}
fn arguments(
    id: &str,
    url: &str,
    settings: &Settings,
    engines: &Engines,
    work: &Path,
    proxy: &str,
) -> Result<Vec<String>, String> {
    let url = crate::web_proxy::web_url(url)?;
    settings.preferences.validate()?;
    if !is_download(id) {
        return Err("Unknown download action.".into());
    }
    let ffmpeg = engine(engines, "ffmpeg")?;
    let ffprobe = engine(engines, "ffprobe")?;
    if ffmpeg.path.parent() != ffprobe.path.parent() {
        return Err("Choose FFmpeg and FFprobe from the same folder for downloads.".into());
    }
    let mut args: Vec<String> = [
        "--ignore-config",
        "--no-plugin-dirs",
        "--no-remote-components",
        "--no-js-runtimes",
        "--no-cache-dir",
        "--no-playlist",
        "--max-downloads",
        "1",
        "--break-per-input",
        "--playlist-items",
        "1",
        "--no-overwrites",
        "--restrict-filenames",
        "--windows-filenames",
        "--trim-filenames",
        "120",
        "--no-continue",
        "--downloader",
        "native",
        "--hls-prefer-native",
        "--socket-timeout",
        "30",
        "--retries",
        "3",
        "--fragment-retries",
        "3",
        "--concurrent-fragments",
        "1",
        "--max-filesize",
        "2G",
        "--no-write-thumbnail",
        "--no-write-info-json",
        "--no-write-subs",
        "--no-write-auto-subs",
        "--newline",
        "--no-quiet",
        "--no-colors",
        "--progress",
        "--progress-delta",
        "0.25",
        "--progress-template",
        "download:PIEALL_PROGRESS:%(progress.{status,downloaded_bytes,total_bytes})j",
        "--print",
        "after_move:PIEALL_RESULT:%(.{id,title,ext,width,height,vcodec,acodec})j",
        "--no-simulate",
        "--postprocessor-args",
        "ffmpeg_i:-protocol_whitelist file,pipe",
        "--postprocessor-args",
        "ffprobe:-protocol_whitelist file,pipe",
    ]
    .into_iter()
    .map(String::from)
    .collect();
    args.extend([
        "--proxy".into(),
        proxy.into(),
        "--geo-verification-proxy".into(),
        proxy.into(),
        "--ffmpeg-location".into(),
        ffmpeg
            .path
            .to_str()
            .ok_or("Engine path must be Unicode")?
            .into(),
        "--paths".into(),
        work.to_str()
            .ok_or("Download folder must be Unicode")?
            .into(),
        "--output".into(),
        "media.%(ext)s".into(),
    ]);
    if let Some(deno) = engines.get("deno") {
        args.extend([
            "--js-runtimes".into(),
            format!(
                "deno:{}",
                deno.path.to_str().ok_or("Engine path must be Unicode")?
            ),
        ]);
    }
    if id == "download_audio" {
        args.extend(["--format".into(), "bestaudio/best".into()]);
        args.extend([
            "--extract-audio".into(),
            "--audio-format".into(),
            if settings.preferences.audio == "original" {
                "best".into()
            } else {
                settings.preferences.audio.clone()
            },
            "--audio-quality".into(),
            "0".into(),
        ]);
    } else {
        let cap = if settings.preferences.quality == "best" {
            String::new()
        } else {
            format!("[height<={}]", settings.preferences.quality)
        };
        args.extend([
            "--format".into(),
            format!("bestvideo{cap}+bestaudio/best{cap}"),
            "--merge-output-format".into(),
            settings.preferences.container.clone(),
            "--remux-video".into(),
            settings.preferences.container.clone(),
        ]);
    }
    args.extend(["--".into(), url.into()]);
    Ok(args)
}
fn progress(line: &str) -> Option<(String, Option<f64>)> {
    let value: serde_json::Value =
        serde_json::from_str(line.trim().strip_prefix("PIEALL_PROGRESS:")?).ok()?;
    let bytes = value["downloaded_bytes"]
        .as_f64()
        .filter(|n| n.is_finite() && *n >= 0.)?;
    let total = value["total_bytes"]
        .as_f64()
        .filter(|n| n.is_finite() && *n > 0.);
    // Estimated totals can move backwards; show their bytes without a percentage.
    let fraction = if value["status"] == "finished" {
        None
    } else {
        total.map(|n| (bytes / n).clamp(0., 1.))
    };
    let detail = if value["status"] == "finished" {
        "Stream downloaded; preparing file…".into()
    } else {
        format!(
            "Downloading · {:.1} MiB{}",
            bytes / 1048576.,
            total.map_or(String::new(), |n| format!(" of {:.1} MiB", n / 1048576.))
        )
    };
    Some((detail, fraction))
}
fn filename(title: &str, id: &str) -> String {
    let clean = |s: &str, limit: usize| -> String {
        s.chars()
            .filter(|c| !c.is_control() && !"<>:\"/\\|?*".contains(*c))
            .take(limit)
            .collect::<String>()
            .trim_matches([' ', '.'])
            .to_string()
    };
    let title = clean(title, 60);
    let id = clean(id, 32);
    // Fixed prefix also prevents Windows device names such as CON and NUL.
    format!(
        "PieAll - {}{}",
        if title.is_empty() { "download" } else { &title },
        if id.is_empty() {
            String::new()
        } else {
            format!(" [{id}]")
        }
    )
}
pub fn run(id: &str, url: &str, settings: &Settings, engines: &Engines) -> Result<String, String> {
    crate::web_proxy::web_url(url)?;
    let _folder_guard = output::lock_parent(&settings.destination)?;
    let work = tempfile::Builder::new()
        .prefix(".pieall-download-")
        .tempdir_in(&settings.destination)
        .map_err(|e| e.to_string())?;
    let _guards: Vec<_> = engines
        .iter()
        .filter(|(name, _)| ["ffmpeg", "ffprobe", "deno"].contains(&name.as_str()))
        .map(|(_, e)| e.validated_file())
        .collect::<Result<_, _>>()?;
    let proxy = crate::web_proxy::Proxy::start()?;
    let args = arguments(id, url, settings, engines, work.path(), &proxy.url())?;
    jobs::progress(0, 1, "Reading the selected link…");
    let report = crate::engine_process::run(
        engine(engines, "yt-dlp")?,
        &args,
        Duration::from_secs(3600),
        Some(work.path()),
        |line| {
            if let Some((detail, fraction)) = progress(line) {
                jobs::progress_fraction(0, 1, detail, fraction);
            }
        },
    )?;
    let metadata: serde_json::Value = report
        .lines()
        .find_map(|l| {
            l.strip_prefix("PIEALL_RESULT:")
                .and_then(|v| serde_json::from_str(v).ok())
        })
        .ok_or("Downloader did not report a completed file. Nothing was published.")?;
    let dest = publish(work.path(), &metadata, &settings.destination)?;
    let ext = metadata["ext"].as_str().unwrap_or("unknown");
    jobs::progress(1, 1, "Download saved");
    // The toast takes Open location from the first line's absolute path; only that line may start with "Saved ".
    Ok(format!(
        "Saved {}\r\nSource video: {}×{} · {} · Source audio: {}\r\nFormat: {ext} · Quality: {} · Audio preference: {}",
        dest.display(),
        metadata["width"],
        metadata["height"],
        metadata["vcodec"],
        metadata["acodec"],
        settings.preferences.quality,
        settings.preferences.audio,
    ))
}
fn publish(
    work: &Path,
    metadata: &serde_json::Value,
    destination: &Path,
) -> Result<PathBuf, String> {
    let ext = metadata["ext"]
        .as_str()
        .ok_or("Missing downloaded format")?;
    if ![
        "mkv", "mp4", "webm", "m4a", "mp3", "opus", "ogg", "aac", "flac", "wav", "mov",
    ]
    .contains(&ext)
    {
        return Err("Downloaded media format is not supported. Nothing was published.".into());
    }
    let source = work.join(format!("media.{ext}"));
    output::plain_path(&source)?;
    let mut input = std::fs::File::open(&source).map_err(|e| e.to_string())?;
    let len = input.metadata().map_err(|e| e.to_string())?.len();
    if len == 0 || len > 2 * 1024 * 1024 * 1024 {
        return Err("Downloaded file is empty or exceeds 2 GiB.".into());
    }
    jobs::progress(0, 1, "Saving completed download…");
    let mut stage = output::Staged::new(destination)?;
    let mut chunk = [0; 65536];
    let mut copied = 0u64;
    loop {
        jobs::check()?;
        let n = input.read(&mut chunk).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        copied += n as u64;
        if copied > 2 * 1024 * 1024 * 1024 {
            return Err("Download grew beyond its file limit.".into());
        }
        stage
            .file
            .write_all(&chunk[..n])
            .map_err(|e| e.to_string())?;
    }
    let base = filename(
        metadata["title"].as_str().unwrap_or("download"),
        metadata["id"].as_str().unwrap_or(""),
    );
    stage.publish_named(destination, &base, ext)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn publication_is_controlled_non_replacing_and_bounded() {
        let dir = tempfile::tempdir().unwrap();
        let work = tempfile::tempdir().unwrap();
        let bytes = b"download fixture";
        std::fs::write(work.path().join("media.mp4"), bytes).unwrap();
        let metadata = serde_json::json!({"ext":"mp4", "title":"../CON: example?", "id":"a/b"});
        let first = publish(work.path(), &metadata, dir.path()).unwrap();
        let second = publish(work.path(), &metadata, dir.path()).unwrap();
        assert_ne!(first, second);
        assert_eq!(first.parent().unwrap(), dir.path());
        assert_eq!(std::fs::read(first).unwrap(), bytes);
        assert_eq!(std::fs::read(work.path().join("media.mp4")).unwrap(), bytes);
        assert!(publish(
            work.path(),
            &serde_json::json!({"ext":"../exe"}),
            dir.path()
        )
        .is_err());
        assert!(
            filename(&"😀".repeat(500), &"😀".repeat(500))
                .encode_utf16()
                .count()
                < 220
        );
    }
    #[test]
    fn defaults_migrate_and_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("settings.sqlite3");
        crate::engines::load(&db).unwrap();
        let mut s = load(&db, dir.path()).unwrap();
        assert_eq!(s.preferences.quality, "best");
        s.preferences.quality = "1080".into();
        store(&db, &s).unwrap();
        assert_eq!(
            load(&db, Path::new("ignored")).unwrap().preferences.quality,
            "1080"
        );
        s.preferences.container = "../evil".into();
        assert!(store(&db, &s).is_err());
    }
    #[test]
    fn options_do_not_become_commands_or_paths() {
        assert!(Preferences::default()
            .with_overrides(&BTreeMap::from([("destination".into(), "C:/".into())]))
            .is_err());
        assert_eq!(
            filename("../CON: example?", "a/b"),
            "PieAll - CON example [ab]"
        );
        assert!(
            progress("PIEALL_PROGRESS:{\"downloaded_bytes\":50,\"total_bytes\":100}")
                .unwrap()
                .1
                == Some(0.5)
        );
        assert!(
            progress("PIEALL_PROGRESS:{\"downloaded_bytes\":50,\"total_bytes_estimate\":100}")
                .unwrap()
                .1
                .is_none()
        );
    }
    #[test]
    fn highest_quality_and_native_networking_are_explicit() {
        let e = Engine {
            path: "C:/tools/ffmpeg.exe".into(),
            sha256: String::new(),
        };
        let engines = BTreeMap::from([
            ("ffmpeg".into(), e.clone()),
            (
                "ffprobe".into(),
                Engine {
                    path: "C:/tools/ffprobe.exe".into(),
                    ..e
                },
            ),
        ]);
        let s = Settings {
            destination: "C:/Downloads".into(),
            preferences: Preferences::default(),
        };
        let args = arguments(
            "download_video",
            "https://example.com/watch?v=1",
            &s,
            &engines,
            Path::new("C:/temp"),
            "http://127.0.0.1:9999",
        )
        .unwrap();
        for value in [
            "bestvideo+bestaudio/best",
            "--no-playlist",
            "--ignore-config",
            "--no-plugin-dirs",
            "--no-remote-components",
            "native",
            "ffmpeg_i:-protocol_whitelist file,pipe",
        ] {
            assert!(args.iter().any(|a| a == value), "{value}");
        }
        assert_eq!(args.last().unwrap(), "https://example.com/watch?v=1");
    }
}
