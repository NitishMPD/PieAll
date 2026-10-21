//! Typed FFmpeg jobs. All paths come from native captures and engine selection.
use crate::{engines::Engines, jobs, output};
use std::{
    collections::BTreeMap,
    fs::File,
    io::{Read, Write},
    path::Path,
    time::{Duration, Instant},
};

pub fn is_media(path: &Path, video_only: bool) -> bool {
    demuxer(path).is_some()
        && (!video_only
            || matches!(
                extension(path).as_str(),
                "mp4" | "m4v" | "mov" | "mkv" | "webm" | "avi" | "wmv"
            ))
}
fn extension(path: &Path) -> String {
    path.extension()
        .unwrap_or_default()
        .to_string_lossy()
        .to_ascii_lowercase()
}
fn demuxer(path: &Path) -> Option<&'static str> {
    Some(match extension(path).as_str() {
        "mp4" | "m4v" | "mov" | "m4a" => "mov",
        "mkv" | "webm" => "matroska",
        "avi" => "avi",
        "wmv" | "wma" => "asf",
        "mp3" => "mp3",
        "wav" => "wav",
        "flac" => "flac",
        "ogg" | "opus" => "ogg",
        "aac" => "aac",
        _ => return None,
    })
}
fn numeric(
    p: &BTreeMap<String, String>,
    key: &str,
    default: f64,
    min: f64,
    max: f64,
) -> Result<f64, String> {
    let n = p.get(key).map_or(Ok(default), |v| {
        v.parse::<f64>().map_err(|_| format!("Invalid {key}"))
    })?;
    if !n.is_finite() || n < min || n > max {
        return Err(format!("{key} must be between {min} and {max}."));
    }
    Ok(n)
}
struct Conversion {
    args: Vec<String>,
    ext: &'static str,
    label: &'static str,
    duration: Option<f64>,
}
fn conversion(id: &str, p: &BTreeMap<String, String>) -> Result<Conversion, String> {
    let mut c = Conversion {
        args: Vec::new(),
        ext: "mp4",
        label: "converted",
        duration: None,
    };
    let mut video = true;
    match id {
        "video_convert" => {
            c.ext = match p.get("format").map_or("mp4", String::as_str) {
                "mp4" => "mp4",
                "mkv" => "mkv",
                "mov" => "mov",
                "webm" => "webm",
                _ => return Err("Unsupported video format".into()),
            };
        }
        "video_compress" => {
            let crf = numeric(p, "quality", 28., 0., 51.)?;
            if crf.fract() != 0. {
                return Err("Quality must be a whole number.".into());
            }
            c.args.extend(["-crf".into(), format!("{crf:.0}")]);
            c.label = "compressed";
        }
        "video_resize" => {
            let width = numeric(p, "width", 1280., 2., 7680.)?;
            let height = numeric(p, "height", 720., 2., 4320.)?;
            if width.fract() != 0. || height.fract() != 0. {
                return Err("Dimensions must be whole pixels.".into());
            }
            c.args.extend(["-vf".into(),format!("scale=w='min(iw,{width:.0})':h='min(ih,{height:.0})':force_original_aspect_ratio=decrease:force_divisible_by=2")]);
            c.label = "resized";
        }
        "video_rotate" => {
            let filter = match p.get("angle").map_or("90", String::as_str) {
                "90" => "transpose=1",
                "180" => "hflip,vflip",
                "270" => "transpose=2",
                _ => return Err("Invalid video rotation".into()),
            };
            c.args.extend(["-vf".into(), filter.into()]);
            c.label = "rotated";
        }
        "video_mute" => {
            c.args.push("-an".into());
            c.label = "muted";
        }
        "video_trim" | "audio_trim" => {
            let start = numeric(p, "start", 0., 0., 86400.)?;
            let duration = numeric(p, "duration", 10., 0.01, 86400.)?;
            c.args.extend([
                "-ss".into(),
                format!("{start:.3}"),
                "-t".into(),
                format!("{duration:.3}"),
            ]);
            c.duration = Some(duration);
            c.label = "trimmed";
            if id == "audio_trim" {
                video = false;
                c.ext = "mp3";
            }
        }
        "video_gif" => {
            let width = numeric(p, "width", 480., 16., 1920.)?;
            let fps = numeric(p, "fps", 12., 1., 30.)?;
            c.args.extend(["-an".into(),"-filter_complex".into(),format!("[0:v:0]fps={fps:.3},scale={width:.0}:-1:flags=lanczos,split[a][b];[a]palettegen[p];[b][p]paletteuse")]);
            c.ext = "gif";
            c.label = "animated";
            return Ok(c);
        }
        "video_frame" => {
            let start = numeric(p, "start", 0., 0., 86400.)?;
            c.args.extend([
                "-ss".into(),
                format!("{start:.3}"),
                "-map".into(),
                "0:v:0".into(),
                "-frames:v".into(),
                "1".into(),
                "-update".into(),
                "1".into(),
            ]);
            c.ext = "png";
            c.label = "frame";
            return Ok(c);
        }
        "audio_convert" | "video_audio" | "audio_compress" | "audio_normalize" | "audio_volume" => {
            video = false;
            c.ext = match p.get("format").map_or("mp3", String::as_str) {
                "mp3" => "mp3",
                "wav" => "wav",
                "flac" => "flac",
                "m4a" => "m4a",
                "ogg" => "ogg",
                "opus" => "opus",
                _ => return Err("Unsupported audio format".into()),
            };
            if id == "audio_normalize" {
                c.args
                    .extend(["-af".into(), "loudnorm=I=-16:TP=-1.5:LRA=11".into()]);
                c.label = "normalized";
            }
            if id == "audio_volume" {
                let gain = numeric(p, "gain", 1.5, 0., 10.)?;
                c.args.extend(["-af".into(), format!("volume={gain:.4}")]);
                c.label = "volume";
            }
            if id == "audio_compress" {
                c.ext = "mp3";
                c.label = "compressed";
            }
        }
        _ => return Err("Unknown media action".into()),
    }
    if video {
        c.args.extend([
            "-map".into(),
            "0:v:0".into(),
            "-map".into(),
            "0:a:0?".into(),
        ]);
        if !c.args.iter().any(|a| a == "-vf") {
            c.args
                .extend(["-vf".into(), "scale=trunc(iw/2)*2:trunc(ih/2)*2".into()]);
        }
        c.args.extend([
            "-c:v".into(),
            if c.ext == "webm" {
                "libvpx-vp9"
            } else {
                "libx264"
            }
            .into(),
            "-c:a".into(),
            if c.ext == "webm" { "libopus" } else { "aac" }.into(),
            "-pix_fmt".into(),
            "yuv420p".into(),
        ]);
        if c.ext == "mp4" {
            c.args.extend(["-movflags".into(), "+faststart".into()]);
        }
    } else {
        c.args.extend([
            "-map".into(),
            "0:a:0".into(),
            "-vn".into(),
            "-c:a".into(),
            match c.ext {
                "wav" => "pcm_s16le",
                "flac" => "flac",
                "m4a" => "aac",
                "ogg" => "libvorbis",
                "opus" => "libopus",
                _ => "libmp3lame",
            }
            .into(),
        ]);
        if matches!(c.ext, "mp3" | "m4a" | "ogg" | "opus") {
            let bitrate = numeric(p, "bitrate", 192., 32., 320.)?;
            if bitrate.fract() != 0. {
                return Err("Bitrate must be a whole number.".into());
            }
            c.args.extend(["-b:a".into(), format!("{bitrate:.0}k")]);
        }
    }
    Ok(c)
}

