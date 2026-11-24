//! Bounded post/page media collection. No browser cookies or page scripts are used.
use crate::{
    downloads::{self, Settings},
    engines::Engines,
    jobs, output, web_proxy,
};
use base64::Engine;
use scraper::{Html, Selector};
use std::{
    collections::{HashMap, HashSet},
    fs,
    io::Write,
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
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
fn youtube_video(url: &Url) -> bool {
    (host_is(url, "youtube.com")
        && (url.path() == "/watch"
            && url
                .query_pairs()
                .any(|(key, value)| key == "v" && !value.is_empty())
            || ["/shorts/", "/live/", "/embed/"].iter().any(|prefix| {
                url.path()
                    .strip_prefix(prefix)
                    .is_some_and(|id| !id.is_empty() && !id.contains('/'))
            })))
        || (host_is(url, "youtu.be")
            && url
                .path()
                .strip_prefix('/')
                .is_some_and(|id| !id.is_empty() && !id.contains('/')))
}

pub fn single_video(input: &str) -> bool {
    web_proxy::web_url(input).is_ok_and(|url| youtube_video(&url))
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
        || host_is(url, "pinterest.com")
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
    names: HashMap<Url, String>,
    previews: HashMap<Url, Url>,
}
fn media_url(base: &Url, input: &str) -> Option<Url> {
    let mut url = base.join(input.trim()).ok()?;
    url.set_fragment(None);
    web_proxy::web_url(url.as_str()).ok()
}

fn add_url(list: &mut Vec<Url>, base: &Url, text: &str) {
    if list.len() > MAX_FILES {
        return;
    }
    if let Some(url) = media_url(base, text) {
        if !list.contains(&url) {
            list.push(url);
        }
    }
}
fn selector(value: &str) -> Selector {
    Selector::parse(value).expect("static media selector")
}
fn page_media(html: &str, base: &Url) -> PageMedia {
    let doc = Html::parse_document(html);
    if host_is(base, "playtester.io") {
        return playtester_media(&doc, base);
    }
    if pinterest(base) {
        if let Some(media) = pinterest_media(&doc, base) {
            return media;
        }
    }
    let mut media = PageMedia::default();
    let linkedin = host_is(base, "linkedin.com");
    let reddit = host_is(base, "reddit.com");
    if youtube_video(base) {
        media.videos.push(base.clone());
        if let Some(name) = meta_content(&doc, "meta[property='og:title']") {
            media
                .names
                .insert(base.clone(), name.chars().take(200).collect());
        }
        if let Some(poster) =
            meta_content(&doc, "meta[property='og:image']").and_then(|value| media_url(base, value))
        {
            media.previews.insert(base.clone(), poster);
        }
        return media;
    }
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
    let scope = if reddit {
        doc.select(&selector("shreddit-post"))
            .next()
            .unwrap_or(scope)
    } else {
        scope
    };
    if reddit {
        for player in scope.select(&selector("shreddit-player, shreddit-player-2")) {
            let element = player.value();
            let poster = element.attr("poster").and_then(|url| media_url(base, url));
            let stream = poster.as_ref().and_then(reddit_poster_stream);
            let source = reddit_player_source(element, base);
            let source = source
                .filter(|url| extension(url) != "mp4" || stream.is_none())
                .or(stream);
            if let Some(source) = source {
                add_reddit_video(&mut media, &doc, base, &source, poster);
            }
        }
    }
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
                            || linkedin
                                && c.split_ascii_whitespace().any(|class| {
                                    class == "comment"
                                        || class.starts_with("comment__")
                                        || class == "comments"
                                })
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
            if let Some(poster) = reddit.then(|| media_url(base, url)).flatten() {
                if let Some(stream) = reddit_poster_stream(&poster) {
                    let source = img
                        .ancestors()
                        .filter_map(scraper::ElementRef::wrap)
                        .find(|parent| {
                            matches!(
                                parent.value().name(),
                                "shreddit-player" | "shreddit-player-2"
                            )
                        })
                        .and_then(|player| reddit_player_source(player.value(), base))
                        .unwrap_or(stream);
                    add_reddit_video(&mut media, &doc, base, &source, Some(poster));
                    continue;
                }
            }
            let before = media.images.len();
            add_url(&mut media.images, base, url);
            if let Some((name, url)) = e
                .attr("alt")
                .filter(|name| !name.trim().is_empty())
                .zip(media.images.get(before))
            {
                media
                    .names
                    .insert(url.clone(), name.chars().take(200).collect());
            }
        }
    }
    let video_sources = selector("source[src]");
    for video in scope.select(&selector("video")) {
        let poster = video
            .value()
            .attr("data-poster-url")
            .or_else(|| video.value().attr("poster"))
            .and_then(|poster| media_url(base, poster));
        let from = media.videos.len();
        if linkedin {
            if let Some(url) = video
                .value()
                .attr("data-sources")
                .and_then(|sources| linkedin_video_source(sources, base))
            {
                add_url(&mut media.videos, base, url.as_str());
                if let (Some(poster), Some(url)) = (poster, media.videos.get(from)) {
                    media.previews.insert(url.clone(), poster);
                }
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
        if let Some(poster) = poster {
            for url in &media.videos[from..] {
                media.previews.insert(url.clone(), poster.clone());
            }
        }
    }
    let native_linkedin_video = linkedin && !media.videos.is_empty();
    // Structured post attachments can include carousel images absent from the HTML.
    for script in doc.select(&selector("script[type='application/ld+json']")) {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&script.inner_html()) {
            json_media(&json, base, &mut media, native_linkedin_video, 0);
        }
    }
    if media.images.is_empty() && !((linkedin || reddit) && !media.videos.is_empty()) {
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

fn pinterest_media(doc: &Html, base: &Url) -> Option<PageMedia> {
    let mut media = PageMedia::default();
    for script in doc.select(&selector(
        "script[data-test-id='video-snippet'][type='application/ld+json']",
    )) {
        let Ok(json) = serde_json::from_str::<serde_json::Value>(&script.inner_html()) else {
            continue;
        };
        json_media(&json, base, &mut media, false, 0);
    }
    // Pinterest also publishes the video's poster as the pin image.
    // A native video pin has one playable attachment.
    media
        .videos
        .iter()
        .any(pinterest_direct_video)
        .then_some(media)
}

fn pinterest_direct_video(url: &Url) -> bool {
    host_is(url, "pinimg.com") && extension(url) == "mp4"
}

fn reddit_player_source(player: &scraper::node::Element, base: &Url) -> Option<Url> {
    player
        .attr("src")
        .and_then(|url| media_url(base, url))
        .filter(|url| matches!(extension(url).as_str(), "mpd" | "m3u8" | "mp4"))
}

fn reddit_poster_stream(poster: &Url) -> Option<Url> {
    if poster.host_str() != Some("preview.redd.it")
        || !poster
            .query_pairs()
            .any(|(key, value)| key == "vthumb" && value == "1")
    {
        return None;
    }
    let id = poster.path().strip_prefix('/')?.rsplit_once('.')?.0;
    if id.is_empty() || id.len() > 32 || !id.bytes().all(|byte| byte.is_ascii_alphanumeric()) {
        return None;
    }
    Url::parse(&format!("https://v.redd.it/{id}/DASHPlaylist.mpd")).ok()
}

fn add_reddit_video(
    media: &mut PageMedia,
    doc: &Html,
    base: &Url,
    stream: &Url,
    poster: Option<Url>,
) {
    // The player and its fallback poster can describe the same Reddit video.
    let asset = |url: &Url| {
        (url.host_str() == Some("v.redd.it"))
            .then(|| url.path_segments()?.next().map(str::to_owned))
            .flatten()
    };
    let id = asset(stream);
    let existing = media
        .videos
        .iter()
        .find(|url| *url == stream || (id.is_some() && asset(url) == id));
    let video = if let Some(existing) = existing {
        existing.clone()
    } else {
        let before = media.videos.len();
        add_url(&mut media.videos, base, stream.as_str());
        let Some(video) = media.videos.get(before) else {
            return;
        };
        video.clone()
    };
    if let Some(name) = meta_content(doc, "meta[property='og:title']") {
        media
            .names
            .insert(video.clone(), name.chars().take(200).collect());
    }
    if let Some(poster) = poster {
        media.previews.entry(video).or_insert(poster);
    }
}

fn meta_content<'a>(doc: &'a Html, selector_text: &str) -> Option<&'a str> {
    doc.select(&selector(selector_text))
        .next()?
        .value()
        .attr("content")
}

