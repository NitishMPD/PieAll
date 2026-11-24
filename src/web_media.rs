//! Bounded post/page media collection. No browser cookies or page scripts are used.
use crate::{
    downloads::{self, Settings},
    engines::Engines,
    jobs, output, web_proxy,
};
use scraper::{Html, Selector};
use std::{
    collections::HashSet,
    fs,
    io::Write,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};
use url::Url;

const MAX_FILES: usize = 100;
const MAX_BYTES: u64 = 2 * 1024 * 1024 * 1024;
const MAX_PAGE: usize = 8 * 1024 * 1024;

fn image_extension(ext: &str) -> bool {
    matches!(
        ext,
        "jpg" | "jpeg" | "png" | "gif" | "webp" | "avif" | "bmp" | "tif" | "tiff"
    )
}
fn media_extension(ext: &str) -> bool {
    image_extension(ext)
        || matches!(
            ext,
            "mp4"
                | "webm"
                | "mov"
                | "mkv"
                | "m4v"
                | "m4a"
                | "mp3"
                | "opus"
                | "ogg"
                | "aac"
                | "flac"
                | "wav"
        )
}
fn extension(url: &Url) -> String {
    Path::new(url.path())
        .extension()
        .unwrap_or_default()
        .to_string_lossy()
        .to_ascii_lowercase()
}
fn host_is(url: &Url, domain: &str) -> bool {
    url.host_str()
        .is_some_and(|h| h == domain || h.ends_with(&format!(".{domain}")))
}
fn linkedin_short(url: &Url) -> bool {
    matches!(url.host_str(), Some("lnkd.in" | "www.lnkd.in"))
}
fn social(url: &Url) -> bool {
    host_is(url, "x.com")
        || host_is(url, "twitter.com")
        || host_is(url, "linkedin.com")
        || pinterest(url)
}
fn pinterest(url: &Url) -> bool {
    host_is(url, "pin.it")
        || url.host_str().is_some_and(|h| {
            let h = h.strip_prefix("www.").unwrap_or(h);
            h.starts_with("pinterest.")
        })
}
fn post_url(input: &str) -> Result<Url, String> {
    let mut url = web_proxy::web_url(input)?;
    let parts: Vec<_> = url.path().trim_matches('/').split('/').collect();
    if host_is(&url, "x.com") || host_is(&url, "twitter.com") {
        let index = parts.iter().position(|p| *p == "status");
        let id = index
            .and_then(|i| parts.get(i + 1))
            .filter(|id| !id.is_empty() && id.bytes().all(|b| b.is_ascii_digit()));
        let id = id
            .ok_or("Open the specific X post first. Feeds and profiles are not downloaded.")?
            .to_string();
        url.set_path(&format!("/i/web/status/{id}"));
        url.set_query(None);
    } else if pinterest(&url) && !host_is(&url, "pin.it") {
        if parts.len() < 2 || parts[0] != "pin" || !parts[1].bytes().all(|b| b.is_ascii_digit()) {
            return Err(
                "Open a specific Pinterest pin first. Boards and feeds are not downloaded.".into(),
            );
        }
    } else if host_is(&url, "linkedin.com")
        && !matches!(
            parts.as_slice(),
            ["posts", slug] | ["feed", "update", slug] if !slug.is_empty()
        )
    {
        return Err(
            "Open the specific LinkedIn post first. Feeds and profiles are not downloaded.".into(),
        );
    }
    url.set_fragment(None);
    Ok(url)
}

fn linkedin_post_target(url: &Url) -> Result<Url, String> {
    if url.scheme() == "https" && host_is(url, "linkedin.com") {
        return post_url(url.as_str());
    }
    Err(
        "This LinkedIn link did not resolve to a public LinkedIn post. Try the full post URL."
            .into(),
    )
}

