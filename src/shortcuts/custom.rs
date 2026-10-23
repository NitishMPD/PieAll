//! User keymaps are native-owned configuration, never executable commands.
use super::{Profile, Shortcut};
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, path::Path, sync::RwLock};

#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct App {
    pub executable: String,
    pub name: String,
    pub shortcuts: Vec<Binding>,
}
#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Binding {
    pub id: String,
    pub title: String,
    pub keys: String,
}
static APPS: RwLock<Vec<App>> = RwLock::new(Vec::new());

impl App {
    pub fn id(&self) -> String {
        format!(
            "custom-{}",
            self.executable
                .bytes()
                .map(|b| format!("{b:02x}"))
                .collect::<String>()
        )
    }
    fn validate(&self) -> Result<(), String> {
        if self.executable.len() > 200
            || !self.executable.ends_with(".exe")
            || self.executable.len() <= 4
            || self
                .executable
                .chars()
                .any(|c| c.is_control() || "\\/:*?\"<>|".contains(c))
            || self.executable.trim() != self.executable
            || self.executable != self.executable.to_ascii_lowercase()
        {
            return Err("Choose a Windows .exe file.".into());
        }
        if super::catalog()
            .by_executable
            .contains_key(&self.executable)
        {
            return Err(
                "This app already has a bundled profile. Customize its ring instead.".into(),
            );
        }
        if self.name.trim().is_empty()
            || self.name.chars().count() > 80
            || self.name.chars().any(char::is_control)
        {
            return Err("Use an app name of 1–80 characters.".into());
        }
        if self.shortcuts.is_empty() || self.shortcuts.len() > 32 {
            return Err("Add between 1 and 32 shortcuts.".into());
        }
        let mut ids = HashSet::new();
        for binding in &self.shortcuts {
            if binding.id.is_empty()
                || binding.id.len() > 64
                || !binding
                    .id
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'-')
                || !ids.insert(&binding.id)
            {
                return Err("Shortcut identities must be unique.".into());
            }
            if binding.title.trim().is_empty()
                || binding.title.chars().count() > 80
                || binding.title.chars().any(char::is_control)
            {
                return Err("Use shortcut names of 1–80 characters.".into());
            }
            if binding.keys.len() > 120 {
                return Err("Shortcut keys are too long.".into());
            }
            super::parse_keys(&binding.keys).map_err(|e| format!("{}: {e}", binding.title))?;
        }
        Ok(())
    }
    fn profile(&self) -> Profile {
        Profile {
            id: self.id(),
            name: self.name.clone(),
            executables: vec![self.executable.clone()],
            sets: vec![],
            source: "Custom app".into(),
            capture_text: false,
            quick: self
                .shortcuts
                .iter()
                .take(6)
                .map(|b| b.id.clone())
                .collect(),
        }
    }
    fn actions(&self) -> Vec<Shortcut> {
        self.shortcuts
            .iter()
            .enumerate()
            .map(|(i, b)| Shortcut {
                id: format!("app:{}:{}", self.id(), b.id),
                profile: self.id(),
                title: b.title.clone(),
                keys: b.keys.clone(),
                category: "Custom".into(),
                priority: (i < 6).then_some(i),
            })
            .collect()
    }
}
pub fn apps() -> Vec<App> {
    APPS.read().unwrap().clone()
}
pub fn profiles() -> Vec<Profile> {
    APPS.read().unwrap().iter().map(App::profile).collect()
}
pub fn actions() -> Vec<Shortcut> {
    APPS.read().unwrap().iter().flat_map(App::actions).collect()
}

