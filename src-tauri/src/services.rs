use crate::models::{AppStatus, LogSource, PresetKind, ServiceName, ServiceState, ServiceStatus};
use crate::{logging, paths, presets, profiles, runtime, system_process};
use std::io::{BufRead, BufReader, Read};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager};

use crate::state::RuntimeState;

pub fn start_active_profile(
    app: &AppHandle,
    state: &Mutex<RuntimeState>,
) -> Result<crate::models::AppState, String> {
    let profile = profiles::active_profile()?;
    let mut zapret_started = false;

    {
        let mut runtime = state.lock().unwrap();
        runtime.app_state.status = AppStatus::Starting;
        runtime.app_state.last_error = None;
    }
    emit_state(app, state);

    if !profile.zapret_enabled && !profile.tg_ws_enabled {
        refresh_status(state);
        logging::push(
            app,
            state,
            LogSource::App,
            "No services enabled for profile",
        );
        emit_state(app, state);
        return Ok(state.lock().unwrap().app_state.clone());
    }

    if profile.zapret_enabled {
        let Some(preset_id) = profile.zapret_preset_id.clone() else {
            let error = "No zapret preset selected".to_string();
            set_error(app, state, Some(ServiceName::Zapret), error.clone());
            return Err(error);
        };
        if let Err(error) = start_zapret(app, state, preset_id) {
            set_error(app, state, Some(ServiceName::Zapret), error.clone());
            return Err(error);
        }
        zapret_started = true;
    }

    if profile.tg_ws_enabled {
        if let Err(error) = start_tg_ws(
            app,
            state,
            profile.tg_ws_host,
            profile.tg_ws_port,
            profile.tg_ws_secret,
        ) {
            if zapret_started {
                let _ = stop_zapret(app, state);
            }
            set_error(app, state, Some(ServiceName::TgWs), error.clone());
            return Err(error);
        }
    }

    refresh_status(state);
    logging::push(app, state, LogSource::App, "Profile started");
    emit_state(app, state);
    Ok(state.lock().unwrap().app_state.clone())
}

pub fn restore_owned_processes(state: &Mutex<RuntimeState>) {
    let owned = load_owned_winws_marker();
    if owned.is_empty() {
        return;
    }
    {
        let mut runtime = state.lock().unwrap();
        runtime.zapret_winws_pids = owned;
    }
    refresh_status(state);
}

pub fn stop_active_profile(
    app: &AppHandle,
    state: &Mutex<RuntimeState>,
) -> Result<crate::models::AppState, String> {
    {
        let mut runtime = state.lock().unwrap();
        runtime.app_state.status = AppStatus::Stopping;
        runtime.app_state.last_error = None;
    }
    emit_state(app, state);

    let mut errors = Vec::new();
    if let Err(error) = stop_tg_ws(app, state) {
        errors.push(error);
    }
    if let Err(error) = stop_zapret(app, state) {
        errors.push(error);
    }
    refresh_status(state);
    logging::push(app, state, LogSource::App, "Profile stopped");
    emit_state(app, state);
    if errors.is_empty() {
        Ok(state.lock().unwrap().app_state.clone())
    } else {
        let error = errors.join("; ");
        state.lock().unwrap().app_state.last_error = Some(error.clone());
        let _ = app.emit("operation_failed", error.clone());
        Err(error)
    }
}

