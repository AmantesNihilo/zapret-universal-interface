use std::path::{Path, PathBuf};

fn exe_dir() -> Option<PathBuf> {
    std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|parent| parent.to_path_buf()))
}

pub fn project_root() -> PathBuf {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    for dir in cwd.ancestors() {
        if dir.join("src-tauri").join("tauri.conf.json").exists() {
            return dir.to_path_buf();
        }
        if dir.join("tauri.conf.json").exists() {
            return dir.parent().unwrap_or(dir).to_path_buf();
        }
    }

    if let Some(exe_dir) = exe_dir() {
        if has_entries(&exe_dir.join("resources").join("zapret")) {
            return exe_dir;
        }
    }

    cwd
}

pub fn data_dir() -> PathBuf {
    let root = project_root();
    if let Some(exe_dir) = exe_dir() {
        if exe_dir.join("portable.flag").exists() {
            return exe_dir.join("data");
        }
    }

    if is_development_tree(&root) {
        return root.join("data");
    }

    installed_data_dir()
        .unwrap_or_else(|| root.join("data"))
}

pub fn distribution_mode() -> &'static str {
    if let Some(exe_dir) = exe_dir() {
        if exe_dir.join("portable.flag").exists() {
            return "portable";
        }
    }

    if is_development_tree(&project_root()) {
        "development"
    } else {
        "installed"
    }
}

fn installed_data_dir() -> Option<PathBuf> {
    std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("LOCALAPPDATA").map(PathBuf::from))
        .map(|path| path.join("ZUI"))
}

fn is_development_tree(root: &Path) -> bool {
    root.join("src-tauri").join("tauri.conf.json").exists()
        || (root.join("tauri.conf.json").exists() && root.join("src").exists())
}

pub fn logs_dir() -> PathBuf {
    data_dir().join("logs")
}

pub fn settings_path() -> PathBuf {
    data_dir().join("settings.json")
}

pub fn profiles_path() -> PathBuf {
    data_dir().join("profiles.json")
}

pub fn test_results_path() -> PathBuf {
    data_dir().join("test-results.json")
}

pub fn preset_preferences_path() -> PathBuf {
    data_dir().join("preset-preferences.json")
}

pub fn resources_zapret_dir() -> PathBuf {
    if let Some(exe_dir) = exe_dir() {
        let bundled = exe_dir.join("resources").join("zapret");
        if has_entries(&bundled) {
            return bundled;
        }
    }

    let local = project_root().join("resources").join("zapret");
    if has_entries(&local) {
        return local;
    }

    let sibling = project_root()
        .parent()
        .unwrap_or(&project_root())
        .join("zapret_preset_");
    if sibling.exists() {
        return sibling;
    }

    local
}

fn has_entries(path: &PathBuf) -> bool {
    std::fs::read_dir(path)
        .map(|mut entries| {
            entries.any(|entry| {
                entry
                    .ok()
                    .and_then(|entry| entry.file_name().into_string().ok())
                    .map(|name| name != ".gitkeep")
                    .unwrap_or(false)
            })
        })
        .unwrap_or(false)
}

pub fn find_file(root: &Path, file_name: &str) -> Option<PathBuf> {
    let entries = std::fs::read_dir(root).ok()?;

    for entry in entries.flatten() {
        let path = entry.path();
        if path
            .file_name()
            .and_then(|name| name.to_str())
            .map(|name| name.eq_ignore_ascii_case(file_name))
            .unwrap_or(false)
        {
            return Some(path);
        }
        if path.is_dir() {
            if let Some(found) = find_file(&path, file_name) {
                return Some(found);
            }
        }
    }

    None
}

pub fn ensure_data_layout() -> Result<(), std::io::Error> {
    std::fs::create_dir_all(logs_dir())?;
    migrate_legacy_install_data()?;
    let root = project_root();
    if is_development_tree(&root) {
        std::fs::create_dir_all(root.join("resources").join("zapret"))?;
    }
    Ok(())
}

fn migrate_legacy_install_data() -> Result<(), std::io::Error> {
    let target = data_dir();
    let root = project_root();
    if is_development_tree(&root) {
        return Ok(());
    }

    let Some(exe_dir) = exe_dir() else {
        return Ok(());
    };
    if exe_dir.join("portable.flag").exists() {
        return Ok(());
    }

    let legacy = exe_dir.join("data");
    if !legacy.exists() || legacy == target {
        return Ok(());
    }

    copy_missing_entries(&legacy, &target)
}

fn copy_missing_entries(source: &Path, target: &Path) -> Result<(), std::io::Error> {
    std::fs::create_dir_all(target)?;
    for entry in std::fs::read_dir(source)? {
        let entry = entry?;
        let source_path = entry.path();
        let target_path = target.join(entry.file_name());
        if source_path.is_dir() {
            copy_missing_entries(&source_path, &target_path)?;
        } else if target_path.exists() {
            continue;
        } else {
            std::fs::copy(&source_path, &target_path)?;
        }
    }
    Ok(())
}
