//! Native-only first-use setup. Sources and filenames are never supplied by IPC.
use crate::{
    engines::{Engine, Engines},
    jobs::Progress,
};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File},
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};

const MAX_DOWNLOAD: u64 = 256 * 1024 * 1024;
#[derive(Deserialize)]
struct Release {
    assets: Vec<Asset>,
}
#[derive(Deserialize)]
struct Asset {
    name: String,
    size: u64,
    digest: Option<String>,
    browser_download_url: String,
}
struct Package {
    repo: &'static str,
    asset: &'static str,
    names: &'static [&'static str],
}
const MEDIA: Package = Package {
    repo: "BtbN/FFmpeg-Builds",
    asset: "ffmpeg-master-latest-win64-gpl.zip",
    names: &["ffmpeg", "ffprobe"],
};
const DOWNLOADER: Package = Package {
    repo: "yt-dlp/yt-dlp",
    asset: "yt-dlp.exe",
    names: &["yt-dlp"],
};
const RUNTIME: Package = Package {
    repo: "denoland/deno",
    asset: "deno-x86_64-pc-windows-msvc.zip",
    names: &["deno"],
};
fn check(cancel: &AtomicBool) -> Result<(), String> {
    if cancel.load(Ordering::SeqCst) {
        Err("Cancelled. Existing tools are kept.".into())
    } else {
        Ok(())
    }
}
fn digest(asset: &Asset, package: &Package) -> Result<String, String> {
    let hash = asset
        .digest
        .as_deref()
        .and_then(|s| s.strip_prefix("sha256:"))
        .filter(|s| s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit()))
        .ok_or("Release has no valid SHA-256 digest. Tool setup was stopped.")?;
    let url = url::Url::parse(&asset.browser_download_url).map_err(|e| e.to_string())?;
    if asset.name != package.asset
        || asset.size == 0
        || asset.size > MAX_DOWNLOAD
        || url.scheme() != "https"
        || url.host_str() != Some("github.com")
        || !url.username().is_empty()
        || url.password().is_some()
        || url.port().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
        || !url
            .path()
            .starts_with(&format!("/{}/releases/download/", package.repo))
        || !url.path().ends_with(&format!("/{}", package.asset))
    {
        return Err("Unexpected tool release asset.".into());
    }
    Ok(hash.to_ascii_lowercase())
}
fn report(progress: &impl Fn(Progress), detail: String, fraction: Option<f64>) {
    progress(Progress {
        completed: 0,
        total: 1,
        item: detail,
        fraction,
    });
}
fn extract(
    archive: &Path,
    destination: &Path,
    cancel: &AtomicBool,
) -> Result<Vec<PathBuf>, String> {
    let mut zip = zip::ZipArchive::new(File::open(archive).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    if zip.len() > 10000 {
        return Err("Tool archive has too many entries.".into());
    }
    let mut names = std::collections::HashSet::new();
    let mut total = 0u64;
    let mut paths = Vec::new();
    for i in 0..zip.len() {
        check(cancel)?;
        let mut entry = zip.by_index(i).map_err(|e| e.to_string())?;
        let relative = crate::output::safe_archive_name(entry.name())?;
        if relative.components().count() > 32
            || !names.insert(relative.to_string_lossy().to_lowercase())
            || entry.unix_mode().is_some_and(|m| {
                m & 0o170000 != 0 && m & 0o170000 != 0o100000 && m & 0o170000 != 0o040000
            })
        {
            return Err("Unsafe or duplicate tool archive entry.".into());
        }
        total = total
            .checked_add(entry.size())
            .ok_or("Tool archive size overflow")?;
        if entry.size() > MAX_DOWNLOAD || total > 1024 * 1024 * 1024 {
            return Err("Tool archive is too large.".into());
        }
        let path = destination.join(relative);
        if entry.is_dir() {
            fs::create_dir_all(&path).map_err(|e| e.to_string())?;
            continue;
        }
        fs::create_dir_all(path.parent().ok_or("Missing tool parent")?)
            .map_err(|e| e.to_string())?;
        let mut file = File::options()
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(|e| e.to_string())?;
        let mut remaining = entry.size();
        let mut buffer = [0u8; 65536];
        loop {
            check(cancel)?;
            let n = entry.read(&mut buffer).map_err(|e| e.to_string())?;
            if n == 0 {
                break;
            }
            remaining = remaining
                .checked_sub(n as u64)
                .ok_or("Tool archive entry exceeds its declared size")?;
            file.write_all(&buffer[..n]).map_err(|e| e.to_string())?;
        }
        if remaining != 0 {
            return Err("Incomplete tool archive entry.".into());
        }
        paths.push(path);
    }
    Ok(paths)
}
fn client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .tls_backend_native()
        .https_only(true)
        .connect_timeout(Duration::from_secs(15))
        .read_timeout(Duration::from_secs(30))
        .timeout(Duration::from_secs(900))
        .user_agent("PieAll-tool-setup")
        .build()
        .map_err(|e| e.to_string())
}
// Keep the request future alive across polls; cancellation drops the connection promptly.
async fn cancellable<T>(
    future: impl std::future::Future<Output = T>,
    cancel: &AtomicBool,
) -> Result<T, String> {
    let mut future = std::pin::pin!(future);
    loop {
        check(cancel)?;
        if let Ok(result) = tokio::time::timeout(Duration::from_millis(100), future.as_mut()).await
        {
            return Ok(result);
        }
    }
}
async fn download_archive(
    client: &reqwest::Client,
    asset: &Asset,
    path: &Path,
    cancel: &AtomicBool,
    progress: &impl Fn(Progress),
) -> Result<String, String> {
    let mut response = cancellable(client.get(&asset.browser_download_url).send(), cancel)
        .await?
        .and_then(reqwest::Response::error_for_status)
        .map_err(|e| format!("Cannot download tools: {e}"))?;
    let mut file = File::options()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|e| e.to_string())?;
    let mut hash = Sha256::new();
    let mut total = 0u64;
    let mut last = std::time::Instant::now() - Duration::from_secs(1);
    while let Some(bytes) = cancellable(response.chunk(), cancel)
        .await?
        .map_err(|e| format!("Tool download interrupted: {e}. Retry setup."))?
    {
        total += bytes.len() as u64;
        if total > asset.size {
            return Err("Tool download exceeds its declared size.".into());
        }
        hash.update(&bytes);
        file.write_all(&bytes).map_err(|e| e.to_string())?;
        if last.elapsed() >= Duration::from_millis(150) {
            report(
                progress,
                format!(
                    "Downloading tools: {} / {} MB",
                    total / 1_000_000,
                    asset.size / 1_000_000
                ),
                Some(total as f64 / asset.size as f64),
            );
            last = std::time::Instant::now();
        }
    }
    check(cancel)?;
    if total != asset.size {
        return Err("Incomplete tool download. Retry setup.".into());
    }
    file.sync_all().map_err(|e| e.to_string())?;
    Ok(format!("{:x}", hash.finalize()))
}
fn install(
    root: &Path,
    package: &Package,
    cancel: &AtomicBool,
    progress: &impl Fn(Progress),
) -> Result<(tempfile::TempDir, Engines), String> {
    check(cancel)?;
    if !cfg!(target_arch = "x86_64") {
        return Err("Automatic tools currently require 64-bit Intel/AMD Windows. Use Advanced to select your installation.".into());
    }
    report(progress, format!("Finding {}…", package.names[0]), None);
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| e.to_string())?;
    let client = client()?;
    let release: Release = runtime.block_on(async {
        let mut response = cancellable(
            client
                .get(format!(
                    "https://api.github.com/repos/{}/releases/latest",
                    package.repo
                ))
                .timeout(Duration::from_secs(30))
                .send(),
            cancel,
        )
        .await?
        .and_then(reqwest::Response::error_for_status)
        .map_err(|e| format!("Cannot find tool release: {e}"))?;
        let mut metadata = Vec::new();
        while let Some(bytes) = cancellable(response.chunk(), cancel)
            .await?
            .map_err(|e| e.to_string())?
        {
            if metadata.len() + bytes.len() > 2 * 1024 * 1024 {
                return Err("Tool release metadata is too large.".into());
            }
            metadata.extend_from_slice(&bytes);
        }
        serde_json::from_slice(&metadata).map_err(|e| e.to_string())
    })?;
    let asset = release
        .assets
        .into_iter()
        .find(|a| a.name == package.asset)
        .ok_or("Required Windows tool asset is unavailable")?;
    let expected = digest(&asset, package)?;
    check(cancel)?;
    let stage = tempfile::Builder::new()
        .prefix("tools-")
        .tempdir_in(root)
        .map_err(|e| e.to_string())?;
    let archive = stage.path().join(package.asset);
    if runtime.block_on(download_archive(
        &client, &asset, &archive, cancel, progress,
    ))? != expected
    {
        return Err("Tool download failed SHA-256 verification. Nothing was configured.".into());
    }
    report(progress, format!("Preparing {}…", package.names[0]), None);
    let paths = if package.asset.ends_with(".zip") {
        let paths = extract(&archive, &stage.path().join("distribution"), cancel)?;
        fs::remove_file(archive).map_err(|e| e.to_string())?;
        paths
    } else {
        vec![archive]
    };
    let mut engines = Engines::new();
    for name in package.names {
        let matches: Vec<_> = paths
            .iter()
            .filter(|p| {
                p.file_name()
                    .is_some_and(|n| n.eq_ignore_ascii_case(format!("{name}.exe")))
            })
            .collect();
        if matches.len() != 1 {
            return Err(format!("Tool release must contain one {name}.exe."));
        }
        engines.insert((*name).into(), Engine::select(name, matches[0])?);
    }
    if engines
        .get("ffmpeg")
        .zip(engines.get("ffprobe"))
        .is_some_and(|(a, b)| a.path.parent() != b.path.parent())
    {
        return Err("Media tools must share a distribution.".into());
    }
    fs::write(stage.path().join("release.json"), serde_json::to_vec_pretty(&serde_json::json!({"repository":package.repo,"asset":asset.name,"url":asset.browser_download_url,"sha256":expected})).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
    Ok((stage, engines))
}
/// All changes become visible through one database transaction, after every package verifies.
pub fn ensure(
    database: &Path,
    id: &str,
    engines: &mut Engines,
    cancel: &AtomicBool,
    progress: &impl Fn(Progress),
) -> Result<(), String> {
    let download = crate::downloads::is_download(id);
    if !download && !crate::actions::media_action(id) && id != "setup_tools" {
        return Ok(());
    }
    let root = database
        .parent()
        .ok_or("Missing settings folder")?
        .join("tools");
    fs::create_dir_all(&root).map_err(|e| e.to_string())?;
    crate::output::plain_path(&root)?;
    let mut next = engines.clone();
    // Preserve a custom media installation: choose its sibling rather than mix distributions.
    for (present, absent) in [("ffmpeg", "ffprobe"), ("ffprobe", "ffmpeg")] {
        if let Some(existing) = next.get(present).filter(|_| !next.contains_key(absent)) {
            let sibling = existing
                .path
                .parent()
                .ok_or("Missing custom tool folder")?
                .join(format!("{absent}.exe"));
            let engine = Engine::select(absent, &sibling).map_err(|_| "Your custom media installation is missing its matching tool. Choose a complete FFmpeg/FFprobe pair in Advanced, or remove both to use automatic setup.".to_string())?;
            next.insert(absent.into(), engine);
        }
    }
    let mut staged = Vec::new();
    for package in [&MEDIA, &DOWNLOADER, &RUNTIME] {
        if package.repo != MEDIA.repo && !download && id != "setup_tools" {
            continue;
        }
        if package.names.iter().all(|n| next.contains_key(*n)) {
            continue;
        }
        let installed = install(&root, package, cancel, progress);
        check(cancel)?;
        let (stage, records) = installed?;
        next.extend(records);
        staged.push(stage);
    }
    check(cancel)?;
    crate::engines::store_all(database, &next)?;
    for stage in staged {
        let _ = stage.keep();
    }
    *engines = next;
    report(progress, "Tools ready. Starting…".into(), None);
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn network_wait_can_be_cancelled_without_waiting_for_body_deadline() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .build()
            .unwrap();
        let cancelled = AtomicBool::new(false);
        let start = std::time::Instant::now();
        let result = runtime.block_on(cancellable(
            async {
                cancelled.store(true, Ordering::SeqCst);
                std::future::pending::<()>().await;
            },
            &cancelled,
        ));
        assert!(result.unwrap_err().starts_with("Cancelled."));
        assert!(start.elapsed() < Duration::from_secs(2));
    }
    #[test]
    fn setup_uses_the_enabled_native_tls_provider() {
        // tls_backend_native() must construct successfully with only native-tls enabled.
        assert!(client().is_ok());
    }
    #[test]
    fn validates_fixed_release_origin_and_digest() {
        let mut asset = Asset {
            name: DOWNLOADER.asset.into(),
            size: 100,
            digest: Some(format!("sha256:{}", "a".repeat(64))),
            browser_download_url:
                "https://github.com/yt-dlp/yt-dlp/releases/download/2026/yt-dlp.exe".into(),
        };
        assert!(digest(&asset, &DOWNLOADER).is_ok());
        asset.browser_download_url =
            "https://example.com/yt-dlp/yt-dlp/releases/download/2026/yt-dlp.exe".into();
        assert!(digest(&asset, &DOWNLOADER).is_err());
        asset.browser_download_url =
            "https://github.com/yt-dlp/yt-dlp/releases/download/2026/yt-dlp.exe".into();
        asset.digest = None;
        assert!(digest(&asset, &DOWNLOADER).is_err());
    }
    #[test]
    fn completes_custom_media_pair_without_network_or_replacing_it() {
        let dir = tempfile::tempdir().unwrap();
        let mut engines = Engines::new();
        for name in crate::engines::NAMES {
            let path = dir.path().join(format!("{name}.exe"));
            fs::write(&path, b"MZfixture").unwrap();
            engines.insert((*name).into(), Engine::select(name, &path).unwrap());
        }
        let original = engines["ffmpeg"].path.clone();
        engines.remove("ffprobe");
        let database = dir.path().join("settings.sqlite3");
        ensure(
            &database,
            "setup_tools",
            &mut engines,
            &AtomicBool::new(false),
            &|_| {},
        )
        .unwrap();
        assert_eq!(engines["ffmpeg"].path, original);
        assert_eq!(
            engines["ffprobe"].path.parent(),
            engines["ffmpeg"].path.parent()
        );
        assert_eq!(crate::engines::load(&database).unwrap().len(), 4);
        assert_eq!(fs::read_dir(dir.path().join("tools")).unwrap().count(), 0);
    }
    #[test]
    fn refuses_zip_escape_and_cancelled_setup_without_records() {
        let dir = tempfile::tempdir().unwrap();
        let archive = dir.path().join("test.zip");
        let mut zip = zip::ZipWriter::new(File::create(&archive).unwrap());
        zip.start_file("../escape.exe", zip::write::SimpleFileOptions::default())
            .unwrap();
        zip.write_all(b"MZfixture").unwrap();
        zip.finish().unwrap();
        assert!(extract(&archive, &dir.path().join("out"), &AtomicBool::new(false)).is_err());
        assert!(!dir.path().join("escape.exe").exists());
        let mut engines = Engines::new();
        assert!(ensure(
            &dir.path().join("settings.sqlite3"),
            "setup_tools",
            &mut engines,
            &AtomicBool::new(true),
            &|_| {}
        )
        .is_err());
        assert!(engines.is_empty());
        assert!(!dir.path().join("settings.sqlite3").exists());
    }
}
