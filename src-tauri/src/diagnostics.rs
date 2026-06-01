use crate::models::Diagnostics;
use crate::state::RuntimeState;
use crate::{paths, presets, profiles, runtime, system_process};
use std::net::TcpListener;
use std::process::{Command, Stdio};
use std::sync::Mutex;

pub fn collect(state: Option<&Mutex<RuntimeState>>) -> Diagnostics {
    let resources_path = paths::resources_zapret_dir();
    let data_path = paths::data_dir();
    let logs_path = paths::logs_dir();
    let presets = presets::discover_presets().unwrap_or_default();
    let profile = profiles::active_profile().ok();
    let winws_running = system_process::is_running("winws.exe");
    let tg_ws_running = state
        .map(|state| {
            state
                .lock()
                .unwrap()
                .tg_ws_runtime
                .as_ref()
                .map(|handle| handle.is_running())
                .unwrap_or(false)
        })
        .unwrap_or(false);
    let selected_preset_exists = profile
        .as_ref()
        .and_then(|profile| profile.zapret_preset_id.as_ref())
        .map(|id| presets.iter().any(|preset| &preset.id == id))
        .unwrap_or(false);
    let tg_ws_port_available = profile
        .as_ref()
        .map(|profile| {
            tg_ws_running
                || TcpListener::bind((&profile.tg_ws_host[..], profile.tg_ws_port)).is_ok()
        })
        .unwrap_or(true);

    let winws_found = paths::find_file(&resources_path, "winws.exe").is_some();
    let tg_ws_found = true;
    let is_admin = is_admin();
    let mut warnings = Vec::new();

    if !is_admin {
        warnings.push("Administrator rights are not detected. zapret may fail to start.".into());
    }
    if presets.is_empty() {
        warnings.push("No zapret presets were discovered.".into());
    }
    if profile
        .as_ref()
        .and_then(|profile| profile.zapret_preset_id.as_ref())
        .is_some()
        && !selected_preset_exists
    {
        warnings.push("Selected zapret preset is missing.".into());
    }
    if !winws_found {
        warnings.push("winws.exe was not found in zapret resources.".into());
    }
    if profile
        .as_ref()
        .map(|profile| profile.tg_ws_enabled && !tg_ws_port_available)
        .unwrap_or(false)
    {
        warnings.push("tg-ws port is busy or unavailable.".into());
    }

    Diagnostics {
        resources_path: resources_path.to_string_lossy().to_string(),
        data_path: data_path.to_string_lossy().to_string(),
        logs_path: logs_path.to_string_lossy().to_string(),
        preset_count: presets.len(),
        selected_preset_exists,
        winws_found,
        tg_ws_found,
        tg_ws_engine: runtime::tg_ws::ENGINE_NAME.into(),
        tg_ws_engine_version: runtime::tg_ws::ENGINE_VERSION.into(),
        winws_running,
        tg_ws_running,
        is_admin,
        tg_ws_port_available,
        warnings,
    }
}

#[cfg(windows)]
fn is_admin() -> bool {
    Command::new("net")
        .arg("session")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .creation_flags(0x08000000)
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

#[cfg(not(windows))]
fn is_admin() -> bool {
    true
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
