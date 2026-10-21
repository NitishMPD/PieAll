use crate::{jobs, output};
use std::{
    collections::{BTreeMap, HashSet},
    fs::{self, File},
    io::{Read, Write},
    path::PathBuf,
};

struct Entry {
    path: PathBuf,
    name: String,
    directory: bool,
}
fn entries(paths: &[PathBuf]) -> Result<Vec<Entry>, String> {
    let mut result = Vec::new();
    let mut names = HashSet::new();
    let mut bytes = 0u64;
    let mut stack = paths
        .iter()
        .rev()
        .map(|p| {
            Ok((
                p.clone(),
                p.file_name()
                    .and_then(|s| s.to_str())
                    .ok_or("ZIP requires a named Unicode target")?
                    .to_string(),
                0usize,
            ))
        })
        .collect::<Result<Vec<_>, String>>()?;
    while let Some((path, name, depth)) = stack.pop() {
        jobs::check()?;
        if depth > 64 || result.len() >= 10000 {
            return Err(
                "ZIP creation supports at most 10,000 entries and 64 folder levels.".into(),
            );
        }
        output::plain_path(&path)?;
        output::safe_archive_name(&name)?;
        if !names.insert(name.to_lowercase()) {
            return Err(
                "Selected targets have duplicate archive paths. Choose them separately.".into(),
            );
        }
        let metadata = fs::symlink_metadata(&path).map_err(|e| e.to_string())?;
        if metadata.is_dir() {
            let mut children = fs::read_dir(&path)
                .map_err(|e| e.to_string())?
                .take(10001)
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| e.to_string())?;
            if children.len() > 10000 || stack.len() + result.len() + children.len() > 10000 {
                return Err("ZIP entry limit exceeded.".into());
            }
            children.sort_by_key(|e| e.file_name());
            for child in children.into_iter().rev() {
                let child_name = child
                    .file_name()
                    .into_string()
                    .map_err(|_| "ZIP requires Unicode filenames")?;
                stack.push((child.path(), format!("{name}/{child_name}"), depth + 1));
            }
        } else if metadata.is_file() {
            bytes = bytes
                .checked_add(metadata.len())
                .ok_or("ZIP size overflow")?;
            if bytes > 512 * 1024 * 1024 {
                return Err("ZIP creation limit is 512 MiB of source data.".into());
            }
        } else {
            return Err("ZIP supports ordinary files and folders only.".into());
        }
        result.push(Entry {
            path,
            name,
            directory: metadata.is_dir(),
        });
    }
    Ok(result)
}
pub fn create(paths: &[PathBuf], params: &BTreeMap<String, String>) -> Result<String, String> {
    let source = paths.first().ok_or("No targets selected")?;
    let method = match params.get("method").map_or("deflate", String::as_str) {
        "deflate" => zip::CompressionMethod::Deflated,
        "store" => zip::CompressionMethod::Stored,
        _ => return Err("Choose Deflate or Store.".into()),
    };
    let level = params.get("level").map_or(Ok(6), |s| {
        s.parse::<i64>()
            .map_err(|_| "Invalid ZIP compression level")
    })?;
    if !(0..=9).contains(&level) {
        return Err("ZIP compression level must be 0–9.".into());
    }
    // Collect before staging: a selected ancestor must never archive our own output.
    let entries = entries(paths)?;
    let parent = source.parent().ok_or("No parent folder")?;
    let stage = output::Staged::new(parent)?;
    let mut writer = zip::ZipWriter::new(stage.file.try_clone().map_err(|e| e.to_string())?);
    let mut options = zip::write::SimpleFileOptions::default().compression_method(method);
    if method == zip::CompressionMethod::Deflated {
        options = options.compression_level(Some(level));
    }
    let mut total = 0u64;
    for (index, entry) in entries.iter().enumerate() {
        jobs::check()?;
        jobs::progress(index, entries.len(), &entry.name);
        output::plain_path(&entry.path)?;
        if entry.directory {
            writer
                .add_directory(format!("{}/", entry.name), options)
                .map_err(|e| e.to_string())?;
            continue;
        }
        let mut file = File::open(&entry.path).map_err(|e| e.to_string())?;
        writer
            .start_file(&entry.name, options)
            .map_err(|e| e.to_string())?;
        let mut buffer = [0; 65536];
        loop {
            jobs::check()?;
            let n = file.read(&mut buffer).map_err(|e| e.to_string())?;
            if n == 0 {
                break;
            }
            total += n as u64;
            if total > 512 * 1024 * 1024 {
                return Err("ZIP sources grew beyond the 512 MiB limit.".into());
            }
            writer.write_all(&buffer[..n]).map_err(|e| e.to_string())?;
        }
    }
    writer.finish().map_err(|e| e.to_string())?;
    if stage.file.metadata().map_err(|e| e.to_string())?.len() > 512 * 1024 * 1024 {
        return Err("ZIP output exceeds 512 MiB; select fewer files.".into());
    }
    let base = if paths.len() == 1 {
        if source.is_dir() {
            source.file_name()
        } else {
            source.file_stem()
        }
    } else {
        parent.file_name()
    }
    .unwrap_or_default()
    .to_string_lossy();
    Ok(format!(
        "Saved {}",
        stage.publish_named(parent, &base, "zip")?.display()
    ))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn archives_nested_folders_empty_directories_and_compression_choices() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("album");
        fs::create_dir_all(source.join("empty")).unwrap();
        fs::create_dir(source.join("nested")).unwrap();
        fs::write(source.join("nested/a.txt"), b"original").unwrap();
        create(
            std::slice::from_ref(&source),
            &BTreeMap::from([("method".into(), "store".into())]),
        )
        .unwrap();
        let mut archive =
            zip::ZipArchive::new(File::open(dir.path().join("album.zip")).unwrap()).unwrap();
        assert!(archive.by_name("album/empty/").unwrap().is_dir());
        let mut file = archive.by_name("album/nested/a.txt").unwrap();
        assert_eq!(file.compression(), zip::CompressionMethod::Stored);
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes).unwrap();
        assert_eq!(bytes, b"original");
        drop(file);
        drop(archive);
        create(std::slice::from_ref(&source), &BTreeMap::new()).unwrap();
        assert!(dir.path().join("album (2).zip").exists());
        assert_eq!(fs::read(source.join("nested/a.txt")).unwrap(), b"original");
    }
    #[test]
    fn duplicate_root_names_and_invalid_levels_produce_no_archive() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("a");
        fs::write(&source, b"a").unwrap();
        assert!(create(&[source.clone(), source.clone()], &BTreeMap::new()).is_err());
        assert!(create(&[source], &BTreeMap::from([("level".into(), "10".into())])).is_err());
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
    }
}