fn gallery_arguments(url: &Url, proxy: &str) -> Vec<String> {
    let mut args: Vec<String> = [
        "--config-ignore",
        "--no-input",
        "--no-colors",
        "--get-urls",
        "-o",
        "output.fallback=false",
        "--range",
        "1-101",
        "--post-range",
        "1",
        "--retries",
        "2",
        "--http-timeout",
        "30",
        "-o",
        "cache.file=null",
        "-o",
        "extractor.netrc=false",
        "-o",
        "extractor.cookies=null",
        "-o",
        "extractor.cookies-update=false",
        "-o",
        "extractor.twitter.conversations=false",
        "-o",
        "extractor.twitter.quoted=false",
        "-o",
        "extractor.twitter.expand=false",
        "-o",
        "extractor.twitter.cards=false",
        "-o",
        "extractor.twitter.ratelimit=abort",
        "-o",
        "extractor.twitter.videos=true",
        "-o",
        "extractor.ytdl.enabled=false",
        "-o",
        "extractor.postprocessors=[]",
    ]
    .into_iter()
    .map(String::from)
    .collect();
    args.extend(["--proxy".into(), proxy.into(), "--".into(), url.to_string()]);
    args
}

fn gallery_urls(text: &str) -> Result<Vec<Url>, String> {
    let mut urls = Vec::new();
    for line in text.lines().filter(|l| !l.trim().is_empty()) {
        let line = line.trim();
        // Older/custom engines may still include lower-resolution alternatives.
        // They belong to the preceding attachment, not additional media files.
        if line.starts_with("| ") {
            continue;
        }
        // gallery-dl prefixes manifests handled by yt-dlp with ytdl:.
        let line = line.strip_prefix("ytdl:").unwrap_or(line);
        let url = web_proxy::web_url(line)
            .map_err(|e| format!("The media extractor returned an invalid attachment URL: {e}"))?;
        if !urls.contains(&url) {
            urls.push(url);
        }
        if urls.len() > MAX_FILES {
            return Err("This post exceeds the 100-attachment limit.".into());
        }
    }
    Ok(urls)
}

#[derive(Default)]
struct PageMedia {
    images: Vec<Url>,
    videos: Vec<Url>,
}
fn add_url(list: &mut Vec<Url>, base: &Url, text: &str) {
    if list.len() > MAX_FILES {
        return;
    }
    if let Ok(mut url) = base.join(text.trim()) {
        url.set_fragment(None);
        if web_proxy::web_url(url.as_str()).is_ok() && !list.contains(&url) {
            list.push(url);
        }
    }
}
fn selector(value: &str) -> Selector {
    Selector::parse(value).expect("static media selector")
}
fn page_media(html: &str, base: &Url) -> PageMedia {
    let doc = Html::parse_document(html);
    let mut media = PageMedia::default();
    // Prefer the post body over recommendations, navigation, avatars and comments.
    let scope = [
        ".main-feed-activity-card",
        ".share-update-card",
        ".feed-shared-update-v2",
        "article",
        "main",
    ]
    .iter()
    .find_map(|s| doc.select(&selector(s)).next())
    .unwrap_or(doc.root_element());
    for img in scope.select(&selector("img, picture source")) {
        let e = img.value();
        let class = e.attr("class").unwrap_or("").to_ascii_lowercase();
        if ["avatar", "profile", "logo", "emoji", "icon"]
            .iter()
            .any(|c| class.contains(c))
        {
            continue;
        }
        if img
            .ancestors()
            .filter_map(scraper::ElementRef::wrap)
            .any(|parent| {
                let tag = parent.value();
                matches!(tag.name(), "nav" | "header" | "footer" | "aside")
                    || tag.attr("class").is_some_and(|c| {
                        let c = c.to_ascii_lowercase();
                        ["avatar", "profile", "__actor", "__author", "recommend"]
                            .iter()
                            .any(|word| c.contains(word))
                    })
            })
        {
            continue;
        }
        if ["width", "height"].iter().any(|a| {
            e.attr(a)
                .and_then(|v| v.parse::<u32>().ok())
                .is_some_and(|n| n <= 32)
        }) {
            continue;
        }
        let largest = e
            .attr("srcset")
            .or_else(|| e.attr("data-srcset"))
            .and_then(|set| {
                set.split(',')
                    .filter_map(|candidate| {
                        let mut parts = candidate.split_whitespace();
                        let url = parts.next()?;
                        let size = parts
                            .next()
                            .unwrap_or("1x")
                            .trim_end_matches(['w', 'x'])
                            .parse::<f64>()
                            .unwrap_or(0.);
                        Some((url, size))
                    })
                    .max_by(|a, b| a.1.total_cmp(&b.1))
                    .map(|(url, _)| url)
            });
        if let Some(url) = e
            .attr("data-delayed-url")
            .or(e.attr("data-src"))
            .or(largest)
            .or(e.attr("src"))
        {
            add_url(&mut media.images, base, url);
        }
    }
    let video_sources = selector("source[src]");
    for video in scope.select(&selector("video")) {
        if host_is(base, "linkedin.com") {
            if let Some(url) = video
                .value()
                .attr("data-sources")
                .and_then(|sources| linkedin_video_source(sources, base))
            {
                add_url(&mut media.videos, base, url.as_str());
                continue;
            }
        }
        if let Some(url) = video.value().attr("src") {
            add_url(&mut media.videos, base, url);
        }
        for source in video.select(&video_sources) {
            if let Some(url) = source.value().attr("src") {
                add_url(&mut media.videos, base, url);
            }
        }
    }
    // Structured post attachments can include carousel images absent from the HTML.
    for script in doc.select(&selector("script[type='application/ld+json']")) {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&script.inner_html()) {
            json_media(&json, base, &mut media, 0);
        }
    }
    if media.images.is_empty() {
        for tag in doc.select(&selector(
            "meta[property='og:image'], meta[property='og:image:url'], meta[name='twitter:image']",
        )) {
            if let Some(url) = tag.value().attr("content") {
                add_url(&mut media.images, base, url);
            }
        }
    }
    media
}

