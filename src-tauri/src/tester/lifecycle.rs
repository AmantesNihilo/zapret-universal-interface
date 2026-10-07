use crate::models::ServiceState;
use crate::services;
use crate::state::RuntimeState;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tauri::AppHandle;

const STARTUP_TIMEOUT: Duration = Duration::from_secs(6);
const STABLE_SAMPLES: usize = 2;

pub fn wait_until_ready(app: &AppHandle, state: &Mutex<RuntimeState>) -> Result<Vec<u32>, String> {
    let started = Instant::now();
    let mut stable = 0usize;
    let mut last_error = None;
    while started.elapsed() < STARTUP_TIMEOUT {
        if state.lock().unwrap().test_cancelled {
            return Err("Test cancelled during zapret startup".into());
        }
        services::refresh_status(app, state);
        let (running, pids, error) = {
            let runtime = state.lock().unwrap();
            let mut pids = runtime.zapret_winws_pids.clone();
            if let Some(child) = runtime.zapret_child.as_ref() {
                if !pids.contains(&child.id()) {
                    pids.push(child.id());
                }
            }
            (
                runtime.app_state.zapret.state == ServiceState::Running && !pids.is_empty(),
                pids,
                runtime.app_state.zapret.error.clone(),
            )
        };
        if running {
            stable += 1;
            if stable >= STABLE_SAMPLES {
                return Ok(pids);
            }
        } else {
            stable = 0;
            last_error = error.or(last_error);
        }
        std::thread::sleep(Duration::from_millis(350));
    }
    Err(last_error
        .unwrap_or_else(|| "zapret process did not remain ready before the startup timeout".into()))
}

pub fn is_healthy(app: &AppHandle, state: &Mutex<RuntimeState>) -> bool {
    services::refresh_status(app, state);
    let runtime = state.lock().unwrap();
    runtime.app_state.zapret.state == ServiceState::Running
        && (runtime.zapret_child.is_some() || !runtime.zapret_winws_pids.is_empty())
}
