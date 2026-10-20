use std::{
    fs::{self, File, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};
static NEXT: AtomicU64 = AtomicU64::new(1);
pub fn lock_parent(parent: &Path) -> Result<Vec<File>, String> {
    plain_path(parent)?;
    let absolute = fs::canonicalize(parent).map_err(|e| e.to_string())?;
    let mut guards = Vec::new();
    #[cfg(windows)]
    {
        use std::os::windows::fs::{MetadataExt, OpenOptionsExt};
        for path in absolute.ancestors() {
            // Deny directory replacement while publishing through this path.
            let file = OpenOptions::new()
                .access_mode(0x80)
                .share_mode(1)
                .custom_flags(0x02200000)
                .open(path)
                .map_err(|e| format!("Cannot protect destination folder: {e}"))?;
            if file
                .metadata()
                .map_err(|e| e.to_string())?
                .file_attributes()
                & 0x400
                != 0
            {
                return Err("Destination folder became a reparse point.".into());
            }
            guards.push(file);
        }
    }
    Ok(guards)
}
pub fn plain_path(path: &Path) -> Result<(), String> {
    let absolute = fs::canonicalize(path).map_err(|e| e.to_string())?;
    for p in path.ancestors().chain(absolute.ancestors()) {
        if p.as_os_str().is_empty() {
            continue;
        }
        let m = fs::symlink_metadata(p).map_err(|e| e.to_string())?;
        #[cfg(windows)]
        {
            use std::os::windows::fs::MetadataExt;
            if m.file_attributes() & 0x400 != 0 {
                return Err("Reparse points are not supported in this preview.".into());
            }
        }
        if m.file_type().is_symlink() {
            return Err("Symbolic links are not supported.".into());
        }
    }
    Ok(())
}
pub struct Staged {
    path: PathBuf,
    pub file: File,
    _parent_guards: Vec<File>,
}
impl Staged {
    pub fn new(parent: &Path) -> Result<Self, String> {
        let guards = lock_parent(parent)?;
        for _ in 0..100 {
            let path = parent.join(format!(
                ".pieall-{}-{}.tmp",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            match OpenOptions::new()
                .write(true)
                .read(true)
                .create_new(true)
                .open(&path)
            {
                Ok(file) => {
                    return Ok(Self {
                        path,
                        file,
                        _parent_guards: guards,
                    })
                }
                Err(e) if e.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(e) => return Err(e.to_string()),
            }
        }
        Err("Could not reserve temporary output.".into())
    }
    /// Publishes beside `source` as `<stem>-<label>.<ext>` (or `<stem>.<ext>` for an empty label).
    pub fn publish(self, source: &Path, label: &str, extension: &str) -> Result<PathBuf, String> {
        plain_path(source)?;
        let parent = source.parent().ok_or("Missing parent folder")?;
        let stem = source.file_stem().unwrap_or_default().to_string_lossy();
        self.publish_named(parent, &output_stem(&stem, label), extension)
    }
    /// Publishes as `<base>.<ext>`, adding Windows-style ` (2)` counters instead of replacing.
    pub fn publish_named(
        mut self,
        parent: &Path,
        base: &str,
        extension: &str,
    ) -> Result<PathBuf, String> {
        self.file.flush().map_err(|e| e.to_string())?;
        self.file.sync_all().map_err(|e| e.to_string())?;
        plain_path(parent)?;
        for n in 0..10000 {
            let suffix = if n == 0 {
                String::new()
            } else {
                format!(" ({})", n + 1)
            };
            let dest = parent.join(format!("{base}{suffix}.{extension}"));
            match fs::hard_link(&self.path, &dest) {
                Ok(()) => return Ok(dest),
                Err(e) if e.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(e) => {
                    return Err(format!(
                        "Cannot publish output without replacing a file: {e}"
                    ))
                }
            }
        }
        Err("Too many output name collisions.".into())
    }
}
fn is_dimensions(s: &str) -> bool {
    s.split_once('x').is_some_and(|(w, h)| {
        !w.is_empty() && !h.is_empty() && w.bytes().chain(h.bytes()).all(|b| b.is_ascii_digit())
    })
}
fn strip_counter(s: &str) -> &str {
    s.strip_suffix(')')
        .and_then(|rest| rest.rsplit_once(" ("))
        .filter(|(_, n)| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
        .map_or(s, |(base, _)| base)
}
fn strip_tag<'a>(s: &'a str, label: &str) -> Option<&'a str> {
    let (base, tail) = s.rsplit_once('-')?;
    let same = if is_dimensions(label) {
        is_dimensions(tail)
    } else {
        tail.eq_ignore_ascii_case(label)
    };
    (same && !base.is_empty()).then_some(base)
}
/// Names an output after its source without stacking the same tag:
/// `photo-compressed (2)` compressed again becomes `photo-compressed`, and
/// `photo-1920x1080` resized becomes `photo-<new size>`.
pub fn output_stem(stem: &str, label: &str) -> String {
    if label.is_empty() {
        return stem.to_string();
    }
    let base = strip_tag(strip_counter(stem), label)
        .or_else(|| strip_tag(stem, label))
        .unwrap_or(stem);
    format!("{base}-{label}")
}
impl Drop for Staged {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}
pub fn write(source: &Path, label: &str, ext: &str, bytes: &[u8]) -> Result<PathBuf, String> {
    let mut s = Staged::new(source.parent().ok_or("Missing parent")?)?;
    s.file.write_all(bytes).map_err(|e| e.to_string())?;
    s.publish(source, label, ext)
}
pub fn safe_archive_name(name: &str) -> Result<PathBuf, String> {
    if name.is_empty() || name.starts_with(['/', '\\']) || name.contains(['\\', ':', '\0']) {
        return Err("Unsafe archive path".into());
    }
    let mut result = PathBuf::new();
    for part in name.split('/').filter(|p| !p.is_empty()) {
        let base = part.split('.').next().unwrap_or("").to_ascii_uppercase();
        if part == "."
            || part == ".."
            || part.ends_with(['.', ' '])
            || part.chars().any(|c| c < ' ' || "<>\"|?*".contains(c))
            || matches!(base.as_str(), "CON" | "PRN" | "AUX" | "NUL")
            || (base.len() == 4
                && (base.starts_with("COM") || base.starts_with("LPT"))
                && base.as_bytes()[3].is_ascii_digit())
        {
            return Err("Unsafe archive path component".into());
        }
        result.push(part);
    }
    if result.as_os_str().is_empty() {
        return Err("Empty archive path".into());
    }
    Ok(result)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn refuses_archive_escape() {
        for n in [
            "../x", "/x", "C:/x", "a\\b", "a/../b", "NUL.txt", "a.", "a:b",
        ] {
            assert!(safe_archive_name(n).is_err(), "{n}");
        }
        assert_eq!(
            safe_archive_name("a/b.txt").unwrap(),
            PathBuf::from("a/b.txt")
        );
    }
    #[test]
    fn preserves_original_and_collisions() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("source.txt");
        fs::write(&p, b"original").unwrap();
        let a = write(&p, "copy", "txt", b"one").unwrap();
        let b = write(&p, "copy", "txt", b"two").unwrap();
        assert_ne!(a, b);
        assert_eq!(fs::read(p).unwrap(), b"original");
        assert_eq!(fs::read(&a).unwrap(), b"one");
        assert!(a.ends_with("source-copy.txt"));
        assert!(b.ends_with("source-copy (2).txt"));
    }
    #[test]
    fn smart_names_do_not_stack() {
        assert_eq!(output_stem("photo", "compressed"), "photo-compressed");
        assert_eq!(
            output_stem("photo-compressed", "compressed"),
            "photo-compressed"
        );
        assert_eq!(
            output_stem("photo-compressed (3)", "compressed"),
            "photo-compressed"
        );
        assert_eq!(output_stem("photo-1920x1080", "960x540"), "photo-960x540");
        assert_eq!(output_stem("photo (2)", "rotated"), "photo (2)-rotated");
        assert_eq!(output_stem("photo (2)", ""), "photo (2)");
        assert_eq!(
            output_stem("-compressed", "compressed"),
            "-compressed-compressed"
        );
        assert_eq!(
            output_stem("photo-compressed", "rotated"),
            "photo-compressed-rotated"
        );
    }
}
#[cfg(test)]
mod concurrency_tests {
    use super::*;
    #[test]
    fn concurrent_publish_never_replaces() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("source.txt");
        fs::write(&p, b"original").unwrap();
        let jobs: Vec<_> = (0..8)
            .map(|i| {
                let p = p.clone();
                std::thread::spawn(move || {
                    write(&p, "copy", "txt", i.to_string().as_bytes()).unwrap()
                })
            })
            .collect();
        let mut paths = std::collections::HashSet::new();
        for job in jobs {
            assert!(paths.insert(job.join().unwrap()));
        }
        assert_eq!(fs::read(&p).unwrap(), b"original");
        assert_eq!(fs::read_dir(d.path()).unwrap().count(), 9);
    }
}