fn linkedin_video_source(sources: &str, base: &Url) -> Option<Url> {
    #[derive(serde::Deserialize)]
    struct Source {
        src: Option<String>,
        #[serde(rename = "type")]
        mime: Option<String>,
        #[serde(rename = "data-bitrate")]
        bitrate: Option<u64>,
    }
    serde_json::from_str::<Vec<Source>>(sources)
        .ok()?
        .into_iter()
        .filter_map(|source| {
            let url = base.join(source.src?.trim()).ok()?;
            if source.mime.as_deref() != Some("video/mp4") && extension(&url) != "mp4" {
                return None;
            }
            let url = web_proxy::web_url(url.as_str()).ok()?;
            Some((source.bitrate.unwrap_or(0), url))
        })
        .max_by_key(|(bitrate, _)| *bitrate)
        .map(|(_, url)| url)
}

fn json_media(value: &serde_json::Value, base: &Url, media: &mut PageMedia, depth: usize) {
    if depth > 24 {
        return;
    }
    match value {
        serde_json::Value::Array(values) => {
            for v in values {
                json_media(v, base, media, depth + 1);
            }
        }
        serde_json::Value::Object(obj) => {
            let kind = obj.get("@type").and_then(|v| v.as_str()).unwrap_or("");
            if matches!(kind, "ImageObject" | "VideoObject") {
                if let Some(url) = obj
                    .get("contentUrl")
                    .or_else(|| obj.get("url"))
                    .and_then(|v| v.as_str())
                {
                    add_url(
                        if kind == "ImageObject" {
                            &mut media.images
                        } else {
                            &mut media.videos
                        },
                        base,
                        url,
                    );
                }
            }
            for (key, v) in obj {
                if matches!(
                    key.as_str(),
                    "image"
                        | "associatedMedia"
                        | "video"
                        | "@graph"
                        | "mainEntity"
                        | "sharedContent"
                        | "hasPart"
                ) {
                    if key == "image" {
                        match v {
                            serde_json::Value::String(s) => add_url(&mut media.images, base, s),
                            serde_json::Value::Array(items) => {
                                for s in items.iter().filter_map(|v| v.as_str()) {
                                    add_url(&mut media.images, base, s);
                                }
                            }
                            _ => {}
                        }
                    }
                    json_media(v, base, media, depth + 1);
                }
            }
        }
        _ => {}
    }
}

