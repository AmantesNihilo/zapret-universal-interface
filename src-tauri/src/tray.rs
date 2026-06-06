use crate::state::RuntimeState;
use crate::{logging, services, settings};
use std::sync::Mutex;
use tauri::menu::MenuBuilder;
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{App, AppHandle, Emitter, Manager, WindowEvent};

const MENU_SHOW: &str = "tray_show";
const MENU_TURN_ON: &str = "tray_turn_on";
const MENU_TURN_OFF: &str = "tray_turn_off";
const MENU_SETTINGS: &str = "tray_settings";
const MENU_QUIT: &str = "tray_quit";

pub fn setup(app: &mut App) -> tauri::Result<()> {
    let labels = TrayLabels::new(settings::load_settings().unwrap_or_default().language);
    let menu = MenuBuilder::new(app)
        .text(MENU_SHOW, labels.show)
        .separator()
        .text(MENU_TURN_ON, labels.turn_on)
        .text(MENU_TURN_OFF, labels.turn_off)
        .separator()
        .text(MENU_SETTINGS, labels.settings)
        .text(MENU_QUIT, labels.quit)
        .build()?;

    let mut tray_builder = TrayIconBuilder::with_id("main")
        .menu(&menu)
        .tooltip(labels.tooltip)
        .show_menu_on_left_click(false)
        .on_menu_event(handle_menu_event)
        .on_tray_icon_event(|tray, event| {
            if matches!(
                event,
                TrayIconEvent::DoubleClick {
                    button: MouseButton::Left,
                    ..
                } | TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                }
            ) {
                show_main_window(tray.app_handle());
            }
        });

    if let Some(icon) = app.default_window_icon().cloned() {
        tray_builder = tray_builder.icon(icon);
    }
    tray_builder.build(app)?;

    if let Some(window) = app.get_webview_window("main") {
        let handle = app.handle().clone();
        window.on_window_event(move |event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                let runtime_state = handle.state::<Mutex<RuntimeState>>();
                {
                    let mut runtime = runtime_state.lock().unwrap();
                    if runtime.shutting_down {
                        return;
                    }
                    runtime.shutting_down = true;
                }
                let cleanup_app = handle.clone();
                api.prevent_close();
                tauri::async_runtime::spawn(async move {
                    let thread_app = cleanup_app.clone();
                    let _ = tauri::async_runtime::spawn_blocking(move || {
                        let state = thread_app.state::<Mutex<RuntimeState>>();
                        let _ = services::stop_active_profile(&thread_app, &state);
                    })
                    .await;
                    cleanup_app.exit(0);
                });
            }
        });
    }

    Ok(())
}

struct TrayLabels {
    show: &'static str,
    turn_on: &'static str,
    turn_off: &'static str,
    settings: &'static str,
    quit: &'static str,
    tooltip: &'static str,
}

impl TrayLabels {
    fn new(language: String) -> Self {
        if language == "ru" {
            Self {
                show: "Открыть ZUI",
                turn_on: "Запустить активный профиль",
                turn_off: "Остановить активный профиль",
                settings: "Настройки",
                quit: "Выйти из ZUI",
                tooltip: "ZUI - утилита zapret",
            }
        } else {
            Self {
                show: "Open ZUI",
                turn_on: "Start active profile",
                turn_off: "Stop active profile",
                settings: "Settings",
                quit: "Quit ZUI",
                tooltip: "ZUI - zapret utility",
            }
        }
    }
}

pub fn hide_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.hide();
    }
}

pub fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.set_focus();
    }
}

fn handle_menu_event(app: &AppHandle, event: tauri::menu::MenuEvent) {
    match event.id().as_ref() {
        MENU_SHOW => show_main_window(app),
        MENU_SETTINGS => {
            show_main_window(app);
            let _ = app.emit("tray_action", "settings");
        }
        MENU_TURN_ON => start_active_profile(app),
        MENU_TURN_OFF => stop_active_profile(app),
        MENU_QUIT => {
            stop_active_profile(app);
            app.exit(0);
        }
        _ => {}
    }
}

pub fn start_active_profile(app: &AppHandle) {
    let thread_app = app.clone();
    tauri::async_runtime::spawn(async move {
        let worker_app = thread_app.clone();
        let outcome = tauri::async_runtime::spawn_blocking(move || {
            let runtime_state = worker_app.state::<Mutex<RuntimeState>>();
            services::start_active_profile(&worker_app, &runtime_state)
        })
        .await;
        if let Ok(Err(error)) = outcome {
            let runtime_state = thread_app.state::<Mutex<RuntimeState>>();
            logging::push(
                &thread_app,
                &runtime_state,
                crate::models::LogSource::App,
                error,
            );
        }
    });
}

fn stop_active_profile(app: &AppHandle) {
    let thread_app = app.clone();
    tauri::async_runtime::spawn(async move {
        let worker_app = thread_app.clone();
        let outcome = tauri::async_runtime::spawn_blocking(move || {
            let runtime_state = worker_app.state::<Mutex<RuntimeState>>();
            services::stop_active_profile(&worker_app, &runtime_state)
        })
        .await;
        if let Ok(Err(error)) = outcome {
            let runtime_state = thread_app.state::<Mutex<RuntimeState>>();
            logging::push(
                &thread_app,
                &runtime_state,
                crate::models::LogSource::App,
                error,
            );
        }
    });
}