pub fn start_zapret(
    app: &AppHandle,
    state: &Mutex<RuntimeState>,
    preset_id: String,
) -> Result<ServiceStatus, String> {
    refresh_status(state);
    if state.lock().unwrap().zapret_child.is_some() {
        return Ok(state.lock().unwrap().app_state.zapret.clone());
    }
    let known_winws = state.lock().unwrap().zapret_winws_pids.clone();
    let foreign_winws: Vec<u32> = system_process::image_pids("winws.exe")
        .into_iter()
        .filter(|pid| !known_winws.contains(pid))
        .collect();
    if !foreign_winws.is_empty() {
        return Err(format!(
            "Foreign winws.exe is already running: PID {}. Stop it before starting ZUI zapret.",
            foreign_winws
                .iter()
                .map(u32::to_string)
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    let winws_before = system_process::image_pids("winws.exe");

    let preset = presets::find_preset(&preset_id)?;
    if !matches!(preset.kind, PresetKind::Bat | PresetKind::Cmd) {
        return Err("Selected preset is not executable".into());
    }

    logging::push(
        app,
        state,
        LogSource::Zapret,
        format!("Starting preset: {}", preset.relative_path),
    );

    let launch_path = managed_zapret_script(&preset)?;
    let launch_dir = Path::new(&preset.path)
        .parent()
        .unwrap_or_else(|| Path::new("."));

    let mut command = Command::new("cmd");
    command
        .arg("/D")
        .arg("/Q")
        .arg("/C")
        .arg(&launch_path)
        .current_dir(launch_dir)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .creation_flags(0x08000000);

    let mut child = command.spawn().map_err(|error| error.to_string())?;
    let pid = child.id();
    let owned_winws = wait_for_owned_winws(&winws_before);
    if let Some(stdout) = child.stdout.take() {
        spawn_pipe_logger(app.clone(), LogSource::Zapret, stdout);
    }
    if let Some(stderr) = child.stderr.take() {
        spawn_pipe_logger(app.clone(), LogSource::Zapret, stderr);
    }

    let status = ServiceStatus {
        service: ServiceName::Zapret,
        state: ServiceState::Running,
        pid: Some(pid),
        message: Some(preset.name),
        error: None,
    };

    {
        let mut runtime = state.lock().unwrap();
        runtime.zapret_child = Some(child);
        runtime.zapret_winws_pids = owned_winws;
        write_owned_winws_marker(&runtime.zapret_winws_pids);
        runtime.app_state.zapret = status.clone();
        runtime.app_state.status = AppStatus::On;
        runtime.app_state.last_error = None;
    }
    emit_state(app, state);
    Ok(status)
}

pub fn stop_zapret(app: &AppHandle, state: &Mutex<RuntimeState>) -> Result<ServiceStatus, String> {
    let child = state.lock().unwrap().zapret_child.take();
    if let Some(mut child) = child {
        let pid = child.id();
        logging::push(
            app,
            state,
            LogSource::Zapret,
            format!("Stopping PID {}", pid),
        );
        let _ = Command::new("taskkill")
            .args(["/PID", &pid.to_string(), "/T", "/F"])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .creation_flags(0x08000000)
            .status();
        let _ = child.kill();
        wait_child_exit(&mut child, Duration::from_millis(1500));
    }
    let owned_winws = state.lock().unwrap().zapret_winws_pids.clone();
    if !owned_winws.is_empty() {
        logging::push(app, state, LogSource::Zapret, "Stopping owned winws.exe");
        for pid in &owned_winws {
            let _ = system_process::kill_pid(*pid);
        }
        let still_running: Vec<u32> = owned_winws
            .into_iter()
            .filter(|pid| system_process::is_pid_running(*pid))
            .collect();
        if !still_running.is_empty() {
            return Err(format!(
                "Failed to stop owned winws.exe PID {}. Run ZUI as administrator and try again.",
                still_running
                    .iter()
                    .map(u32::to_string)
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
    }
    state.lock().unwrap().zapret_winws_pids.clear();
    clear_owned_winws_marker();

    refresh_status(state);
    let status = state.lock().unwrap().app_state.zapret.clone();
    emit_state(app, state);
    Ok(status)
}

fn wait_child_exit(child: &mut Child, timeout: Duration) {
    let started = Instant::now();
    loop {
        if child.try_wait().ok().flatten().is_some() {
            break;
        }
        if started.elapsed() >= timeout {
            break;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

fn owned_winws_marker_path() -> PathBuf {
    paths::data_dir().join("runtime").join("zapret-owned-pids.txt")
}

fn write_owned_winws_marker(pids: &[u32]) {
    if pids.is_empty() {
        clear_owned_winws_marker();
        return;
    }
    let path = owned_winws_marker_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let text = pids
        .iter()
        .map(u32::to_string)
        .collect::<Vec<_>>()
        .join("\n");
    let _ = std::fs::write(path, text);
}

fn load_owned_winws_marker() -> Vec<u32> {
    let path = owned_winws_marker_path();
    let Ok(text) = std::fs::read_to_string(path) else {
        return Vec::new();
    };
    text.lines()
        .filter_map(|line| line.trim().parse::<u32>().ok())
        .filter(|pid| system_process::is_pid_running(*pid))
        .collect()
}

fn clear_owned_winws_marker() {
    let _ = std::fs::remove_file(owned_winws_marker_path());
}

fn managed_zapret_script(preset: &crate::models::Preset) -> Result<PathBuf, String> {
    let source_path = Path::new(&preset.path);
    let content = std::fs::read_to_string(source_path).map_err(|error| error.to_string())?;
    let source_dir = source_path.parent().unwrap_or_else(|| Path::new("."));
    let mut source_dir_text = source_dir.to_string_lossy().replace('/', "\\");
    if !source_dir_text.ends_with('\\') {
        source_dir_text.push('\\');
    }
    let mut changed = false;
    let mut output = String::with_capacity(content.len() + 128);

    for line in content.lines() {
        let line = line.replace("%~dp0", &source_dir_text);
        if skip_update_check_line(&line) {
            output.push_str("rem zui: disabled upstream update check");
            changed = true;
        } else if let Some(rewritten) = rewrite_winws_start_line(&line) {
            output.push_str(&rewritten);
            changed = true;
        } else {
            output.push_str(&line);
        }
        output.push_str("\r\n");
    }

    if !changed {
        return Ok(source_path.to_path_buf());
    }

    let runtime_dir = paths::data_dir().join("runtime");
    std::fs::create_dir_all(&runtime_dir).map_err(|error| error.to_string())?;
    let extension = source_path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("cmd");
    let script_path = runtime_dir.join(format!("zapret-managed-{}.{}", preset.id, extension));
    std::fs::write(&script_path, output).map_err(|error| error.to_string())?;
    Ok(script_path)
}

fn skip_update_check_line(line: &str) -> bool {
    let normalized = line
        .trim()
        .to_ascii_lowercase()
        .replace(".\\", "")
        .replace("\"", "");
    normalized == "call service.bat check_updates"
        || normalized == "call service.cmd check_updates"
        || normalized.ends_with("\\service.bat check_updates")
        || normalized.ends_with("\\service.cmd check_updates")
}

fn rewrite_winws_start_line(line: &str) -> Option<String> {
    let trimmed = line.trim_start();
    if !trimmed.get(..5)?.eq_ignore_ascii_case("start") {
        return None;
    }

    let lower = line.to_ascii_lowercase();
    let winws_index = lower.find("winws.exe")?;
    let executable_start = line[..winws_index].rfind('"').unwrap_or(winws_index);
    let direct = line[executable_start..].trim_start();

    Some(format!("rem zui: managed hidden winws launch\r\n{direct}"))
}

fn wait_for_owned_winws(before: &[u32]) -> Vec<u32> {
    for _ in 0..20 {
        let current = system_process::image_pids("winws.exe");
        let owned: Vec<u32> = current
            .into_iter()
            .filter(|pid| !before.contains(pid))
            .collect();
        if !owned.is_empty() {
            return owned;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    Vec::new()
}

pub fn start_tg_ws(
    app: &AppHandle,
    state: &Mutex<RuntimeState>,
    host: String,
    port: u16,
    secret: String,
) -> Result<ServiceStatus, String> {
    refresh_status(state);
    if state
        .lock()
        .unwrap()
        .tg_ws_runtime
        .as_ref()
        .map(|handle| handle.is_running())
        .unwrap_or(false)
    {
        let status = ServiceStatus {
            service: ServiceName::TgWs,
            state: ServiceState::Running,
            pid: None,
            message: Some(format!("{}:{}", host, port)),
            error: None,
        };
        {
            let mut runtime = state.lock().unwrap();
            runtime.app_state.tg_ws = status.clone();
            runtime.app_state.status = AppStatus::On;
        }
        emit_state(app, state);
        return Ok(status);
    }

    if TcpListener::bind((&host[..], port)).is_err() {
        return Err("tg-ws port is busy or unavailable".into());
    }

    logging::push(
        app,
        state,
        LogSource::TgWs,
        format!(
            "Starting {} {} on {}:{}",
            runtime::tg_ws::ENGINE_NAME,
            runtime::tg_ws::ENGINE_VERSION,
            host,
            port
        ),
    );
    let mut profile = profiles::active_profile()?;
    profile.tg_ws_host = host;
    profile.tg_ws_port = port;
    profile.tg_ws_secret = secret;
    let handle = runtime::tg_ws::spawn(&profile)?;

    let status = ServiceStatus {
        service: ServiceName::TgWs,
        state: ServiceState::Running,
        pid: None,
        message: Some(format!("{}:{} {}", handle.host, handle.port, handle.link)),
        error: None,
    };

    {
        let mut runtime = state.lock().unwrap();
        runtime.tg_ws_runtime = Some(handle);
        runtime.app_state.tg_ws = status.clone();
        runtime.app_state.status = AppStatus::On;
        runtime.app_state.last_error = None;
    }
    emit_state(app, state);
    Ok(status)
}

pub fn stop_tg_ws(app: &AppHandle, state: &Mutex<RuntimeState>) -> Result<ServiceStatus, String> {
    let runtime_handle = state.lock().unwrap().tg_ws_runtime.take();
    if let Some(runtime_handle) = runtime_handle {
        logging::push(
            app,
            state,
            LogSource::TgWs,
            format!(
                "Stopping {} on {}:{}",
                runtime::tg_ws::ENGINE_NAME,
                runtime_handle.host,
                runtime_handle.port
            ),
        );
        runtime_handle.stop()?;
    }

    refresh_status(state);
    let status = state.lock().unwrap().app_state.tg_ws.clone();
    emit_state(app, state);
    Ok(status)
}

pub fn refresh_status(state: &Mutex<RuntimeState>) {
    let mut runtime = state.lock().unwrap();
    let zapret_child_alive = runtime
        .zapret_child
        .as_mut()
        .map(|child| child.try_wait().ok().flatten().is_none())
        .unwrap_or(false);
    runtime
        .zapret_winws_pids
        .retain(|pid| system_process::is_pid_running(*pid));
    let winws_running = !runtime.zapret_winws_pids.is_empty();
    if !zapret_child_alive {
        runtime.zapret_child = None;
    }
    runtime.app_state.zapret = if zapret_child_alive || winws_running {
        let pid = runtime.zapret_child.as_ref().map(|child| child.id());
        ServiceStatus {
            service: ServiceName::Zapret,
            state: ServiceState::Running,
            pid: pid.or_else(|| runtime.zapret_winws_pids.first().copied()),
            message: Some(if zapret_child_alive {
                "preset process".into()
            } else {
                "owned winws.exe".into()
            }),
            error: None,
        }
    } else {
        ServiceStatus::stopped(ServiceName::Zapret)
    };

    let tg_ws_running = runtime
        .tg_ws_runtime
        .as_ref()
        .map(|handle| handle.is_running())
        .unwrap_or(false);
    if !tg_ws_running {
        runtime.tg_ws_runtime = None;
    }
    runtime.app_state.tg_ws = if tg_ws_running {
        let message = runtime
            .tg_ws_runtime
            .as_ref()
            .map(|handle| format!("{}:{} {}", handle.host, handle.port, handle.link));
        ServiceStatus {
            service: ServiceName::TgWs,
            state: ServiceState::Running,
            pid: None,
            message,
            error: None,
        }
    } else {
        ServiceStatus::stopped(ServiceName::TgWs)
    };

    runtime.app_state.status = if zapret_child_alive || winws_running || tg_ws_running {
        AppStatus::On
    } else {
        AppStatus::Off
    };
}

pub fn emit_state(app: &AppHandle, state: &Mutex<RuntimeState>) {
    let app_state = state.lock().unwrap().app_state.clone();
    let _ = app.emit("app_state_changed", app_state);
}

pub fn set_error(
    app: &AppHandle,
    state: &Mutex<RuntimeState>,
    service: Option<ServiceName>,
    message: impl Into<String>,
) {
    let message = message.into();
    {
        let mut runtime = state.lock().unwrap();
        runtime.app_state.status = AppStatus::Error;
        runtime.app_state.last_error = Some(message.clone());
        match service {
            Some(ServiceName::Zapret) => {
                runtime.app_state.zapret.state = ServiceState::Error;
                runtime.app_state.zapret.error = Some(message.clone());
            }
            Some(ServiceName::TgWs) => {
                runtime.app_state.tg_ws.state = ServiceState::Error;
                runtime.app_state.tg_ws.error = Some(message.clone());
            }
            None => {}
        }
    }
    let _ = app.emit("operation_failed", message);
    emit_state(app, state);
}

fn spawn_pipe_logger<R>(app: AppHandle, source: LogSource, reader: R)
where
    R: Read + Send + 'static,
{
    std::thread::spawn(move || {
        let runtime_state = app.state::<Mutex<RuntimeState>>();
        let mut reader = BufReader::new(reader);
        let mut buffer = Vec::new();

        loop {
            buffer.clear();
            match reader.read_until(b'\n', &mut buffer) {
                Ok(0) => break,
                Ok(_) => {
                    let line = String::from_utf8_lossy(&buffer).trim().to_string();
                    if !line.is_empty() {
                        logging::push(&app, &runtime_state, source.clone(), line);
                    }
                }
                Err(error) => {
                    logging::push(&app, &runtime_state, source.clone(), error.to_string());
                    break;
                }
            }
        }
    });
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

#[cfg(not(windows))]
trait CommandExtHidden {
    fn creation_flags(&mut self, _flags: u32) -> &mut Self;
}

#[cfg(not(windows))]
impl CommandExtHidden for Command {
    fn creation_flags(&mut self, _flags: u32) -> &mut Self {
        self
    }
}
