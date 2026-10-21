//! Bundled Windows default keymaps. Only native captured process identity selects a profile.
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashMap},
    sync::OnceLock,
};

#[derive(Deserialize)]
struct Source {
    sets: BTreeMap<String, Vec<Binding>>,
    profiles: Vec<Profile>,
}
#[derive(Clone, Deserialize)]
struct Binding {
    id: String,
    title: String,
    keys: String,
    category: String,
}
#[derive(Deserialize, Serialize)]
pub struct Profile {
    pub id: String,
    pub name: String,
    pub executables: Vec<String>,
    pub sets: Vec<String>,
    pub source: String,
    pub capture_text: bool,
    pub quick: Vec<String>,
}
#[derive(Serialize)]
pub struct Shortcut {
    pub id: String,
    pub profile: String,
    pub title: String,
    pub keys: String,
    pub category: String,
    pub priority: Option<usize>,
}
pub struct Catalog {
    pub profiles: Vec<Profile>,
    pub actions: Vec<Shortcut>,
    by_id: HashMap<String, usize>,
    by_executable: HashMap<String, usize>,
    by_profile: HashMap<String, std::ops::Range<usize>>,
}
pub fn catalog() -> &'static Catalog {
    static CATALOG: OnceLock<Catalog> = OnceLock::new();
    CATALOG.get_or_init(|| {
        let source: Source = serde_json::from_str(include_str!("shortcuts.json"))
            .expect("bundled shortcut catalog must be valid");
        let mut actions = Vec::new();
        let mut by_id = HashMap::new();
        let mut by_executable = HashMap::new();
        let mut by_profile = HashMap::new();
        for (index, profile) in source.profiles.iter().enumerate() {
            let start = actions.len();
            for executable in &profile.executables {
                assert!(by_executable
                    .insert(executable.to_ascii_lowercase(), index)
                    .is_none());
            }
            for set in &profile.sets {
                for binding in source.sets.get(set).expect("known shortcut set") {
                    let id = format!("app:{}:{}", profile.id, binding.id);
                    assert!(
                        parse_keys(&binding.keys).is_ok(),
                        "invalid chord: {}",
                        binding.keys
                    );
                    assert!(
                        by_id.insert(id.clone(), actions.len()).is_none(),
                        "duplicate shortcut: {id}"
                    );
                    actions.push(Shortcut {
                        id,
                        profile: profile.id.clone(),
                        title: binding.title.clone(),
                        keys: binding.keys.clone(),
                        category: binding.category.clone(),
                        priority: profile.quick.iter().position(|id| id == &binding.id),
                    });
                }
            }
            assert!(by_profile
                .insert(profile.id.clone(), start..actions.len())
                .is_none());
        }
        Catalog {
            profiles: source.profiles,
            actions,
            by_id,
            by_executable,
            by_profile,
        }
    })
}
pub fn find(id: &str) -> Option<&'static Shortcut> {
    let catalog = catalog();
    catalog.by_id.get(id).map(|&index| &catalog.actions[index])
}
pub fn uses_invocation_shortcut(action: &Shortcut) -> bool {
    action
        .keys
        .split_whitespace()
        .any(|chord| chord == "Ctrl+Backquote")
}
pub fn for_executable(executable: &str) -> Option<&'static Profile> {
    let catalog = catalog();
    catalog
        .by_executable
        .get(&executable.to_ascii_lowercase())
        .map(|&index| &catalog.profiles[index])
}
pub fn for_profile(profile: &str) -> std::slice::Iter<'static, Shortcut> {
    let c = catalog();
    c.by_profile
        .get(profile)
        .map_or(&[][..], |range| &c.actions[range.clone()])
        .iter()
}