fn read(path: &Path) -> Result<Vec<App>, String> {
    let db = crate::engines::database(path)?;
    let mut query = db
        .prepare("SELECT value FROM custom_apps ORDER BY executable")
        .map_err(|e| e.to_string())?;
    let rows = query
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(|e| e.to_string())?;
    let mut result = Vec::new();
    for row in rows {
        let raw = row.map_err(|e| e.to_string())?;
        if raw.len() > 32 * 1024 || result.len() >= 64 {
            return Err("Custom app settings exceed their limit.".into());
        }
        let app: App = serde_json::from_str(&raw).map_err(|e| e.to_string())?;
        app.validate()?;
        if result.iter().any(|a: &App| a.executable == app.executable) {
            return Err("Duplicate custom app.".into());
        }
        result.push(app);
    }
    Ok(result)
}
pub fn load(path: &Path) -> Result<(), String> {
    let mut apps = APPS.write().unwrap();
    *apps = read(path)?;
    Ok(())
}
pub fn save(path: &Path, app: App, replacing: Option<&str>) -> Result<String, String> {
    app.validate()?;
    let mut apps = APPS.write().unwrap();
    let mut next = apps.clone();
    if let Some(original) = replacing {
        let index = next
            .iter()
            .position(|a| a.executable == original)
            .ok_or("This custom app no longer exists.")?;
        if next
            .iter()
            .enumerate()
            .any(|(i, a)| i != index && a.executable == app.executable)
        {
            return Err("This executable already has a custom profile.".into());
        }
        next[index] = app.clone();
    } else {
        if next.len() >= 64 {
            return Err("At most 64 custom apps are supported.".into());
        }
        if next.iter().any(|a| a.executable == app.executable) {
            return Err("This executable already has a custom profile. Edit it instead.".into());
        }
        next.push(app.clone());
    }
    let mut db = crate::engines::database(path)?;
    let tx = db.transaction().map_err(|e| e.to_string())?;
    if let Some(original) = replacing {
        tx.execute("DELETE FROM custom_apps WHERE executable=?1", [original])
            .map_err(|e| e.to_string())?;
    }
    tx.execute(
        "INSERT INTO custom_apps(executable,value) VALUES(?1,?2)",
        rusqlite::params![
            app.executable,
            serde_json::to_string(&app).map_err(|e| e.to_string())?
        ],
    )
    .map_err(|e| e.to_string())?;
    tx.commit().map_err(|e| e.to_string())?;
    *apps = next;
    Ok(app.id())
}
pub fn remove(path: &Path, executable: &str) -> Result<(), String> {
    let mut apps = APPS.write().unwrap();
    let db = crate::engines::database(path)?;
    db.execute("DELETE FROM custom_apps WHERE executable=?1", [executable])
        .map_err(|e| e.to_string())?;
    apps.retain(|a| a.executable != executable);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn custom_apps_validate_persist_edit_and_remove() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.sqlite3");
        let app = App {
            executable: "test-custom.exe".into(),
            name: "My editor".into(),
            shortcuts: vec![Binding {
                id: "save".into(),
                title: "Save".into(),
                keys: "Ctrl+S".into(),
            }],
        };
        let id = save(&path, app.clone(), None).unwrap();
        assert_eq!(
            super::super::for_executable("TEST-CUSTOM.EXE").unwrap().id,
            id
        );
        assert_eq!(
            super::super::find(&format!("app:{id}:save")).unwrap().keys,
            "Ctrl+S"
        );
        let dispatched = super::super::find(&format!("app:{id}:save")).unwrap();
        assert!(save(&path, app.clone(), None).is_err());
        let mut edited = app.clone();
        edited.shortcuts[0].keys = "Ctrl+Shift+S".into();
        assert_eq!(save(&path, edited, Some(&app.executable)).unwrap(), id);
        load(&path).unwrap();
        assert_eq!(dispatched.keys, "Ctrl+S");
        assert_eq!(
            super::super::for_profile(&id).next().unwrap().keys,
            "Ctrl+Shift+S"
        );
        assert_eq!(read(&path).unwrap()[0].shortcuts[0].keys, "Ctrl+Shift+S");
        let blocked = dir.path().join("not-a-database");
        std::fs::create_dir(&blocked).unwrap();
        assert!(save(&blocked, app.clone(), Some(&app.executable)).is_err());
        assert_eq!(
            super::super::find(&format!("app:{id}:save")).unwrap().keys,
            "Ctrl+Shift+S"
        );
        for executable in ["chrome.exe", "../bad.exe", "bad.exe/evil", "Bad.exe"] {
            let mut bad = app.clone();
            bad.executable = executable.into();
            assert!(bad.validate().is_err());
        }
        for keys in ["", "Win+R", "Ctrl+S extra", "Ctrl+Ctrl+S"] {
            let mut bad = app.clone();
            bad.shortcuts[0].keys = keys.into();
            assert!(save(&path, bad, Some(&app.executable)).is_err());
        }
        let mut bad = app.clone();
        bad.shortcuts.push(bad.shortcuts[0].clone());
        assert!(bad.validate().is_err());
        bad.shortcuts.clear();
        assert!(bad.validate().is_err());
        let mut bad = app.clone();
        bad.shortcuts[0].id = "app:chrome:copy-url".into();
        assert!(bad.validate().is_err());
        let mut renamed = app.clone();
        renamed.executable = "renamed-custom.exe".into();
        let renamed_id = save(&path, renamed.clone(), Some(&app.executable)).unwrap();
        assert_ne!(renamed_id, id);
        assert!(super::super::for_executable(&app.executable).is_none());
        assert!(super::super::for_executable(&renamed.executable).is_some());
        save(
            &path,
            App {
                shortcuts: vec![Binding {
                    keys: "Ctrl+Shift+S".into(),
                    ..app.shortcuts[0].clone()
                }],
                ..app.clone()
            },
            Some(&renamed.executable),
        )
        .unwrap();
        assert_eq!(read(&path).unwrap()[0].shortcuts[0].keys, "Ctrl+Shift+S");
        remove(&path, &app.executable).unwrap();
        assert!(read(&path).unwrap().is_empty());
        assert!(super::super::for_executable(&app.executable).is_none());
    }
}
