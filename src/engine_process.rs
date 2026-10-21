use crate::{engines::Engine, jobs};
use std::{
    io::{BufRead, Read},
    path::Path,
    process::{Command, Stdio},
    time::{Duration, Instant},
};

fn output_size(path: &Path) -> Result<u64, String> {
    if !path.exists() {
        return Ok(0);
    }
    let metadata = std::fs::symlink_metadata(path).map_err(|e| e.to_string())?;
    crate::output::plain_path(path)?;
    if metadata.is_file() {
        return Ok(metadata.len());
    }
    let mut total = 0u64;
    for (index, entry) in std::fs::read_dir(path)
        .map_err(|e| e.to_string())?
        .enumerate()
    {
        if index >= 256 {
            return Err("Engine output has too many files.".into());
        }
        let entry = entry.map_err(|e| e.to_string())?;
        // yt-dlp renames and removes fragments while this directory is scanned.
        let meta = match std::fs::symlink_metadata(entry.path()) {
            Ok(meta) => meta,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
            Err(e) => return Err(e.to_string()),
        };
        if !meta.is_file() {
            return Err("Unexpected engine output entry.".into());
        }
        #[cfg(windows)]
        {
            use std::os::windows::fs::MetadataExt;
            if meta.file_attributes() & 0x400 != 0 {
                return Err("Reparse points are not supported in engine output.".into());
            }
        }
        total = total
            .checked_add(meta.len())
            .ok_or("Output size overflow")?;
    }
    Ok(total)
}
fn check_output(destination: Option<&Path>) -> Result<(), String> {
    if let Some(path) = destination {
        let size = output_size(path).map_err(|e| format!("Cannot inspect media output: {e}"))?;
        if size > 2 * 1024 * 1024 * 1024 {
            return Err("Media output exceeded 2 GiB.".into());
        }
    }
    Ok(())
}
/// Drain both pipes even after display limits; never let a verbose codec block.
pub fn run(
    engine: &Engine,
    args: &[String],
    timeout: Duration,
    destination: Option<&Path>,
    mut line: impl FnMut(&str),
) -> Result<String, String> {
    jobs::check()?;
    let _engine_guard = engine.validated_file()?;
    let mut command = Command::new(&engine.path);
    command
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    let mut child = command
        .spawn()
        .map_err(|e| format!("Cannot start engine: {e}"))?;
    let stdout = child.stdout.take().ok_or("Engine output unavailable")?;
    let stderr = child.stderr.take().ok_or("Engine errors unavailable")?;
    let (tx, rx) = std::sync::mpsc::sync_channel(64);
    let out = std::thread::spawn(move || {
        let mut reader = std::io::BufReader::new(stdout);
        let mut result = String::new();
        loop {
            let mut s = String::new();
            match reader.by_ref().take(8192).read_line(&mut s) {
                Ok(0) | Err(_) => break,
                _ => {}
            }
            result.push_str(&s);
            if result.len() > 128 * 1024 {
                let mut start = result.len() - 128 * 1024;
                while !result.is_char_boundary(start) {
                    start += 1;
                }
                result.drain(..start);
            }
            let _ = tx.try_send(s);
        }
        result
    });
    let err = std::thread::spawn(move || {
        let mut reader = stderr;
        let mut tail = Vec::new();
        let mut chunk = [0u8; 4096];
        loop {
            match reader.read(&mut chunk) {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    tail.extend_from_slice(&chunk[..n]);
                    if tail.len() > 16384 {
                        tail.drain(..tail.len() - 16384);
                    }
                }
            }
        }
        String::from_utf8_lossy(&tail).into_owned()
    });
    let started = Instant::now();
    let status = loop {
        for s in rx.try_iter() {
            line(&s);
        }
        if let Err(e) = jobs::check() {
            let _ = child.kill();
            let _ = child.wait();
            let _ = out.join();
            let _ = err.join();
            return Err(e);
        }
        let output_check = check_output(destination);
        if started.elapsed() > timeout || output_check.is_err() {
            let _ = child.kill();
            let _ = child.wait();
            let _ = out.join();
            let _ = err.join();
            return Err(output_check
                .err()
                .unwrap_or_else(|| "Media engine exceeded its time limit.".into()));
        }
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) => {}
            Err(e) => {
                let _ = child.kill();
                let _ = child.wait();
                let _ = out.join();
                let _ = err.join();
                return Err(e.to_string());
            }
        }
        std::thread::sleep(Duration::from_millis(50));
    };
    let stdout = out.join().map_err(|_| "Engine output reader stopped")?;
    let stderr = err.join().map_err(|_| "Engine error reader stopped")?;
    check_output(destination)?;
    for s in rx.try_iter() {
        line(&s);
    }
    if !status.success() {
        return Err(format!("Engine failed. {}", stderr.trim()));
    }
    jobs::check()?;
    Ok(stdout)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn output_checks_distinguish_inspection_errors_and_size_limits() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("media.mp4"), b"video").unwrap();
        assert_eq!(output_size(dir.path()).unwrap(), 5);
        check_output(Some(dir.path())).unwrap();
        std::fs::create_dir(dir.path().join("unexpected")).unwrap();
        assert!(check_output(Some(dir.path()))
            .unwrap_err()
            .starts_with("Cannot inspect media output:"));
        let large = dir.path().join("large.mp4");
        std::fs::File::create(&large)
            .unwrap()
            .set_len(2 * 1024 * 1024 * 1024 + 1)
            .unwrap();
        assert_eq!(
            check_output(Some(&large)).unwrap_err(),
            "Media output exceeded 2 GiB."
        );
    }

    #[test]
    fn output_scan_tolerates_fragment_renames() {
        let dir = tempfile::tempdir().unwrap();
        let first = dir.path().join("media.part");
        let second = dir.path().join("media.mp4");
        std::fs::write(&first, b"video").unwrap();
        std::thread::scope(|scope| {
            scope.spawn(|| {
                for _ in 0..1000 {
                    std::fs::rename(&first, &second).unwrap();
                    std::fs::rename(&second, &first).unwrap();
                }
            });
            for _ in 0..1000 {
                assert!(output_size(dir.path()).unwrap() <= 10);
            }
        });
    }
}
