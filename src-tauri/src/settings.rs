use crate::models::Settings;
use crate::{json_storage, paths};
use std::ffi::OsString;
use std::path::Path;
use std::process::{Command, Stdio};

pub const WINDOWS_STARTUP_ARG: &str = "--windows-startup";

pub fn load_settings() -> Result<Settings, String> {
    paths::ensure_data_layout().map_err(|error| error.to_string())?;
    let path = paths::settings_path();
    if !path.exists() {
        let settings = Settings::default();
        save_settings(&settings)?;
        return Ok(settings);
    }

    let text = std::fs::read_to_string(&path).map_err(|error| error.to_string())?;
    match json_storage::parse(&text) {
        Ok(settings) => {
            if text.starts_with('\u{feff}') {
                save_settings(&settings)?;
            }
            Ok(settings)
        }
        Err(error) => {
            let backup = json_storage::backup_invalid(&path)?;
            let settings = Settings::default();
            save_settings(&settings)?;
            eprintln!(
                "Stored settings were reset after a JSON error ({error}); backup: {}",
                backup.display()
            );
            Ok(settings)
        }
    }
}

pub fn save_settings(settings: &Settings) -> Result<(), String> {
    paths::ensure_data_layout().map_err(|error| error.to_string())?;
    let startup_changed = std::fs::read_to_string(paths::settings_path())
        .ok()
        .and_then(|text| json_storage::parse::<Settings>(&text).ok())
        .map(|previous| previous.start_with_windows != settings.start_with_windows)
        .unwrap_or(true);
    if startup_changed {
        sync_startup_registration(settings)?;
    }
    let text = serde_json::to_string_pretty(settings).map_err(|error| error.to_string())?;
    json_storage::write_atomic(&paths::settings_path(), text.as_bytes())
}

#[cfg(windows)]
pub fn sync_startup_registration(settings: &Settings) -> Result<(), String> {
    const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
    const APP_NAME: &str = "ZUI";

    if settings.start_with_windows {
        let exe = std::env::current_exe().map_err(|error| error.to_string())?;
        let value = startup_registry_value(&exe);
        let output = Command::new("reg")
            .args([
                "add",
                r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run",
                "/v",
                APP_NAME,
                "/t",
                "REG_SZ",
                "/d",
                &value,
                "/f",
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .creation_flags(0x08000000)
            .output()
            .map_err(|error| format!("Failed to run reg.exe for Windows startup: {error}"))?;
        if output.status.success() {
            Ok(())
        } else {
            let detail = String::from_utf8_lossy(&output.stderr).trim().to_string();
            Err(if detail.is_empty() {
                format!("Failed to update Windows startup key: {RUN_KEY}")
            } else {
                format!("Failed to update Windows startup key {RUN_KEY}: {detail}")
            })
        }
    } else {
        let _ = Command::new("reg")
            .args([
                "delete",
                r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run",
                "/v",
                APP_NAME,
                "/f",
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .creation_flags(0x08000000)
            .status();
        Ok(())
    }
}

#[cfg(not(windows))]
pub fn sync_startup_registration(_settings: &Settings) -> Result<(), String> {
    Ok(())
}

fn startup_registry_value(exe: &Path) -> String {
    format!("\"{}\" {WINDOWS_STARTUP_ARG}", exe.display())
}

pub fn is_windows_startup_launch() -> bool {
    has_windows_startup_arg(std::env::args_os())
}

fn has_windows_startup_arg(args: impl IntoIterator<Item = OsString>) -> bool {
    args.into_iter().any(|arg| arg == WINDOWS_STARTUP_ARG)
}

#[cfg(windows)]
trait CommandExtHidden {
    fn creation_flags(&mut self, flags: u32) -> &mut Self;
}

#[cfg(windows)]
impl CommandExtHidden for Command {
    fn creation_flags(&mut self, flags: u32) -> &mut Self {
        use std::os::windows::process::CommandExt;
        CommandExt::creation_flags(self, flags);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn startup_registry_value_quotes_executable_and_marks_startup() {
        let value = startup_registry_value(Path::new(r"C:\Program Files\ZUI\zui.exe"));
        assert_eq!(value, r#""C:\Program Files\ZUI\zui.exe" --windows-startup"#);
    }

    #[test]
    fn startup_argument_detection_is_exact() {
        assert!(has_windows_startup_arg([
            OsString::from("zui.exe"),
            OsString::from(WINDOWS_STARTUP_ARG),
        ]));
        assert!(!has_windows_startup_arg([
            OsString::from("zui.exe"),
            OsString::from("--windows-startup-extra"),
        ]));
    }
}
