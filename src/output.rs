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
    pub fn publish(
        mut self,
        source: &Path,
        label: &str,
        extension: &str,
    ) -> Result<PathBuf, String> {
        self.file.flush().map_err(|e| e.to_string())?;
        self.file.sync_all().map_err(|e| e.to_string())?;
        plain_path(source)?;
        let parent = source.parent().ok_or("Missing parent folder")?;
        plain_path(parent)?;
        let stem = source.file_stem().unwrap_or_default().to_string_lossy();
        for n in 0..10000 {
            let suffix = if n == 0 {
                String::new()
            } else {
                format!(" ({n})")
            };
            let dest = parent.join(format!("{stem} - {label}{suffix}.{extension}"));
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
        assert_eq!(fs::read(a).unwrap(), b"one");
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
