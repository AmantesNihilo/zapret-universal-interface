use crate::models::{Preset, PresetKind, PresetPreferences};
use crate::{paths, settings};
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

pub fn discover_presets() -> Result<Vec<Preset>, String> {
    let mut presets = Vec::new();
    for root in preset_roots()? {
        if root.exists() {
            visit_dir(&root, &root, &mut presets)?;
        }
    }

    let preferences = load_preferences()?;
    for preset in &mut presets {
        if let Some(preference) = preferences.get(&preset.id) {
            preset.favorite = preference.favorite;
            preset.hidden = preference.hidden;
        }
    }
    presets.sort_by(|left, right| {
        right
            .favorite
            .cmp(&left.favorite)
            .then_with(|| left.hidden.cmp(&right.hidden))
            .then_with(|| {
                left.relative_path
                    .to_lowercase()
                    .cmp(&right.relative_path.to_lowercase())
            })
    });
    Ok(presets)
}

fn preset_roots() -> Result<Vec<PathBuf>, String> {
    let mut roots = vec![paths::resources_zapret_dir()];
    if let Ok(settings) = settings::load_settings() {
        for root in settings.custom_preset_roots {
            let path = PathBuf::from(root);
            if !roots
                .iter()
                .any(|existing| same_path_loose(existing, &path))
            {
                roots.push(path);
            }
        }
    }
    Ok(roots)
}

pub fn find_preset(preset_id: &str) -> Result<Preset, String> {
    discover_presets()?
        .into_iter()
        .find(|preset| preset.id == preset_id)
        .ok_or_else(|| "Preset not found".into())
}

pub fn set_favorite(preset_id: String, favorite: bool) -> Result<Vec<Preset>, String> {
    update_preference(preset_id, |preference| preference.favorite = favorite)?;
    discover_presets()
}

pub fn set_hidden(preset_id: String, hidden: bool) -> Result<Vec<Preset>, String> {
    update_preference(preset_id, |preference| preference.hidden = hidden)?;
    discover_presets()
}

fn update_preference(
    preset_id: String,
    update: impl FnOnce(&mut PresetPreferences),
) -> Result<(), String> {
    let mut preferences = load_preferences()?;
    let preference = preferences.entry(preset_id).or_default();
    update(preference);
    save_preferences(&preferences)
}

fn load_preferences() -> Result<HashMap<String, PresetPreferences>, String> {
    paths::ensure_data_layout().map_err(|error| error.to_string())?;
    let path = paths::preset_preferences_path();
    if !path.exists() {
        save_preferences(&HashMap::new())?;
        return Ok(HashMap::new());
    }

    let text = std::fs::read_to_string(path).map_err(|error| error.to_string())?;
    serde_json::from_str(&text).map_err(|error| error.to_string())
}

fn save_preferences(preferences: &HashMap<String, PresetPreferences>) -> Result<(), String> {
    paths::ensure_data_layout().map_err(|error| error.to_string())?;
    let text = serde_json::to_string_pretty(preferences).map_err(|error| error.to_string())?;
    std::fs::write(paths::preset_preferences_path(), text).map_err(|error| error.to_string())
}

fn visit_dir(root: &Path, dir: &Path, presets: &mut Vec<Preset>) -> Result<(), String> {
    for entry in std::fs::read_dir(dir).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        let path = entry.path();
        if path.is_dir() {
            if !is_service_dir(&path) {
                visit_dir(root, &path, presets)?;
            }
            continue;
        }

        if let Some(kind) = preset_kind(root, &path) {
            let relative = path
                .strip_prefix(root)
                .unwrap_or(&path)
                .to_string_lossy()
                .replace('\\', "/");
            let name = path
                .file_stem()
                .and_then(|name| name.to_str())
                .unwrap_or("Preset")
                .to_string();
            presets.push(Preset {
                id: stable_id(&path),
                name,
                path: path.to_string_lossy().to_string(),
                relative_path: relative,
                kind,
                favorite: false,
                hidden: false,
            });
        }
    }
    Ok(())
}

fn preset_kind(root: &Path, path: &Path) -> Option<PresetKind> {
    let ext = path.extension()?.to_string_lossy().to_lowercase();
    let kind = match ext.as_str() {
        "bat" => PresetKind::Bat,
        "cmd" => PresetKind::Cmd,
        _ => return None,
    };

    if is_service_script(path) || !looks_like_zapret_preset(root, path) {
        return None;
    }

    Some(kind)
}

fn looks_like_zapret_preset(root: &Path, path: &Path) -> bool {
    let Ok(relative) = path.strip_prefix(root) else {
        return false;
    };
    if relative
        .components()
        .any(|component| is_service_component(&component.as_os_str().to_string_lossy()))
    {
        return false;
    }

    if looks_like_named_preset(relative) {
        return true;
    }

    has_winws_hint(path)
}

fn looks_like_named_preset(relative: &Path) -> bool {
    let relative_text = relative
        .to_string_lossy()
        .replace('\\', "/")
        .to_ascii_lowercase();
    let stem = relative
        .file_stem()
        .and_then(|name| name.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();

    relative_text.contains("/pre-configs/")
        || stem == "general"
        || stem.starts_with("general ")
        || stem.starts_with("general_")
        || stem.starts_with("general-")
        || stem.starts_with("discord")
        || stem.starts_with("youtube")
        || stem.starts_with("ultimate")
        || stem.starts_with("russia")
        || stem.starts_with("ubisoft")
        || stem.starts_with("preset")
        || stem.starts_with("aggressive")
        || stem.starts_with("gaming")
        || stem.starts_with("minimal")
        || stem.starts_with("smart")
        || stem.starts_with("stealth")
        || stem.starts_with("zapret")
}

fn has_winws_hint(path: &Path) -> bool {
    std::fs::read(path)
        .map(|content| contains_ascii_case_insensitive(&content, b"winws.exe"))
        .unwrap_or(false)
}

fn contains_ascii_case_insensitive(haystack: &[u8], needle: &[u8]) -> bool {
    if needle.is_empty() || haystack.len() < needle.len() {
        return false;
    }

    haystack.windows(needle.len()).any(|window| {
        window
            .iter()
            .zip(needle)
            .all(|(left, right)| left.eq_ignore_ascii_case(right))
    })
}

fn is_service_dir(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(is_service_component)
        .unwrap_or(false)
}

fn is_service_component(name: &str) -> bool {
    matches!(
        name.to_ascii_lowercase().as_str(),
        ".git" | "bin" | "blockcheck" | "cygwin" | "docs" | "lists" | "utils"
    )
}

fn is_service_script(path: &Path) -> bool {
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();

    matches!(
        file_name.as_str(),
        "service.bat"
            | "service.cmd"
            | "blockcheck.cmd"
            | "blockcheck.bat"
            | "switch_game_mode.cmd"
            | "switch_game_mode.bat"
            | "cygwin-admin.cmd"
            | "cygwin-admin.bat"
    )
}

fn stable_id(path: &PathBuf) -> String {
    let mut hasher = DefaultHasher::new();
    path.to_string_lossy().to_lowercase().hash(&mut hasher);
    format!("preset-{:x}", hasher.finish())
}

fn same_path_loose(left: &Path, right: &Path) -> bool {
    left.to_string_lossy()
        .trim_end_matches(['\\', '/'])
        .eq_ignore_ascii_case(right.to_string_lossy().trim_end_matches(['\\', '/']))
}