/// Small fixed grammar, never supplied by the webview. Plus means the physical '=' key.
pub fn parse_keys(keys: &str) -> Result<Vec<Vec<u16>>, String> {
    fn key(token: &str) -> Option<u16> {
        Some(match token {
            "Ctrl" => 0x11,
            "Alt" => 0x12,
            "Shift" | "ShiftKey" => 0x10,
            "Enter" => 0x0D,
            "Escape" => 0x1B,
            "Tab" => 0x09,
            "Space" => 0x20,
            "Left" => 0x25,
            "Up" => 0x26,
            "Right" => 0x27,
            "Down" => 0x28,
            "Home" => 0x24,
            "End" => 0x23,
            "PageUp" => 0x21,
            "PageDown" => 0x22,
            "Backspace" => 0x08,
            "Delete" => 0x2E,
            "Insert" => 0x2D,
            "Plus" => 0xBB,
            "Minus" => 0xBD,
            "Comma" => 0xBC,
            "Period" => 0xBE,
            "Slash" => 0xBF,
            "Backslash" => 0xDC,
            "BracketLeft" => 0xDB,
            "BracketRight" => 0xDD,
            "Semicolon" => 0xBA,
            "Apostrophe" => 0xDE,
            "Backquote" => 0xC0,
            "Decimal" => 0x6E,
            _ if token.len() == 1
                && token.as_bytes()[0].is_ascii_alphanumeric()
                && !token.as_bytes()[0].is_ascii_lowercase() =>
            {
                token.as_bytes()[0] as u16
            }
            _ if token.starts_with("Numpad") => {
                let n = token[6..].parse::<u16>().ok()?;
                if n > 9 {
                    return None;
                }
                0x60 + n
            }
            _ if token.starts_with('F') => {
                let n = token[1..].parse::<u16>().ok()?;
                if !(1..=24).contains(&n) {
                    return None;
                }
                0x70 + n - 1
            }
            _ => return None,
        })
    }
    let mut sequence = Vec::new();
    for chord in keys.split_whitespace() {
        let tokens: Vec<_> = chord.split('+').collect();
        if tokens.is_empty() || tokens.len() > 4 {
            return Err("Invalid chord".into());
        }
        let mut parsed = Vec::new();
        for (index, token) in tokens.iter().enumerate() {
            if index + 1 == tokens.len() && matches!(*token, "Ctrl" | "Alt" | "Shift") {
                return Err("A shortcut must end in a key".into());
            }
            let vk = key(token).ok_or_else(|| format!("Unknown shortcut key: {token}"))?;
            if index + 1 < tokens.len() && !matches!(*token, "Ctrl" | "Alt" | "Shift") {
                return Err("Only modifiers may precede a key".into());
            }
            if parsed.contains(&vk) {
                return Err("Duplicate shortcut key".into());
            }
            parsed.push(vk);
        }
        sequence.push(parsed);
    }
    if sequence.is_empty() || sequence.len() > 3 {
        return Err("Invalid shortcut sequence".into());
    }
    Ok(sequence)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bundled_catalog_is_complete_and_resolvable() {
        let c = catalog();
        assert!(c.actions.len() >= 2000);
        assert!(c.profiles.len() >= 40);
        for profile in &c.profiles {
            assert_eq!(profile.quick.len(), 6);
            assert_eq!(
                for_profile(&profile.id)
                    .filter(|a| a.priority.is_some())
                    .count(),
                6
            );
            assert!(!profile.source.is_empty());
            assert!(for_profile(&profile.id).count() > 10);
            for executable in &profile.executables {
                assert_eq!(
                    for_executable(&executable.to_uppercase()).unwrap().id,
                    profile.id
                );
            }
        }
        for action in &c.actions {
            assert_eq!(find(&action.id).unwrap().keys, action.keys);
            assert!(!action.category.is_empty());
        }
        assert!(for_executable("not-photoshop.exe").is_none());
        assert!(for_executable("C:\\photoshop.exe").is_none());
        assert!(find("app:photoshop:arbitrary-command").is_none());
    }
    #[test]
    fn parses_sequences_and_rejects_unbounded_or_ambiguous_input() {
        assert_eq!(
            parse_keys("Ctrl+K Ctrl+C").unwrap(),
            vec![vec![0x11, 0x4B], vec![0x11, 0x43]]
        );
        assert_eq!(
            parse_keys("ShiftKey ShiftKey").unwrap(),
            vec![vec![0x10], vec![0x10]]
        );
        for invalid in [
            "Ctrl",
            "Alt",
            "Shift",
            "Ctrl+Shift",
            "",
            "Ctrl+",
            "Ctrl+Ctrl+S",
            "S+Ctrl",
            "Win+R",
            "A A A A",
            "F25",
            "Numpad10",
        ] {
            assert!(parse_keys(invalid).is_err(), "{invalid}");
        }
    }
    #[test]
    fn detects_only_the_global_invocation_collision() {
        assert!(uses_invocation_shortcut(
            find("app:vscode:focus-terminal").unwrap()
        ));
        assert!(uses_invocation_shortcut(
            find("app:excel:toggle-formulas").unwrap()
        ));
        assert!(!uses_invocation_shortcut(
            find("app:vscode:new-terminal").unwrap()
        ));
        assert!(!uses_invocation_shortcut(
            find("app:figma:frame-tool").unwrap()
        ));
    }
}