async fn wait<T>(future: impl std::future::Future<Output = T>) -> Result<T, String> {
    let mut future = std::pin::pin!(future);
    loop {
        jobs::check()?;
        if let Ok(result) = tokio::time::timeout(Duration::from_millis(100), future.as_mut()).await
        {
            return Ok(result);
        }
    }
}
struct Collector<'a> {
    client: reqwest::Client,
    work: &'a Path,
    files: Vec<PathBuf>,
    bytes: u64,
    started: Instant,
}
impl Collector<'_> {
    fn check(&self) -> Result<(), String> {
        jobs::check()?;
        if self.started.elapsed() > Duration::from_secs(3600) {
            return Err("Media collection exceeded one hour.".into());
        }
        if self.files.len() >= MAX_FILES {
            return Err("This page exceeds the 100-attachment limit.".into());
        }
        Ok(())
    }
    async fn response(&self, url: &Url, referer: &Url) -> Result<reqwest::Response, String> {
        self.check()?;
        let response = wait(
            self.client
                .get(url.clone())
                .header(reqwest::header::REFERER, referer.as_str())
                .send(),
        )
        .await?
        .and_then(reqwest::Response::error_for_status)
        .map_err(|e| format!("Cannot read media: {e}"))?;
        web_proxy::web_url(response.url().as_str())?;
        Ok(response)
    }
    async fn save_response(&mut self, mut response: reqwest::Response) -> Result<(), String> {
        self.check()?;
        let mime = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .split(';')
            .next()
            .unwrap_or("")
            .trim();
        let ext = match mime {
            "image/jpeg" => "jpg",
            "image/png" => "png",
            "image/gif" => "gif",
            "image/webp" => "webp",
            "image/avif" => "avif",
            "image/bmp" => "bmp",
            "image/tiff" => "tiff",
            "video/mp4" => "mp4",
            "video/webm" => "webm",
            "video/quicktime" => "mov",
            "application/octet-stream" | "" => {
                let ext = extension(response.url());
                if !media_extension(&ext) {
                    return Err("The attachment has no supported media format.".into());
                }
                return self.save_body(&mut response, &ext).await;
            }
            _ => {
                return Err(
                    "The attachment returned a login page or unsupported media format.".into(),
                )
            }
        };
        if response
            .content_length()
            .is_some_and(|len| len > MAX_BYTES - self.bytes)
        {
            return Err("Media collection exceeds 2 GiB.".into());
        }
        self.save_body(&mut response, ext).await
    }
    async fn save_body(
        &mut self,
        response: &mut reqwest::Response,
        ext: &str,
    ) -> Result<(), String> {
        let path = self
            .work
            .join(format!("attachment-{:03}.{ext}", self.files.len() + 1));
        let mut file = fs::File::options()
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(|e| e.to_string())?;
        let mut length = 0u64;
        let result: Result<(), String> = async {
            while let Some(chunk) = wait(response.chunk()).await?.map_err(|e| e.to_string())? {
                self.check()?;
                length += chunk.len() as u64;
                if length > MAX_BYTES - self.bytes {
                    return Err("Media collection exceeds 2 GiB.".into());
                }
                file.write_all(&chunk).map_err(|e| e.to_string())?;
            }
            if length == 0 {
                return Err("The attachment is empty.".into());
            }
            file.sync_all().map_err(|e| e.to_string())?;
            Ok(())
        }
        .await;
        drop(file);
        if let Err(error) = result {
            let _ = fs::remove_file(&path);
            return Err(error);
        }
        self.bytes += length;
        self.files.push(path);
        jobs::progress(
            self.files.len(),
            self.files.len() + 1,
            format!("{} attachments downloaded", self.files.len()),
        );
        Ok(())
    }
    fn import(&mut self, paths: Vec<PathBuf>) -> Result<(), String> {
        for source in paths {
            self.check()?;
            output::plain_path(&source)?;
            let ext = source
                .extension()
                .unwrap_or_default()
                .to_string_lossy()
                .to_ascii_lowercase();
            if !media_extension(&ext) {
                return Err("Unsupported downloaded media format.".into());
            }
            let len = fs::metadata(&source).map_err(|e| e.to_string())?.len();
            if len == 0 || len > MAX_BYTES - self.bytes {
                return Err("Media collection is empty or exceeds 2 GiB.".into());
            }
            let path = self
                .work
                .join(format!("attachment-{:03}.{ext}", self.files.len() + 1));
            fs::hard_link(source, &path).map_err(|e| e.to_string())?;
            self.bytes += len;
            self.files.push(path);
        }
        Ok(())
    }
}

