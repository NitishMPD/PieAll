//! Native-owned optional engine records. Never search PATH or the working directory.
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs::File,
    io::Read,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Engine {
    pub path: PathBuf,
    pub sha256: String,
}
pub type Engines = BTreeMap<String, Engine>;
pub const NAMES: &[&str] = &["ffmpeg", "ffprobe"];
fn fingerprint(path: &Path) -> Result<String, String> {
    crate::output::plain_path(path)?;
    let mut file = File::open(path).map_err(|e| e.to_string())?;
    let len = file.metadata().map_err(|e| e.to_string())?.len();
    if len == 0 || len > 256 * 1024 * 1024 {
        return Err("Engine must be a nonempty executable under 256 MiB.".into());
    }
    let mut hash = Sha256::new();
    let mut buffer = [0; 65536];
    let mut total = 0u64;
    loop {
        crate::jobs::check()?;
        let n = file.read(&mut buffer).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        total += n as u64;
        if total > 256 * 1024 * 1024 {
            return Err("Engine grew beyond its size limit.".into());
        }
        hash.update(&buffer[..n]);
    }
    Ok(format!("{:x}", hash.finalize()))
}
impl Engine {
    pub fn select(name: &str, path: &Path) -> Result<Self, String> {
        if !NAMES.contains(&name) {
            return Err("Unknown engine.".into());
        }
        if !path
            .file_name()
            .is_some_and(|n| n.eq_ignore_ascii_case(format!("{name}.exe")))
        {
            return Err(format!("Choose {name}.exe from a trusted installation."));
        }
        crate::output::plain_path(path)?;
        let path = std::fs::canonicalize(path).map_err(|e| e.to_string())?;
        let sha256 = fingerprint(&path)?;
        let mut file = File::open(&path).map_err(|e| e.to_string())?;
        let mut magic = [0; 2];
        file.read_exact(&mut magic).map_err(|e| e.to_string())?;
        if magic != *b"MZ" {
            return Err("Choose a Windows executable.".into());
        }
        Ok(Self { path, sha256 })
    }
    pub fn validated_file(&self) -> Result<File, String> {
        // Deny write/delete while hashing and executing; prevents a replaced engine
        // from racing the pinned digest check on Windows.
        let mut options = std::fs::OpenOptions::new();
        options.read(true);
        #[cfg(windows)]
        {
            use std::os::windows::fs::OpenOptionsExt;
            options.share_mode(1);
        }
        let file = options
            .open(&self.path)
            .map_err(|e| format!("Engine unavailable: {e}"))?;
        if fingerprint(&self.path)? != self.sha256 {
            return Err("Engine changed since selection. Choose it again in Tool engines to trust the update.".into());
        }
        Ok(file)
    }
}
fn database(path: &Path) -> Result<Connection, String> {
    let parent = path.parent().ok_or("Missing settings folder")?;
    std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    crate::output::plain_path(parent)?;
    if path.exists() {
        crate::output::plain_path(path)?;
    }
    let mut db = Connection::open(path).map_err(|e| e.to_string())?;
    let version: u32 = db
        .pragma_query_value(None, "user_version", |r| r.get(0))
        .map_err(|e| e.to_string())?;
    if version > 1 {
        return Err("Settings were created by a newer PieAll version.".into());
    }
    if version == 0 {
        let tx = db.transaction().map_err(|e| e.to_string())?;
        tx.execute_batch("CREATE TABLE IF NOT EXISTS engines (name TEXT PRIMARY KEY, path TEXT NOT NULL, sha256 TEXT NOT NULL); PRAGMA user_version=1;").map_err(|e| e.to_string())?;
        tx.commit().map_err(|e| e.to_string())?;
    }
    Ok(db)
}
pub fn load(path: &Path) -> Result<Engines, String> {
    let db = database(path)?;
    let mut statement = db
        .prepare("SELECT name,path,sha256 FROM engines")
        .map_err(|e| e.to_string())?;
    let records = statement
        .query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                Engine {
                    path: PathBuf::from(r.get::<_, String>(1)?),
                    sha256: r.get(2)?,
                },
            ))
        })
        .map_err(|e| e.to_string())?;
    let mut engines = Engines::new();
    for record in records {
        let (name, engine) = record.map_err(|e| e.to_string())?;
        if NAMES.contains(&name.as_str()) {
            engines.insert(name, engine);
        }
    }
    Ok(engines)
}
pub fn store(path: &Path, name: &str, engine: Option<&Engine>) -> Result<(), String> {
    if !NAMES.contains(&name) {
        return Err("Unknown engine.".into());
    }
    let db = database(path)?;
    if let Some(engine) = engine {
        db.execute("INSERT INTO engines(name,path,sha256) VALUES(?1,?2,?3) ON CONFLICT(name) DO UPDATE SET path=excluded.path,sha256=excluded.sha256", params![name,engine.path.to_str().ok_or("Engine path must be Unicode")?,engine.sha256]).map_err(|e| e.to_string())?;
    } else {
        db.execute("DELETE FROM engines WHERE name=?1", [name])
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn settings_roundtrip_and_changed_engine_refusal() {
        let dir = tempfile::tempdir().unwrap();
        let exe = dir.path().join("ffmpeg.exe");
        std::fs::write(&exe, b"MZtest fixture").unwrap();
        let engine = Engine::select("ffmpeg", &exe).unwrap();
        let db = dir.path().join("settings.sqlite3");
        store(&db, "ffmpeg", Some(&engine)).unwrap();
        let records = load(&db).unwrap();
        assert_eq!(records["ffmpeg"].sha256, engine.sha256);
        drop(records["ffmpeg"].validated_file().unwrap());
        std::fs::write(&exe, b"MZchanged fixture").unwrap();
        assert!(records["ffmpeg"].validated_file().is_err());
        store(&db, "ffmpeg", None).unwrap();
        assert!(load(&db).unwrap().is_empty());
    }
}
