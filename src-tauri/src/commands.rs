use crate::models::{
    AppState, ConflictProcess, Diagnostics, LayoutOrientation, LogLine, Profile, ProfilesFile,
    ServiceName, ServiceStatus, Settings, TestResult, UpdateCheck,
};
use crate::state::RuntimeState;
use crate::{
    conflicts, diagnostics, logging, presets, profiles, report, services, settings, tester, updater,
    windowing,
};
use std::sync::Mutex;
use tauri::{AppHandle, Manager, State};
use tauri_plugin_opener::OpenerExt;

#[tauri::command]
pub fn get_app_state(state: State<Mutex<RuntimeState>>) -> AppState {
    services::refresh_status(&state);
    state.lock().unwrap().app_state.clone()
}

#[tauri::command]
pub fn get_settings() -> Result<Settings, String> {
    settings::load_settings()
}

#[tauri::command]
pub fn save_settings(
    state: State<Mutex<RuntimeState>>,
    settings_payload: Settings,
) -> Result<Settings, String> {
    settings::save_settings(&settings_payload)?;
    state.lock().unwrap().settings = settings_payload.clone();
    Ok(settings_payload)
}

#[tauri::command]
pub fn get_profiles() -> Result<ProfilesFile, String> {
    profiles::load_profiles()
}

#[tauri::command]
pub fn save_profile(profile: Profile) -> Result<ProfilesFile, String> {
    profiles::save_profile(profile)
}

#[tauri::command]
pub fn set_active_profile(
    state: State<Mutex<RuntimeState>>,
    profile_id: String,
) -> Result<ProfilesFile, String> {
    let file = profiles::set_active_profile(profile_id.clone())?;
    state.lock().unwrap().app_state.active_profile_id = profile_id;
    Ok(file)
}

#[tauri::command]
pub fn delete_profile(
    state: State<Mutex<RuntimeState>>,
    profile_id: String,
) -> Result<ProfilesFile, String> {
    let file = profiles::delete_profile(profile_id)?;
    state.lock().unwrap().app_state.active_profile_id = file.active_profile_id.clone();
    Ok(file)
}

#[tauri::command]
pub async fn discover_zapret() -> Result<Vec<crate::models::Preset>, String> {
    tauri::async_runtime::spawn_blocking(presets::discover_presets)
        .await
        .map_err(|error| error.to_string())?
}

#[tauri::command]
pub async fn list_presets() -> Result<Vec<crate::models::Preset>, String> {
    tauri::async_runtime::spawn_blocking(presets::discover_presets)
        .await
        .map_err(|error| error.to_string())?
}

#[tauri::command]
pub async fn set_preset_favorite(
    preset_id: String,
    favorite: bool,
) -> Result<Vec<crate::models::Preset>, String> {
    tauri::async_runtime::spawn_blocking(move || presets::set_favorite(preset_id, favorite))
        .await
        .map_err(|error| error.to_string())?
}

#[tauri::command]
pub async fn set_preset_hidden(
    preset_id: String,
    hidden: bool,
) -> Result<Vec<crate::models::Preset>, String> {
    tauri::async_runtime::spawn_blocking(move || presets::set_hidden(preset_id, hidden))
        .await
        .map_err(|error| error.to_string())?
}