fn publish(files: &[PathBuf], destination: &Path, base: &str) -> Result<PathBuf, String> {
    if files.is_empty() || files.len() > MAX_FILES {
        return Err("No supported attachments were downloaded.".into());
    }
    for n in 0..10000 {
        let folder = destination.join(if n == 0 {
            base.to_string()
        } else {
            format!("{base} ({})", n + 1)
        });
        let made = jobs::publish(|| match fs::create_dir(&folder) {
            Ok(()) => Ok(true),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => Ok(false),
            Err(e) => Err(e.to_string()),
        })?;
        if !made {
            continue;
        }
        let mut linked = Vec::new();
        let result = (|| {
            let _guard = output::lock_parent(&folder)?;
            for source in files {
                jobs::check()?;
                output::plain_path(source)?;
                let target = folder.join(source.file_name().ok_or("Missing attachment name")?);
                jobs::publish(|| fs::hard_link(source, &target).map_err(|e| e.to_string()))?;
                linked.push(target);
            }
            Ok(())
        })();
        if let Err(error) = result {
            for path in linked {
                let _ = fs::remove_file(path);
            }
            let _ = fs::remove_dir(&folder);
            return Err(error);
        }
        return Ok(folder);
    }
    Err("Too many download folder name collisions.".into())
}