fn playtester_media(doc: &Html, base: &Url) -> PageMedia {
    let mut media = PageMedia::default();
    let image_source = selector("img[src]");
    for button in doc.select(&selector(
        "[aria-label^='Media carousel for '] button[aria-label$=' in fullscreen']",
    )) {
        let Some(url) = button
            .select(&image_source)
            .next()
            .and_then(|image| image.value().attr("src"))
            .and_then(|src| base.join(src).ok())
        else {
            continue;
        };
        let original = (url.path() == "/_next/image")
            .then(|| url.query_pairs().find(|(name, _)| name == "url"))
            .flatten()
            .and_then(|(_, src)| Url::parse(&src).ok())
            .filter(|url| url.scheme() == "https" && url.host_str() == Some("cdn.playtester.io"));
        let before = media.images.len();
        add_url(
            &mut media.images,
            base,
            original.as_ref().unwrap_or(&url).as_str(),
        );
        if let (Some(name), Some(url)) =
            (button.value().attr("aria-label"), media.images.get(before))
        {
            let name = name.strip_prefix("View ").unwrap_or(name);
            let name = name.strip_suffix(" in fullscreen").unwrap_or(name);
            media
                .names
                .insert(url.clone(), name.chars().take(200).collect());
        }
        if let (Some(image), Some(preview)) =
            (media.images.get(before), media_url(base, url.as_str()))
        {
            media.previews.insert(image.clone(), preview);
        }
    }
    for script in doc.select(&selector("script#game-jsonld[type='application/ld+json']")) {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&script.inner_html()) {
            for video in json["subjectOf"].as_array().into_iter().flatten() {
                if video["@type"] != "VideoObject" {
                    continue;
                }
                if let Some(source) = video["contentUrl"].as_str() {
                    let before = media.videos.len();
                    add_url(&mut media.videos, base, source);
                    if let (Some(name), Some(url)) =
                        (video["name"].as_str(), media.videos.get(before))
                    {
                        media
                            .names
                            .insert(url.clone(), name.chars().take(200).collect());
                    }
                    if let Some((poster, url)) = video["thumbnailUrl"]
                        .as_str()
                        .and_then(|value| media_url(base, value))
                        .zip(media.videos.get(before))
                    {
                        media.previews.insert(url.clone(), poster);
                    }
                }
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

fn json_media(
    value: &serde_json::Value,
    base: &Url,
    media: &mut PageMedia,
    native_linkedin_video: bool,
    depth: usize,
) {
    if depth > 24 {
        return;
    }
    match value {
        serde_json::Value::Array(values) => {
            for v in values {
                json_media(v, base, media, native_linkedin_video, depth + 1);
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
                    if kind == "ImageObject" {
                        add_url(&mut media.images, base, url);
                    } else if let Some(url) = media_url(base, url) {
                        if !native_linkedin_video {
                            add_url(&mut media.videos, base, url.as_str());
                        }
                        if let Some(video) = media
                            .videos
                            .iter()
                            .find(|source| **source == url)
                            .or_else(|| {
                                (native_linkedin_video && media.videos.len() == 1)
                                    .then(|| &media.videos[0])
                            })
                        {
                            if let Some(name) = obj
                                .get("headline")
                                .or_else(|| obj.get("name"))
                                .and_then(|v| v.as_str())
                            {
                                media
                                    .names
                                    .insert(video.clone(), name.chars().take(200).collect());
                            }
                            if let Some(poster) = obj
                                .get("thumbnailUrl")
                                .and_then(|v| v.as_str().or_else(|| v.get(0)?.as_str()))
                                .and_then(|value| media_url(base, value))
                            {
                                media.previews.entry(video.clone()).or_insert(poster);
                            }
                        }
                    }
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
                    json_media(v, base, media, native_linkedin_video, depth + 1);
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

async fn response(
    client: &reqwest::Client,
    url: &Url,
    referer: &Url,
) -> Result<reqwest::Response, String> {
    let response = wait(
        client
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

async fn resolve_post(client: &reqwest::Client, url: Url) -> Result<Url, String> {
    if !linkedin_short(&url) {
        return Ok(url);
    }
    let response = response(client, &url, &url).await?;
    let target = response
        .headers()
        .get(reqwest::header::LOCATION)
        .filter(|_| response.status().is_redirection())
        .and_then(|value| value.to_str().ok())
        .and_then(|location| response.url().join(location).ok());
    linkedin_post_target(target.as_ref().unwrap_or(response.url()))
}

async fn discover_page(mut response: reqwest::Response, source: &Url) -> Result<PageMedia, String> {
    let url = if host_is(source, "linkedin.com") {
        linkedin_post_target(response.url())?
    } else {
        post_url(response.url().as_str())?
    };
    let mut bytes = Vec::new();
    while let Some(chunk) = wait(response.chunk()).await?.map_err(|e| e.to_string())? {
        if bytes.len() + chunk.len() > MAX_PAGE {
            return Err("The page exceeds the 8 MiB discovery limit.".into());
        }
        bytes.extend_from_slice(&chunk);
    }
    let media = page_media(&String::from_utf8_lossy(&bytes), &url);
    if media.images.len() + media.videos.len() > MAX_FILES {
        return Err("This page exceeds the 100-attachment limit.".into());
    }
    Ok(media)
}

fn mime(response: &reqwest::Response) -> &str {
    response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .split(';')
        .next()
        .unwrap_or("")
        .trim()
}

#[derive(Clone, serde::Serialize)]
pub struct MediaItem {
    pub url: String,
    kind: &'static str,
    name: Option<String>,
    preview: Option<String>,
}

#[derive(serde::Deserialize, serde::Serialize)]
pub struct MediaSelection {
    pub url: String,
    pub name: Option<String>,
}

impl MediaSelection {
    pub fn validate(&self) -> Result<Url, String> {
        if let Some(name) = &self.name {
            let base = name
                .split('.')
                .next()
                .unwrap_or("")
                .trim_end()
                .to_ascii_uppercase();
            let reserved = ["COM", "LPT"].iter().any(|prefix| {
                base.strip_prefix(prefix)
                    .is_some_and(|suffix| matches!(suffix, "¹" | "²" | "³"))
            });
            if name.trim() != name
                || name.encode_utf16().count() > 120
                || name.contains('/')
                || name.chars().any(char::is_control)
                || base.is_empty()
                || reserved
                || matches!(base.as_str(), "CONIN$" | "CONOUT$")
                || output::safe_archive_name(&format!("{base}.file")).is_err()
                || output::safe_archive_name(name).is_err()
            {
                return Err("Choose a valid Windows filename of up to 120 characters.".into());
            }
        }
        web_proxy::web_url(&self.url)
    }
}

pub fn scan(input: &str) -> Result<Vec<MediaItem>, String> {
    scan_cancellable(input, &AtomicBool::new(false))
}

pub fn media_thumbnail(source: &str, referer: &str) -> Result<String, String> {
    const MAX_PREVIEW: usize = 5 * 1024 * 1024;
    let source = web_proxy::web_url(source)?;
    let referer = web_proxy::web_url(referer)?;
    let proxy = web_proxy::Proxy::start()?;
    let client = client(&proxy)?;
    runtime()?.block_on(async {
        tokio::time::timeout(Duration::from_secs(12), async {
            let mut response = response(&client, &source, &referer).await?;
            let format = match mime(&response) {
                "image/jpeg" => "jpeg",
                "image/png" => "png",
                "image/webp" => "webp",
                "image/gif" => "gif",
                "image/avif" => "avif",
                _ => return Err("Preview image format is unavailable.".into()),
            };
            if response
                .content_length()
                .is_some_and(|len| len > MAX_PREVIEW as u64)
            {
                return Err("Preview image exceeds the size limit.".into());
            }
            let mut bytes = Vec::new();
            while let Some(chunk) = response.chunk().await.map_err(|error| error.to_string())? {
                if bytes.len() + chunk.len() > MAX_PREVIEW {
                    return Err("Preview image exceeds the size limit.".into());
                }
                bytes.extend_from_slice(&chunk);
            }
            Ok(format!(
                "data:image/{format};base64,{}",
                base64::engine::general_purpose::STANDARD.encode(bytes)
            ))
        })
        .await
        .map_err(|_| "Preview image timed out.".to_string())?
    })
}

pub fn scan_cancellable(input: &str, cancelled: &AtomicBool) -> Result<Vec<MediaItem>, String> {
    let url = post_url(input)?;
    let proxy = web_proxy::Proxy::start()?;
    let client = client(&proxy)?;
    let scan = async {
        let url = resolve_post(&client, url).await?;
        let response = response(&client, &url, &url).await?;
        let direct_kind = match mime(&response) {
            "image/gif" => Some("gif"),
            value if value.starts_with("image/") => Some("image"),
            value if value.starts_with("video/") || stream_type(value) => Some("video"),
            _ => None,
        };
        if let Some(kind) = direct_kind {
            return Ok(vec![MediaItem {
                url: response.url().to_string(),
                kind,
                name: None,
                preview: None,
            }]);
        }
        let page = discover_page(response, &url).await?;
        let mut seen = HashSet::new();
        Ok(page
            .videos
            .into_iter()
            .map(|url| (url, "video"))
            .chain(page.images.into_iter().map(|url| {
                let kind = if extension(&url) == "gif" {
                    "gif"
                } else {
                    "image"
                };
                (url, kind)
            }))
            .filter(|(url, _)| seen.insert(url.clone()))
            .map(|(url, kind)| MediaItem {
                name: page.names.get(&url).cloned(),
                preview: page.previews.get(&url).map(Url::to_string),
                url: url.into(),
                kind,
            })
            .collect())
    };
    runtime()?.block_on(wait_for_scan(scan, cancelled, Duration::from_secs(6)))
}

async fn wait_for_scan<T>(
    scan: impl std::future::Future<Output = Result<T, String>>,
    cancelled: &AtomicBool,
    deadline: Duration,
) -> Result<T, String> {
    tokio::time::timeout(deadline, async {
        let mut scan = std::pin::pin!(scan);
        loop {
            if cancelled.load(Ordering::SeqCst) {
                return Err("Cancelled. Media scan stopped.".into());
            }
            if let Ok(result) =
                tokio::time::timeout(Duration::from_millis(100), scan.as_mut()).await
            {
                return result;
            }
        }
    })
    .await
    .map_err(|_| "Media scan timed out before the page finished loading.".to_string())?
}

fn stream_type(mime: &str) -> bool {
    matches!(
        mime,
        "application/vnd.apple.mpegurl" | "application/x-mpegurl" | "application/dash+xml"
    )
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
        response(&self.client, url, referer).await
    }
    async fn save_response(&mut self, mut response: reqwest::Response) -> Result<(), String> {
        self.check()?;
        let ext = match mime(&response) {
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
        let total = response.content_length().filter(|size| *size > 0);
        let mut last_progress = None;
        let result: Result<(), String> = async {
            while let Some(chunk) = wait(response.chunk()).await?.map_err(|e| e.to_string())? {
                self.check()?;
                length += chunk.len() as u64;
                if length > MAX_BYTES - self.bytes {
                    return Err("Media collection exceeds 2 GiB.".into());
                }
                file.write_all(&chunk).map_err(|e| e.to_string())?;
                let now = Instant::now();
                if last_progress.is_none_or(|previous| {
                    now.duration_since(previous) >= Duration::from_millis(250)
                }) {
                    jobs::progress_fraction(
                        0,
                        1,
                        format!(
                            "Downloading attachment {} · {:.1} MiB",
                            self.files.len() + 1,
                            length as f64 / 1_048_576.0
                        ),
                        total.map(|total| (length as f64 / total as f64).min(1.0)),
                    );
                    last_progress = Some(now);
                }
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
        jobs::progress(0, 1, format!("{} attachments downloaded", self.files.len()));
        Ok(())
    }
    fn import(&mut self, paths: Vec<PathBuf>, title: Option<&str>) -> Result<(), String> {
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
            let stem = if let Some(title) = title.filter(|_| self.files.is_empty()) {
                downloads::filename(title, "")
            } else {
                format!("attachment-{:03}", self.files.len() + 1)
            };
            let path = self.work.join(format!("{stem}.{ext}"));
            fs::hard_link(source, &path).map_err(|e| e.to_string())?;
            self.bytes += len;
            self.files.push(path);
        }
        Ok(())
    }
}

fn publish(
    files: &[PathBuf],
    destination: &Path,
    base: &str,
    names: &HashMap<PathBuf, &str>,
) -> Result<PathBuf, String> {
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
            let ordered = files
                .iter()
                .filter(|file| !names.contains_key(*file))
                .chain(files.iter().filter(|file| names.contains_key(*file)));
            for source in ordered {
                jobs::check()?;
                output::plain_path(source)?;
                let target = if let Some(name) = names.get(source) {
                    let ext = source
                        .extension()
                        .and_then(|ext| ext.to_str())
                        .ok_or("Missing attachment extension")?;
                    let stem = name
                        .rsplit_once('.')
                        .filter(|(stem, suffix)| {
                            !stem.is_empty() && suffix.eq_ignore_ascii_case(ext)
                        })
                        .map_or(*name, |(stem, _)| stem);
                    output::link_named(source, &folder, stem, ext)?
                } else {
                    let target = folder.join(source.file_name().ok_or("Missing attachment name")?);
                    jobs::publish(|| fs::hard_link(source, &target).map_err(|e| e.to_string()))?;
                    target
                };
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

fn runtime() -> Result<tokio::runtime::Runtime, String> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| e.to_string())
}

fn client(proxy: &web_proxy::Proxy) -> Result<reqwest::Client, String> {
    let redirect_limit = reqwest::redirect::Policy::limited(8);
    reqwest::Client::builder().tls_backend_native().no_proxy()
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
        .build().map_err(|e| e.to_string())
}

pub fn run(
    input: &str,
    settings: &Settings,
    engines: &Engines,
    selected: Option<&[MediaSelection]>,
) -> Result<String, String> {
    let url = post_url(input)?;
    let selected_urls = selected
        .map(|items| {
            if items.is_empty() || items.len() > MAX_FILES {
                return Err("Choose between 1 and 100 media items.".into());
            }
            items
                .iter()
                .map(|item| item.validate())
                .collect::<Result<Vec<_>, _>>()
        })
        .transpose()?;
    let requested_names: HashMap<_, _> = selected_urls
        .iter()
        .flatten()
        .zip(selected.into_iter().flatten())
        .filter_map(|(url, item)| item.name.as_deref().map(|name| (url.clone(), name)))
        .collect();
    let mut output_names = HashMap::new();
    settings.preferences.validate()?;
    let _guard = output::lock_parent(&settings.destination)?;
    let work = tempfile::Builder::new()
        .prefix(".pieall-media-")
        .tempdir_in(&settings.destination)
        .map_err(|e| e.to_string())?;
    let proxy = web_proxy::Proxy::start()?;
    let runtime = runtime()?;
    let client = client(&proxy)?;
    let url = runtime.block_on(resolve_post(&client, url))?;
    let mut collector = Collector {
        client,
        work: work.path(),
        files: Vec::new(),
        bytes: 0,
        started: Instant::now(),
    };
    let mut warnings = Vec::new();
    let mut urls = selected_urls.unwrap_or_default();
    jobs::progress(
        0,
        1,
        if selected.is_some() {
            "Preparing selected media…"
        } else {
            "Finding post attachments…"
        },
    );
    // Extractors find original image/GIF/video URLs; all file IO stays native.
    let single_video = youtube_video(&url);
    let mut page = PageMedia::default();
    let mut pinterest_video = false;
    if selected.is_none() && pinterest(&url) {
        let discovered = runtime.block_on(async {
            tokio::time::timeout(Duration::from_secs(6), async {
                let response = collector.response(&url, &url).await?;
                discover_page(response, &url).await
            })
            .await
        });
        jobs::check()?;
        if let Ok(Ok(mut discovered)) = discovered {
            pinterest_video = discovered.videos.iter().any(pinterest_direct_video);
            if pinterest_video {
                urls.append(&mut discovered.images);
                urls.append(&mut discovered.videos);
            }
        }
    }
    if selected.is_none()
        && !pinterest_video
        && !host_is(&url, "linkedin.com")
        && !host_is(&url, "playtester.io")
        && !single_video
        && !media_extension(&extension(&url))
    {
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
    let mut gallery_found = selected.is_none() && !pinterest_video && !urls.is_empty();
    let reddit = host_is(&url, "reddit.com");
    let mut direct_media = false;
    let mut explicit_videos = pinterest_video;
    if selected.is_none() && (!gallery_found || reddit) && !single_video && !pinterest_video {
        let page_result: Result<(), String> = runtime.block_on(async {
            let response = collector.response(&url, &url).await?;
            let content_type = mime(&response);
            if content_type.starts_with("image/")
                || content_type.starts_with("video/")
                || image_extension(&extension(response.url()))
            {
                if host_is(&url, "linkedin.com") {
                    linkedin_post_target(response.url())?;
                } else {
                    post_url(response.url().as_str())?;
                }
                collector.save_response(response).await?;
                direct_media = true;
            } else {
                page = discover_page(response, &url).await?;
            }
            Ok(())
        });
        if let Err(error) = page_result {
            jobs::check()?;
            warnings.push(error);
        }
        if !gallery_found || !page.videos.is_empty() {
            urls.clear();
            gallery_found = false;
            urls.append(&mut page.images);
            explicit_videos = !page.videos.is_empty();
            urls.append(&mut page.videos);
        }
    }
    let mut failures = Vec::new();
    let mut seen = HashSet::new();
    for media_url in urls {
        if !seen.insert(media_url.to_string()) {
            continue;
        }
        collector.check()?;
        if youtube_video(&media_url) {
            page.videos.push(media_url);
            continue;
        }
        if matches!(extension(&media_url).as_str(), "m3u8" | "mpd") {
            page.videos.push(media_url);
            continue;
        }
        jobs::progress(
            0,
            1,
            format!("Connecting for attachment {}…", collector.files.len() + 1),
        );
        if let Err(error) = runtime.block_on(async {
            let response = collector.response(&media_url, &url).await?;
            let mime = mime(&response);
            if stream_type(mime) || gallery_found && mime == "text/html" {
                page.videos.push(media_url.clone());
                return Ok(());
            }
            collector.save_response(response).await?;
            if let Some(name) = requested_names.get(&media_url) {
                output_names.insert(collector.files.last().unwrap().clone(), *name);
            }
            Ok(())
        }) {
            jobs::check()?;
            failures.push(error);
        }
    }
    // Generic extractors handle embedded players and separate audio/video streams.
    if selected.is_none()
        && !gallery_found
        && !direct_media
        && !explicit_videos
        && !image_extension(&extension(&url))
        && page.videos.is_empty()
    {
        page.videos.push(url.clone());
    }
    let mut title = None;
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
            Ok(files) => {
                if youtube_video(&video) {
                    title = files
                        .first()
                        .and_then(|file| file.title.clone())
                        .filter(|name| !name.trim().is_empty());
                }
                let prior = collector.files.len();
                collector.import(
                    files.into_iter().map(|file| file.path).collect(),
                    title.as_deref(),
                )?;
                if let Some(name) = requested_names.get(&video) {
                    for file in &collector.files[prior..] {
                        output_names.insert(file.clone(), *name);
                    }
                }
            }
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
    if selected.is_none() && !gallery_found && !pinterest_video && social(&url) {
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
    let base = title.map_or(base, |title| downloads::filename(&title, ""));
    jobs::progress(0, 1, "Saving finished files…");
    let folder = publish(
        &collector.files,
        &settings.destination,
        &base,
        &output_names,
    )?;
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
    fn pinterest_video_pin_uses_the_direct_video_and_its_poster() {
        let url = Url::parse("https://in.pinterest.com/pin/123/").unwrap();
        let page = page_media(
            r#"<script data-test-id="video-snippet" type="application/ld+json">
                  {"@type":"VideoObject","name":"Tractor video","contentUrl":"https://v1.pinimg.com/videos/demo.mp4","thumbnailUrl":"https://i.pinimg.com/poster.jpg"}
               </script>
               <script data-test-id="leaf-snippet" type="application/ld+json">
                 {"@type":"SocialMediaPosting","image":"https://i.pinimg.com/poster.jpg"}
               </script>
               <img alt="creator avatar" src="/avatar.jpg"><img alt="cover" src="/unrelated.jpg">"#,
            &url,
        );
        assert_eq!(page.videos.len(), 1);
        let video = &page.videos[0];
        assert!(pinterest_direct_video(video));
        assert_eq!(page.names[video], "Tractor video");
        assert_eq!(page.previews[video].path(), "/poster.jpg");
        assert!(page.images.is_empty());
    }
    #[test]
    fn reddit_scan_lists_the_video_and_keeps_screenshots() {
        let url = Url::parse("https://www.reddit.com/r/example/comments/abc/post/").unwrap();
        let page = page_media(
            r#"<meta property="og:title" content="Demo">
               <shreddit-post>
                 <img alt="media poster" src="https://preview.redd.it/videoid.jpg?s=signed&amp;vthumb=1">
                 <shreddit-player src="https://v.redd.it/videoid/HLSPlaylist.m3u8"></shreddit-player>
                 <img src="https://preview.redd.it/screenshot.png?width=1200">
               </shreddit-post>
               <shreddit-post><img src="/other.jpg"></shreddit-post>"#,
            &url,
        );
        assert_eq!(page.videos.len(), 1);
        let video = &page.videos[0];
        assert_eq!(video.as_str(), "https://v.redd.it/videoid/HLSPlaylist.m3u8");
        assert_eq!(page.names[video], "Demo");
        assert_eq!(page.previews[video].path(), "/videoid.jpg");
        assert_eq!(page.images.len(), 1);
        assert_eq!(page.images[0].path(), "/screenshot.png");
        let fallback = page_media(
            r#"<img alt="media poster" src="https://preview.redd.it/videoid.jpg?vthumb=1">"#,
            &url,
        );
        assert_eq!(fallback.videos[0].path(), "/videoid/DASHPlaylist.mpd");
        assert!(fallback.images.is_empty());
    }
    #[test]
    fn stalled_scan_returns_a_timeout() {
        let cancelled = AtomicBool::new(false);
        let result = runtime().unwrap().block_on(wait_for_scan::<()>(
            std::future::pending(),
            &cancelled,
            Duration::from_millis(10),
        ));
        assert_eq!(
            result.unwrap_err(),
            "Media scan timed out before the page finished loading."
        );
    }

    #[test]
    fn youtube_watch_scan_lists_the_video_and_uses_thumbnail_only_as_preview() {
        let url = Url::parse("https://www.youtube.com/watch?v=example&list=other").unwrap();
        assert!(single_video(url.as_str()));
        assert!(!single_video("https://www.youtube.com/playlist?list=other"));
        let media = page_media(
            r#"<meta property="og:title" content="Video title">
               <meta property="og:image" content="https://i.ytimg.com/vi/example/hqdefault.jpg">
               <img src="/unrelated.jpg">"#,
            &url,
        );
        assert_eq!(media.videos, std::slice::from_ref(&url));
        assert_eq!(media.names[&url], "Video title");
        assert_eq!(media.previews[&url].host_str(), Some("i.ytimg.com"));
        assert!(media.images.is_empty());
    }

    #[test]
    fn linkedin_keeps_the_post_player_and_excludes_comments_and_poster_attachment() {
        let url = Url::parse("https://www.linkedin.com/posts/example").unwrap();
        let page = page_media(
            r#"<meta property="og:image" content="/poster.jpg">
            <article class="main-feed-activity-card">
              <video data-poster-url="/poster.jpg" data-sources='[{"src":"/best.mp4","data-bitrate":2}]'></video>
              <div class="comment"><img src="/reply.jpg"></div>
            </article>
            <script type="application/ld+json">{"@type":"VideoObject","headline":"Post video","contentUrl":"/alternate.mp4"}</script>"#,
            &url,
        );
        let video = url.join("/best.mp4").unwrap();
        assert_eq!(page.videos, std::slice::from_ref(&video));
        assert!(page.images.is_empty());
        assert_eq!(page.names[&video], "Post video");
        assert_eq!(page.previews[&video].path(), "/poster.jpg");
    }

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
    fn playtester_collects_game_trailer_and_original_carousel_screenshots() {
        let page = page_media(
            r#"<main>
                <div aria-label="Media carousel for Example">
                    <button aria-label="View Battle in fullscreen">
                        <img src="/_next/image?url=https%3A%2F%2Fcdn.playtester.io%2Fscreenshots%2Fexample%2Fbattle.webp&amp;w=1080">
                    </button>
                    <button aria-label="View video: Official Trailer"><img src="/poster.webp"></button>
                </div>
                <img src="/similar-game.webp">
                <video><source src="/article-loop.webm"></video>
            </main>
            <script id="game-jsonld" type="application/ld+json">
                {"image":"/cover.webp","subjectOf":[{"@type":"VideoObject","name":"Official Trailer","contentUrl":"https://stream.example/manifest/video.m3u8"}]}
            </script>"#,
            &Url::parse("https://playtester.io/example").unwrap(),
        );
        assert_eq!(
            page.images.iter().map(Url::as_str).collect::<Vec<_>>(),
            ["https://cdn.playtester.io/screenshots/example/battle.webp"]
        );
        assert_eq!(
            page.videos.iter().map(Url::as_str).collect::<Vec<_>>(),
            ["https://stream.example/manifest/video.m3u8"]
        );
        assert_eq!(page.names[&page.images[0]], "Battle");
        assert_eq!(page.names[&page.videos[0]], "Official Trailer");
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
        let names = HashMap::new();
        let one = publish(std::slice::from_ref(&gif), dest.path(), "post", &names).unwrap();
        let two = publish(std::slice::from_ref(&gif), dest.path(), "post", &names).unwrap();
        assert_ne!(one, two);
        assert_eq!(
            fs::read(one.join("attachment-001.gif")).unwrap(),
            b"GIF89a animation bytes"
        );
        assert!(publish(
            &[gif, stage.path().join("missing")],
            dest.path(),
            "failed",
            &names
        )
        .is_err());
        assert!(!dest.path().join("failed").exists());
    }

    #[test]
    fn renamed_attachments_keep_their_extensions_and_resolve_conflicts() {
        let stage = tempfile::tempdir().unwrap();
        let dest = tempfile::tempdir().unwrap();
        let first = stage.path().join("attachment-001.gif");
        let second = stage.path().join("attachment-002.gif");
        let unchanged = stage.path().join("Preview.gif");
        fs::write(&first, b"first").unwrap();
        fs::write(&second, b"second").unwrap();
        fs::write(&unchanged, b"unchanged").unwrap();
        let names = HashMap::from([(first.clone(), "Preview.gif"), (second.clone(), "Preview")]);
        let folder = publish(&[first, second, unchanged], dest.path(), "post", &names).unwrap();
        assert_eq!(fs::read(folder.join("Preview.gif")).unwrap(), b"unchanged");
        assert_eq!(fs::read(folder.join("Preview (2).gif")).unwrap(), b"first");
        assert_eq!(fs::read(folder.join("Preview (3).gif")).unwrap(), b"second");
        for name in ["../escape", "a/b", "NUL.txt", "CON .txt", "COM¹", "name."] {
            assert!(
                MediaSelection {
                    url: "https://example.com/image.gif".into(),
                    name: Some(name.into()),
                }
                .validate()
                .is_err(),
                "{name}"
            );
        }
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