#[tauri::command]
pub async fn start_profile(app: AppHandle) -> Result<AppState, String> {
    let thread_app = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let state = thread_app.state::<Mutex<RuntimeState>>();
        services::start_active_profile(&thread_app, &state)
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
pub async fn stop_profile(app: AppHandle) -> Result<AppState, String> {
    let thread_app = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let state = thread_app.state::<Mutex<RuntimeState>>();
        services::stop_active_profile(&thread_app, &state)
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
pub async fn start_zapret(app: AppHandle, preset_id: String) -> Result<ServiceStatus, String> {
    let thread_app = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let state = thread_app.state::<Mutex<RuntimeState>>();
        services::start_zapret(&thread_app, &state, preset_id)
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
pub async fn stop_zapret(app: AppHandle) -> Result<ServiceStatus, String> {
    let thread_app = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let state = thread_app.state::<Mutex<RuntimeState>>();
        services::stop_zapret(&thread_app, &state)
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
pub fn get_conflicting_apps() -> Vec<ConflictProcess> {
    conflicts::detect()
}

#[tauri::command]
pub fn kill_conflicting_apps(pids: Vec<u32>) -> Vec<ConflictProcess> {
    conflicts::kill(&pids)
}

#[tauri::command]
pub async fn start_tg_ws(
    app: AppHandle,
    host: String,
    port: u16,
    secret: String,
) -> Result<ServiceStatus, String> {
    let thread_app = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let state = thread_app.state::<Mutex<RuntimeState>>();
        services::start_tg_ws(&thread_app, &state, host, port, secret)
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
pub async fn stop_tg_ws(app: AppHandle) -> Result<ServiceStatus, String> {
    let thread_app = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let state = thread_app.state::<Mutex<RuntimeState>>();
        services::stop_tg_ws(&thread_app, &state)
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
pub fn get_service_status(
    state: State<Mutex<RuntimeState>>,
    service: ServiceName,
) -> ServiceStatus {
    services::refresh_status(&state);
    let runtime = state.lock().unwrap();
    match service {
        ServiceName::Zapret => runtime.app_state.zapret.clone(),
        ServiceName::TgWs => runtime.app_state.tg_ws.clone(),
    }
}

#[tauri::command]
pub fn get_logs(state: State<Mutex<RuntimeState>>) -> Vec<LogLine> {
    let mut runtime = state.lock().unwrap();
    if runtime.logs.is_empty() {
        runtime.logs = logging::load_recent(250);
    }
    runtime.logs.clone()
}

#[tauri::command]
pub fn clear_logs(state: State<Mutex<RuntimeState>>) -> Result<(), String> {
    logging::clear(&state)
}

#[tauri::command]
pub fn open_path(app: AppHandle, path: String) -> Result<(), String> {
    app.opener()
        .open_path(path, None::<&str>)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn open_url(app: AppHandle, url: String) -> Result<(), String> {
    if !url.starts_with("tg://proxy?") && !updater::is_allowed_release_url(&url) {
        return Err("Only Telegram proxy links and ZUI release links can be opened.".into());
    }

    app.opener()
        .open_url(url, None::<&str>)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn reveal_path(app: AppHandle, path: String) -> Result<(), String> {
    app.opener()
        .reveal_item_in_dir(path)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn minimize_to_tray(app: AppHandle) {
    crate::tray::hide_main_window(&app);
}

#[tauri::command]
pub fn quit_app(app: AppHandle) {
    let exit_app = app.clone();
    tauri::async_runtime::spawn(async move {
        let thread_app = exit_app.clone();
        let _ = tauri::async_runtime::spawn_blocking(move || {
            let state = thread_app.state::<Mutex<RuntimeState>>();
            state.lock().unwrap().shutting_down = true;
            let _ = services::stop_active_profile(&thread_app, &state);
        })
        .await;
        exit_app.exit(0);
    });
}

#[tauri::command]
pub fn show_main_window(app: AppHandle) {
    crate::tray::show_main_window(&app);
}

#[tauri::command]
pub fn set_window_layout(app: AppHandle, layout: LayoutOrientation) -> Result<(), String> {
    windowing::apply_layout(&app, &layout)
}

#[tauri::command]
pub fn run_preset_test(app: AppHandle, preset_id: String) -> Result<String, String> {
    tester::run_quick_test(app, preset_id)
}

#[tauri::command]
pub fn run_best_preset_test(
    app: AppHandle,
    preset_ids: Vec<String>,
    max_count: usize,
) -> Result<String, String> {
    tester::run_best_preset_test(app, preset_ids, max_count)
}

#[tauri::command]
pub fn run_all_preset_test(app: AppHandle, preset_ids: Vec<String>) -> Result<String, String> {
    tester::run_all_preset_test(app, preset_ids)
}

#[tauri::command]
pub fn cancel_preset_test(app: AppHandle) {
    let state = app.state::<Mutex<RuntimeState>>();
    tester::cancel_with_app(&app, &state);
}

#[tauri::command]
pub fn get_test_results(state: State<Mutex<RuntimeState>>) -> Result<Vec<TestResult>, String> {
    let mut runtime = state.lock().unwrap();
    if runtime.test_results.is_empty() {
        runtime.test_results = tester::load_results()?;
    }
    Ok(runtime.test_results.clone())
}

#[tauri::command]
pub async fn get_diagnostics(app: AppHandle) -> Diagnostics {
    let thread_app = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let state = thread_app.state::<Mutex<RuntimeState>>();
        services::refresh_status(&state);
        diagnostics::collect(Some(&state))
    })
    .await
    .unwrap_or_else(|_| diagnostics::collect(None))
}

#[tauri::command]
pub fn collect_support_report(app: AppHandle) -> Result<String, String> {
    let state = app.state::<Mutex<RuntimeState>>();
    let report = report::collect(&state)?;
    logging::push(&app, &state, crate::models::LogSource::App, "Support report collected");
    Ok(report)
}

#[tauri::command]
pub async fn check_for_update(app: AppHandle) -> Result<UpdateCheck, String> {
    {
        let state = app.state::<Mutex<RuntimeState>>();
        logging::push(&app, &state, crate::models::LogSource::App, "Checking for updates");
    }
    let result = tauri::async_runtime::spawn_blocking(updater::check)
        .await
        .map_err(|error| error.to_string())?;
    let state = app.state::<Mutex<RuntimeState>>();
    match &result {
        Ok(update) => logging::push(
            &app,
            &state,
            crate::models::LogSource::App,
            format!(
                "Update check finished: current={}, latest={}, available={}",
                update.current_version,
                update.latest_version.clone().unwrap_or_default(),
                update.update_available
            ),
        ),
        Err(error) => logging::push(
            &app,
            &state,
            crate::models::LogSource::App,
            format!("Update check failed: {error}"),
        ),
    }
    result
}

#[tauri::command]
pub async fn install_update(app: AppHandle) -> Result<(), String> {
    {
        let state = app.state::<Mutex<RuntimeState>>();
        logging::push(
            &app,
            &state,
            crate::models::LogSource::App,
            "Downloading update installer",
        );
    }
    let installer_path = tauri::async_runtime::spawn_blocking(updater::download_installer)
        .await
        .map_err(|error| error.to_string())??;
    {
        let state = app.state::<Mutex<RuntimeState>>();
        logging::push(
            &app,
            &state,
            crate::models::LogSource::App,
            format!("Update installer downloaded: {}", installer_path.display()),
        );
    }

    let stop_app = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let state = stop_app.state::<Mutex<RuntimeState>>();
        state.lock().unwrap().shutting_down = true;
        services::stop_active_profile(&stop_app, &state)
    })
    .await
    .map_err(|error| error.to_string())??;

    updater::launch_installer(&installer_path)?;
    let state = app.state::<Mutex<RuntimeState>>();
    logging::push(
        &app,
        &state,
        crate::models::LogSource::App,
        "Update installer launched",
    );

    let exit_app = app.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(250)).await;
        exit_app.exit(0);
    });

    Ok(())
}
