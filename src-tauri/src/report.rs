use crate::models::LogSource;
use crate::state::RuntimeState;
use crate::{diagnostics, logging, paths, presets, profiles, services};
use std::sync::Mutex;

pub fn collect(state: &Mutex<RuntimeState>) -> Result<String, String> {
    services::refresh_status(state);

    let app_state;
    let settings;
    let owned_winws_pids;
    let memory_logs;
    {
        let runtime = state.lock().unwrap();
        app_state = runtime.app_state.clone();
        settings = runtime.settings.clone();
        owned_winws_pids = runtime.zapret_winws_pids.clone();
        memory_logs = runtime.logs.clone();
    }

    let diagnostics = diagnostics::collect(Some(state));
    let profiles_file = profiles::load_profiles().unwrap_or_default();
    let active_profile = profiles_file
        .profiles
        .iter()
        .find(|profile| profile.id == profiles_file.active_profile_id)
        .cloned()
        .unwrap_or_default();
    let discovered_presets = presets::discover_presets().unwrap_or_default();
    let selected_preset = active_profile
        .zapret_preset_id
        .as_ref()
        .and_then(|id| discovered_presets.iter().find(|preset| &preset.id == id));
    let logs = if memory_logs.is_empty() {
        logging::load_recent(80)
    } else {
        memory_logs
    };

    let mut lines = vec![
        format!("ZUI {} support report", env!("CARGO_PKG_VERSION")),
        format!("created_at={}", timestamp()),
        format!("distribution={}", paths::distribution_mode()),
        String::new(),
        "[App]".into(),
        format!("status={:?}", app_state.status),
        format!("active_profile_id={}", app_state.active_profile_id),
        format!("last_error={}", app_state.last_error.unwrap_or_default()),
        format!(
            "zapret={:?} pid={} message={} error={}",
            app_state.zapret.state,
            app_state
                .zapret
                .pid
                .map(|pid| pid.to_string())
                .unwrap_or_default(),
            app_state.zapret.message.unwrap_or_default(),
            app_state.zapret.error.unwrap_or_default()
        ),
        format!(
            "tg_ws={:?} message={} error={}",
            app_state.tg_ws.state,
            app_state.tg_ws.message.unwrap_or_default(),
            app_state.tg_ws.error.unwrap_or_default()
        ),
        format!("owned_winws_pids={}", join_pids(&owned_winws_pids)),
        String::new(),
        "[Settings]".into(),
        format!("theme={:?}", settings.theme),
        format!("accent={}", settings.accent),
        format!("language={}", settings.language),
        format!("layout={:?}", settings.layout_orientation),
        format!(
            "auto_start_active_profile_on_launch={}",
            settings.auto_start_active_profile_on_launch
        ),
        format!("custom_preset_roots={}", settings.custom_preset_roots.len()),
        String::new(),
        "[Profile]".into(),
        format!("name={}", active_profile.name),
        format!("zapret_enabled={}", active_profile.zapret_enabled),
        format!(
            "zapret_preset={}",
            selected_preset
                .map(|preset| preset.relative_path.clone())
                .or(active_profile.zapret_preset_id.clone())
                .unwrap_or_default()
        ),
        format!("tg_ws_enabled={}", active_profile.tg_ws_enabled),
        format!("tg_ws={}:{}", active_profile.tg_ws_host, active_profile.tg_ws_port),
        String::new(),
        "[Diagnostics]".into(),
        format!("admin={}", diagnostics.is_admin),
        format!("resources={}", diagnostics.resources_path),
        format!("data={}", diagnostics.data_path),
        format!("logs={}", diagnostics.logs_path),
        format!("presets={}", diagnostics.preset_count),
        format!("selected_preset_exists={}", diagnostics.selected_preset_exists),
        format!("winws_found={}", diagnostics.winws_found),
        format!("winws_running={}", diagnostics.winws_running),
        format!("tg_ws_engine={} {}", diagnostics.tg_ws_engine, diagnostics.tg_ws_engine_version),
        format!("tg_ws_running={}", diagnostics.tg_ws_running),
        format!("tg_ws_port_available={}", diagnostics.tg_ws_port_available),
        format!("warnings={}", diagnostics.warnings.join(" | ")),
        String::new(),
        "[Recent logs]".into(),
    ];

    for line in logs.iter().rev().take(80).rev() {
        lines.push(format!(
            "[{}] {}: {}",
            line.timestamp,
            source_name(&line.source),
            line.message
        ));
    }

    Ok(lines.join("\n"))
}

fn join_pids(pids: &[u32]) -> String {
    pids.iter()
        .map(u32::to_string)
        .collect::<Vec<_>>()
        .join(",")
}

fn source_name(source: &LogSource) -> &'static str {
    match source {
        LogSource::App => "app",
        LogSource::Zapret => "zapret",
        LogSource::TgWs => "tg-ws",
        LogSource::Tests => "tests",
    }
}

fn timestamp() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    now.as_secs().to_string()
}
