mod commands;
mod conflicts;
mod diagnostics;
mod logging;
mod models;
mod paths;
mod presets;
mod profiles;
mod runtime;
mod services;
mod settings;
mod state;
mod system_process;
mod tester;
mod tray;
mod updater;
mod windowing;

use commands::*;
use state::RuntimeState;
use std::sync::Mutex;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let _ = rustls::crypto::ring::default_provider().install_default();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(Mutex::new(RuntimeState::default()))
        .invoke_handler(tauri::generate_handler![
            get_app_state,
            get_settings,
            save_settings,
            get_profiles,
            save_profile,
            set_active_profile,
            delete_profile,
            discover_zapret,
            list_presets,
            set_preset_favorite,
            set_preset_hidden,
            start_profile,
            stop_profile,
            start_zapret,
            stop_zapret,
            get_conflicting_apps,
            kill_conflicting_apps,
            start_tg_ws,
            stop_tg_ws,
            get_service_status,
            get_logs,
            clear_logs,
            open_path,
            open_url,
            reveal_path,
            minimize_to_tray,
            quit_app,
            show_main_window,
            set_window_layout,
            run_preset_test,
            run_best_preset_test,
            run_all_preset_test,
            cancel_preset_test,
            get_test_results,
            get_diagnostics,
            check_for_update,
            install_update
        ])
        .setup(|app| {
            paths::ensure_data_layout()?;
            let settings = settings::load_settings().unwrap_or_default();
            let layout_orientation = settings.layout_orientation.clone();
            let launch_minimized = settings.launch_minimized;
            let auto_start_active_profile_on_launch = settings.auto_start_active_profile_on_launch;
            let profiles = profiles::load_profiles().unwrap_or_default();
            let active_profile_id = profiles.active_profile_id.clone();
            let active_profile_autostart = profiles
                .profiles
                .iter()
                .find(|profile| profile.id == active_profile_id)
                .and_then(|profile| profile.autostart_on_app_launch)
                .unwrap_or(false);
            let runtime_state = app.state::<Mutex<RuntimeState>>();
            let mut runtime = runtime_state.lock().unwrap();
            runtime.app_state.active_profile_id = active_profile_id;
            runtime.settings = settings;
            drop(runtime);
            services::restore_owned_processes(&runtime_state);
            tray::setup(app)?;
            windowing::apply_layout(app.handle(), &layout_orientation).ok();
            if launch_minimized {
                tray::hide_main_window(app.handle());
            }
            if auto_start_active_profile_on_launch || active_profile_autostart {
                tray::start_active_profile(app.handle());
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