pub fn run(input: &str, settings: &Settings, engines: &Engines) -> Result<String, String> {
    let mut url = post_url(input)?;
    settings.preferences.validate()?;
    let _guard = output::lock_parent(&settings.destination)?;
    let work = tempfile::Builder::new()
        .prefix(".pieall-media-")
        .tempdir_in(&settings.destination)
        .map_err(|e| e.to_string())?;
    let proxy = web_proxy::Proxy::start()?;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| e.to_string())?;
    let redirect_limit = reqwest::redirect::Policy::limited(8);
    let client = reqwest::Client::builder().tls_backend_native().no_proxy()
        .proxy(reqwest::Proxy::all(proxy.url()).map_err(|e| e.to_string())?)
        .redirect(reqwest::redirect::Policy::custom(move |attempt| {
            if attempt.previous().first().is_some_and(linkedin_short)
                && !linkedin_short(attempt.url())
            {
                // Validate the destination before sending a request to it.
                attempt.stop()
            } else {
                redirect_limit.redirect(attempt)
            }
        }))
        .connect_timeout(Duration::from_secs(15)).read_timeout(Duration::from_secs(30)).timeout(Duration::from_secs(180))
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 Chrome/131.0.0.0 Safari/537.36")
        .build().map_err(|e| e.to_string())?;
    let mut collector = Collector {
        client,
        work: work.path(),
        files: Vec::new(),
        bytes: 0,
        started: Instant::now(),
    };
    let mut warnings = Vec::new();
    let mut urls = Vec::new();
    jobs::progress(0, 1, "Finding post attachments…");
    if linkedin_short(&url) {
        url = runtime.block_on(async {
            let response = collector.response(&url, &url).await?;
            let target = response
                .headers()
                .get(reqwest::header::LOCATION)
                .filter(|_| response.status().is_redirection())
                .and_then(|value| value.to_str().ok())
                .and_then(|location| response.url().join(location).ok());
            linkedin_post_target(target.as_ref().unwrap_or(response.url()))
        })?;
    }
    // Extractors find original image/GIF/video URLs; all file IO stays native.
    if !host_is(&url, "linkedin.com") && !media_extension(&extension(&url)) {
        if let Some(engine) = engines.get("gallery-dl") {
            match crate::engine_process::run_complete(
                engine,
                &gallery_arguments(&url, &proxy.url()),
                Duration::from_secs(120),
                None,
                |_| {},
            ) {
                Ok(report) => urls = gallery_urls(&report)?,
                Err(error) => {
                    jobs::check()?;
                    warnings.push(format!("Gallery extraction unavailable: {error}"));
                }
            }
        }
    }
    let gallery_found = !urls.is_empty();
    let mut page = PageMedia::default();
    let mut direct_media = false;
    let mut explicit_videos = false;
    if !gallery_found {
        let page_result: Result<(), String> = runtime.block_on(async {
            let mut response = collector.response(&url, &url).await?;
            let final_url = if host_is(&url, "linkedin.com") {
                linkedin_post_target(response.url())?
            } else {
                post_url(response.url().as_str())?
            };
            let content_type = response
                .headers()
                .get(reqwest::header::CONTENT_TYPE)
                .and_then(|v| v.to_str().ok())
                .unwrap_or("");
            if content_type.starts_with("image/")
                || content_type.starts_with("video/")
                || image_extension(&extension(&final_url))
            {
                collector.save_response(response).await?;
                direct_media = true;
            } else {
                let mut bytes = Vec::new();
                while let Some(chunk) = wait(response.chunk()).await?.map_err(|e| e.to_string())? {
                    if bytes.len() + chunk.len() > MAX_PAGE {
                        return Err("The page exceeds the 8 MiB discovery limit.".into());
                    }
                    bytes.extend_from_slice(&chunk);
                }
                page = page_media(&String::from_utf8_lossy(&bytes), &final_url);
                if page.images.len() + page.videos.len() > MAX_FILES {
                    return Err("This page exceeds the 100-attachment limit.".into());
                }
            }
            Ok(())
        });
        if let Err(error) = page_result {
            jobs::check()?;
            warnings.push(error);
        }
        urls.append(&mut page.images);
        explicit_videos = !page.videos.is_empty();
        urls.append(&mut page.videos);
    }
    let mut failures = Vec::new();
    let mut seen = HashSet::new();
    for media_url in urls {
        if !seen.insert(media_url.to_string()) {
            continue;
        }
        collector.check()?;
        if matches!(extension(&media_url).as_str(), "m3u8" | "mpd") {
            page.videos.push(media_url);
            continue;
        }
        if let Err(error) = runtime.block_on(async {
            let response = collector.response(&media_url, &url).await?;
            let mime = response
                .headers()
                .get(reqwest::header::CONTENT_TYPE)
                .and_then(|v| v.to_str().ok())
                .unwrap_or("")
                .split(';')
                .next()
                .unwrap_or("");
            if matches!(
                mime,
                "application/vnd.apple.mpegurl" | "application/x-mpegurl" | "application/dash+xml"
            ) || gallery_found && mime == "text/html"
            {
                page.videos.push(media_url.clone());
                return Ok(());
            }
            collector.save_response(response).await
        }) {
            jobs::check()?;
            failures.push(error);
        }
    }
    // Generic extractors handle embedded players and separate audio/video streams.
    if !gallery_found
        && !direct_media
        && !explicit_videos
        && !image_extension(&extension(&url))
        && page.videos.is_empty()
    {
        page.videos.push(url.clone());
    }
    for video in page.videos {
        collector.check()?;
        let stage = tempfile::Builder::new()
            .prefix(".pieall-video-")
            .tempdir_in(&settings.destination)
            .map_err(|e| e.to_string())?;
        match downloads::download_videos(
            video.as_str(),
            settings,
            engines,
            stage.path(),
            &proxy.url(),
        ) {
            Ok(paths) => collector.import(paths)?,
            Err(error) => {
                jobs::check()?;
                if gallery_found || seen.contains(video.as_str()) {
                    failures.push(error);
                } else {
                    warnings.push(format!("Video extraction: {error}"));
                }
            }
        }
    }
    if collector.files.is_empty() {
        return Err(format!("No downloadable media found. Open the exact post or pin, or use a direct image URL. Login-only and protected media are not supported.\r\n{}", warnings.into_iter().chain(failures).collect::<Vec<_>>().join("\r\n")));
    }
    if !gallery_found && social(&url) {
        warnings.push("Public-page fallback was used. Login-only or dynamically loaded attachments may be missing.".into());
    }
    if !failures.is_empty() {
        warnings.push(format!(
            "{} attachment(s) could not be saved:\r\n{}",
            failures.len(),
            failures.join("\r\n")
        ));
    }
    let id = url
        .path_segments()
        .and_then(|mut s| s.rfind(|s| !s.is_empty()))
        .unwrap_or("page");
    let base = format!(
        "PieAll - {} - {}",
        url.host_str().unwrap_or("media"),
        id.chars()
            .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_'))
            .take(60)
            .collect::<String>()
    );
    let folder = publish(&collector.files, &settings.destination, &base)?;
    jobs::progress(collector.files.len(), collector.files.len(), "Media saved");
    Ok(format!(
        "Saved {}\r\n{} media file(s) · {:.1} MiB\r\nSource: {}{}",
        folder.display(),
        collector.files.len(),
        collector.bytes as f64 / 1048576.,
        url,
        if warnings.is_empty() {
            String::new()
        } else {
            format!("\r\nNote: {}", warnings.join("\r\n"))
        }
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn post_scope_excludes_feeds_and_includes_every_x_attachment() {
        assert_eq!(
            post_url("https://x.com/person/status/123/photo/2?s=1")
                .unwrap()
                .as_str(),
            "https://x.com/i/web/status/123"
        );
        for url in [
            "https://x.com/home",
            "https://twitter.com/person",
            "https://www.pinterest.com/person/board/",
            "https://www.linkedin.com/feed/",
        ] {
            assert!(post_url(url).is_err(), "{url}");
        }
        assert!(post_url("https://www.linkedin.com/feed/update/urn:li:activity:123").is_ok());
        assert!(post_url("https://www.pinterest.com/pin/123/").is_ok());
    }
    #[test]
    fn linkedin_share_target_must_be_a_linkedin_post() {
        let short = post_url("https://lnkd.in/p/example").unwrap();
        assert!(linkedin_short(&short));
        let destination = short
            .join(
                "https://www.linkedin.com/posts/example-ugcPost-123-wa_O/?origin=SOCIAL_SHARE#post",
            )
            .unwrap();
        assert_eq!(
            linkedin_post_target(&destination).unwrap().as_str(),
            "https://www.linkedin.com/posts/example-ugcPost-123-wa_O/?origin=SOCIAL_SHARE"
        );
        for url in [
            "https://www.linkedin.com/posts/",
            "https://www.linkedin.com/feed/",
            "https://www.linkedin.com/authwall",
            "https://example.com/posts/example",
            "http://www.linkedin.com/posts/example",
        ] {
            assert!(
                linkedin_post_target(&Url::parse(url).unwrap()).is_err(),
                "{url}"
            );
        }
    }
    #[test]
    fn linkedin_native_player_downloads_one_valid_source_per_player() {
        let page = page_media(
            r#"<article>
                <video src="/low.mp4" data-sources="[
                    {&quot;src&quot;:&quot;/low.mp4&quot;,&quot;data-bitrate&quot;:58062},
                    {&quot;src&quot;:&quot;https://media.licdn.com/dms/video/asset?e=1&amp;t=signed&quot;,&quot;type&quot;:&quot;video/mp4&quot;,&quot;data-bitrate&quot;:142650},
                    {&quot;src&quot;:&quot;http://127.0.0.1/private.mp4&quot;,&quot;data-bitrate&quot;:999999}
                ]"><source src="/low.mp4"></video>
                <video data-sources="invalid"><source src="/another.mp4"></video>
            </article><video src="/outside.mp4"></video>"#,
            &Url::parse("https://www.linkedin.com/posts/example").unwrap(),
        );
        assert_eq!(
            page.videos.iter().map(Url::as_str).collect::<Vec<_>>(),
            [
                "https://media.licdn.com/dms/video/asset?e=1&t=signed",
                "https://www.linkedin.com/another.mp4",
            ]
        );
    }
    #[test]
    fn public_post_collects_carousel_and_animation_without_avatars_or_recommendations() {
        let page = page_media(
            r#"<img src='/outside.jpg'><article><img class='avatar' src='/avatar.jpg'><img src='/small.jpg' srcset='/large.jpg 2000w, /small.jpg 200w'><img data-delayed-url='/animation.gif' src='/placeholder.png'><video><source src='/movie.mp4'></video></article><script type='application/ld+json'>{"@type":"SocialMediaPosting","image":["/large.jpg",{"@type":"ImageObject","contentUrl":"/third.webp"}]}</script>"#,
            &Url::parse("https://www.linkedin.com/posts/test").unwrap(),
        );
        assert_eq!(
            page.images.iter().map(|u| u.path()).collect::<Vec<_>>(),
            ["/large.jpg", "/animation.gif", "/third.webp"]
        );
        assert_eq!(page.videos[0].path(), "/movie.mp4");
    }
    #[test]
    fn gallery_urls_are_deduplicated_bounded_and_public() {
        assert_eq!(gallery_urls("https://example.com/a.gif\nhttps://example.com/a.gif\nytdl:https://example.com/a.m3u8\n").unwrap().len(), 2);
        assert!(gallery_urls("file:///secret").is_err());
        assert!(gallery_urls("http://127.0.0.1/private").is_err());
        let many = (0..101)
            .map(|n| format!("https://example.com/{n}.png\n"))
            .collect::<String>();
        assert!(gallery_urls(&many).is_err());
    }
    #[test]
    fn gallery_image_fallbacks_are_not_parsed_as_extra_attachments() {
        let report = "https://pbs.twimg.com/media/example?format=jpg&name=orig\n| https://pbs.twimg.com/media/example?format=jpg&name=4096x4096\n| https://pbs.twimg.com/media/example?format=jpg&name=large\nhttps://pbs.twimg.com/media/second?format=png&name=orig\n";
        let urls = gallery_urls(report).unwrap();
        assert_eq!(urls.len(), 2);
        assert!(urls
            .iter()
            .all(|u| u.query().unwrap().contains("name=orig")));
        assert!(gallery_urls("not a URL")
            .unwrap_err()
            .starts_with("The media extractor returned an invalid attachment URL:"));
    }
    #[test]
    fn grouped_publication_never_overwrites_and_rolls_back_failed_group() {
        let stage = tempfile::tempdir().unwrap();
        let dest = tempfile::tempdir().unwrap();
        let gif = stage.path().join("attachment-001.gif");
        fs::write(&gif, b"GIF89a animation bytes").unwrap();
        let one = publish(std::slice::from_ref(&gif), dest.path(), "post").unwrap();
        let two = publish(std::slice::from_ref(&gif), dest.path(), "post").unwrap();
        assert_ne!(one, two);
        assert_eq!(
            fs::read(one.join("attachment-001.gif")).unwrap(),
            b"GIF89a animation bytes"
        );
        assert!(publish(&[gif, stage.path().join("missing")], dest.path(), "failed").is_err());
        assert!(!dest.path().join("failed").exists());
    }
    #[test]
    fn native_streaming_keeps_animation_bytes_and_recovers_after_a_failed_attachment() {
        use std::{
            io::{Read, Write},
            net::TcpListener,
        };
        let server = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = server.local_addr().unwrap();
        let writer = std::thread::spawn(move || {
            for body in [
                b"".as_slice(),
                b"GIF89a original animated payload".as_slice(),
            ] {
                let (mut socket, _) = server.accept().unwrap();
                let mut request = [0; 2048];
                let _ = socket.read(&mut request);
                write!(socket, "HTTP/1.1 200 OK\r\nContent-Type: image/gif\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len()).unwrap();
                socket.write_all(body).unwrap();
            }
        });
        let stage = tempfile::tempdir().unwrap();
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        runtime.block_on(async {
            // Deliberately bypass the production public-only proxy for this local HTTP fixture.
            let client = reqwest::Client::builder()
                .no_proxy()
                .timeout(Duration::from_secs(5))
                .build()
                .unwrap();
            let mut collector = Collector {
                client,
                work: stage.path(),
                files: Vec::new(),
                bytes: 0,
                started: Instant::now(),
            };
            let target = format!("http://{address}/animation.gif");
            let empty = collector.client.get(&target).send().await.unwrap();
            assert!(collector.save_response(empty).await.is_err());
            assert_eq!(fs::read_dir(stage.path()).unwrap().count(), 0);
            assert_eq!(collector.bytes, 0);
            let complete = collector.client.get(&target).send().await.unwrap();
            collector.save_response(complete).await.unwrap();
            assert_eq!(collector.files.len(), 1);
            assert_eq!(
                fs::read(&collector.files[0]).unwrap(),
                b"GIF89a original animated payload"
            );
            assert_eq!(
                collector.bytes,
                b"GIF89a original animated payload".len() as u64
            );
        });
        writer.join().unwrap();
    }
}