fn input_args(path: &Path) -> Result<Vec<String>, String> {
    Ok(vec![
        "-protocol_whitelist".into(),
        "file,pipe".into(),
        "-f".into(),
        demuxer(path).ok_or("Unsupported media container")?.into(),
        "-i".into(),
        path.to_str().ok_or("Media path must be Unicode")?.into(),
    ])
}
pub fn run(
    id: &str,
    path: &Path,
    p: &BTreeMap<String, String>,
    engines: &Engines,
    index: usize,
    total: usize,
) -> Result<String, String> {
    output::plain_path(path)?;
    if path.metadata().map_err(|e| e.to_string())?.len() > 8 * 1024 * 1024 * 1024 {
        return Err("Media input exceeds 8 GiB.".into());
    }
    let probe = engines
        .get("ffprobe")
        .ok_or("Choose FFprobe in Tool engines first.")?;
    let mut probe_args = vec![
        "-v".into(),
        "error".into(),
        "-show_entries".into(),
        "format=duration:stream=index,codec_type,codec_name,width,height,sample_rate,channels"
            .into(),
        "-of".into(),
        "json".into(),
    ];
    probe_args.extend(input_args(path)?);
    let info =
        crate::engine_process::run(probe, &probe_args, Duration::from_secs(30), None, |_| {})?;
    if id == "media_details" {
        return Ok(format!("{}\r\n{info}", path.display()));
    }
    let metadata: serde_json::Value =
        serde_json::from_str(&info).map_err(|_| "Invalid media information")?;
    let mut duration = metadata["format"]["duration"]
        .as_str()
        .and_then(|s| s.parse::<f64>().ok())
        .filter(|s| s.is_finite() && *s > 0.);
    let conversion = conversion(id, p)?;
    if let Some(length) = conversion.duration {
        let start = numeric(p, "start", 0., 0., 86400.)?;
        if duration.is_some_and(|d| start >= d) {
            return Err("Trim starts beyond the end of the media.".into());
        }
        duration = duration.map(|d| (d - start).min(length));
    }
    let engine = engines
        .get("ffmpeg")
        .ok_or("Choose FFmpeg in Tool engines first.")?;
    let parent = path.parent().ok_or("Missing parent folder")?;
    let _parents = output::lock_parent(parent)?;
    let temp = tempfile::Builder::new()
        .prefix(".pieall-media-")
        .tempdir_in(parent)
        .map_err(|e| e.to_string())?;
    let destination = temp.path().join(format!("output.{}", conversion.ext));
    let mut args = vec![
        "-hide_banner".into(),
        "-loglevel".into(),
        "error".into(),
        "-nostdin".into(),
        "-n".into(),
        "-progress".into(),
        "pipe:1".into(),
        "-nostats".into(),
    ];
    args.extend(input_args(path)?);
    args.extend(conversion.args);
    args.push(
        destination
            .to_str()
            .ok_or("Output path must be Unicode")?
            .into(),
    );
    let mut last = Instant::now() - Duration::from_secs(1);
    crate::engine_process::run(
        engine,
        &args,
        Duration::from_secs(3600),
        Some(&destination),
        |line| {
            if last.elapsed() < Duration::from_millis(200) {
                return;
            }
            if let Some(us) = line
                .trim()
                .strip_prefix("out_time_us=")
                .and_then(|s| s.parse::<f64>().ok())
                .filter(|s| s.is_finite())
            {
                let seconds = (us / 1_000_000.).max(0.);
                jobs::progress_fraction(
                    index,
                    total,
                    format!(
                        "{} · {seconds:.1} s encoded",
                        path.file_name().unwrap_or_default().to_string_lossy()
                    ),
                    duration.map(|d| (seconds / d).clamp(0., 0.99)),
                );
                last = Instant::now();
            }
        },
    )?;
    output::plain_path(&destination)?;
    let size = destination.metadata().map_err(|e| e.to_string())?.len();
    if size == 0 || size > 2 * 1024 * 1024 * 1024 {
        return Err("Media output must be nonempty and under 2 GiB.".into());
    }
    if id.ends_with("compress") && size >= path.metadata().map_err(|e| e.to_string())?.len() {
        return Ok("Media is already compact; no larger output was saved.".into());
    }
    let mut stage = output::Staged::new(parent)?;
    let mut file = File::open(destination).map_err(|e| e.to_string())?;
    let mut buffer = [0; 65536];
    let mut copied = 0u64;
    loop {
        jobs::check()?;
        let n = file.read(&mut buffer).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        copied += n as u64;
        if copied > 2 * 1024 * 1024 * 1024 {
            return Err("Media output grew beyond 2 GiB.".into());
        }
        stage
            .file
            .write_all(&buffer[..n])
            .map_err(|e| e.to_string())?;
    }
    Ok(format!(
        "Saved {}",
        stage
            .publish(path, conversion.label, conversion.ext)?
            .display()
    ))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn typed_media_parameters_reject_injection_and_invalid_ranges() {
        for (key, value) in [("quality", "NaN"), ("quality", "-1"), ("quality", "2.5")] {
            assert!(conversion(
                "video_compress",
                &BTreeMap::from([(key.into(), value.into())])
            )
            .is_err());
        }
        assert!(conversion(
            "video_convert",
            &BTreeMap::from([("format".into(), "mp4 -y".into())])
        )
        .is_err());
        assert!(conversion(
            "audio_trim",
            &BTreeMap::from([("duration".into(), "inf".into())])
        )
        .is_err());
        let c = conversion("video_resize", &BTreeMap::new()).unwrap();
        assert!(c.args.iter().any(|a| a.contains("force_divisible_by=2")));
        assert_eq!(
            conversion("video_audio", &BTreeMap::new()).unwrap().ext,
            "mp3"
        );
        assert!(!is_media(Path::new("playlist.m3u8"), false));
        assert!(!is_media(Path::new("audio.mp3"), true));
        assert!(is_media(Path::new("VIDEO.MP4"), true));
    }
}
