use crate::models::{
    AppStatus, LogSource, PresetKind, ServiceName, ServiceState, ServiceStatus, ZapretEngine,
};
use crate::{logging, paths, power_intent, presets, profiles, runtime, system_process};
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
    let profile_id = profile.id.clone();
    let mut zapret_started = false;

    {
        let mut runtime = state.lock().unwrap();
        runtime.app_state.status = AppStatus::Starting;
        runtime.app_state.last_error = None;
    }
    emit_state(app, state);

    let start_zapret_service = match should_start_zapret(
        profile.zapret_enabled,
        profile.zapret_engine,
        profile.zapret_preset_id.as_deref(),
        profile.tg_ws_enabled,
    ) {
        Ok(value) => value,
        Err(error) => {
            set_error(app, state, Some(ServiceName::Zapret), error.clone());
            return Err(error);
        }
    };

    if !profile.zapret_enabled && !profile.tg_ws_enabled {
        refresh_status(app, state);
        logging::push(
            app,
            state,
            LogSource::App,
            "No services enabled for profile",
        );
        emit_state(app, state);
        return Ok(state.lock().unwrap().app_state.clone());
    }

    if profile.zapret_enabled && !start_zapret_service {
        logging::push(
            app,
            state,
            LogSource::App,
            "zapret is enabled but not configured; starting tg-ws only",
        );
    }

    if start_zapret_service {
        let Some(preset_id) = profile.zapret_preset_id.clone() else {
            let error = "No zapret preset selected".to_string();
            set_error(app, state, Some(ServiceName::Zapret), error.clone());
            return Err(error);
        };
        let Some(engine) = profile.zapret_engine else {
            let error = "No zapret engine selected".to_string();
            set_error(app, state, Some(ServiceName::Zapret), error.clone());
            return Err(error);
        };
        let preset = presets::find_preset(&preset_id)?;
        if preset.engine != engine {
            let error = "Selected preset belongs to another zapret engine".to_string();
            set_error(app, state, Some(ServiceName::Zapret), error.clone());
            return Err(error);
        }
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

    refresh_status(app, state);
    if let Err(error) = power_intent::remember_started(&profile_id) {
        logging::push(
            app,
            state,
            LogSource::App,
            format!("Could not remember active profile power state: {error}"),
        );
    }
    logging::push(app, state, LogSource::App, "Profile started");
    emit_state(app, state);
    Ok(state.lock().unwrap().app_state.clone())
}

fn should_start_zapret(
    enabled: bool,
    engine: Option<ZapretEngine>,
    preset_id: Option<&str>,
    tg_ws_enabled: bool,
) -> Result<bool, String> {
    if !enabled {
        return Ok(false);
    }
    if engine.is_some() && preset_id.is_some_and(|value| !value.trim().is_empty()) {
        return Ok(true);
    }
    if tg_ws_enabled {
        return Ok(false);
    }
    if engine.is_none() {
        Err("No zapret engine selected".into())
    } else {
        Err("No zapret preset selected".into())
    }
}

pub fn restore_owned_processes(app: &AppHandle, state: &Mutex<RuntimeState>) {
    let marker_exists = owned_winws_marker_path().exists();
    let (engine, owned) = load_owned_winws_marker();
    if owned.is_empty() {
        if marker_exists {
            clear_owned_winws_marker();
            logging::push(
                app,
                state,
                LogSource::App,
                "Crash recovery: stale zapret process marker was cleaned",
            );
        }
        return;
    }
    {
        let mut runtime = state.lock().unwrap();
        runtime.zapret_winws_pids = owned;
        runtime.active_zapret_engine = Some(engine);
    }
    refresh_status(app, state);
    let recovered = state.lock().unwrap().zapret_winws_pids.clone();
    write_owned_winws_marker(engine, &recovered);
    logging::push(
        app,
        state,
        LogSource::App,
        format!(
            "Crash recovery: attached to owned {} PID {}",
            engine.process_name(),
            recovered
                .iter()
                .map(u32::to_string)
                .collect::<Vec<_>>()
                .join(", ")
        ),
    );
}

pub fn stop_active_profile(
    app: &AppHandle,
    state: &Mutex<RuntimeState>,
) -> Result<crate::models::AppState, String> {
    let preserve_power_intent = state.lock().unwrap().shutting_down;
    if !preserve_power_intent {
        if let Err(error) = power_intent::clear() {
            logging::push(
                app,
                state,
                LogSource::App,
                format!("Could not clear active profile power state: {error}"),
            );
        }
    }
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
    refresh_status(app, state);
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
    refresh_status(app, state);
    let already_running = {
        let runtime = state.lock().unwrap();
        runtime.zapret_child.is_some() || !runtime.zapret_winws_pids.is_empty()
    };
    if already_running {
        return Ok(state.lock().unwrap().app_state.zapret.clone());
    }

    let preset = presets::find_preset(&preset_id).map_err(|error| {
        logged_start_failure(app, state, LogSource::Zapret, "preset lookup", error)
    })?;
    match preset.engine {
        ZapretEngine::Classic if !matches!(preset.kind, PresetKind::Bat | PresetKind::Cmd) => {
            return Err(logged_start_failure(
                app,
                state,
                LogSource::Zapret,
                "preset validation",
                "Selected Classic preset is not executable",
            ));
        }
        ZapretEngine::Zapret2 if !matches!(preset.kind, PresetKind::Config) => {
            return Err(logged_start_failure(
                app,
                state,
                LogSource::Zapret,
                "preset validation",
                "Selected Zapret 2 preset is not a config",
            ));
        }
        _ => {}
    }
    ensure_no_foreign_zapret_processes(state).map_err(|error| {
        logged_start_failure(
            app,
            state,
            LogSource::Zapret,
            "process conflict check",
            error,
        )
    })?;
    let process_name = preset.engine.process_name();
    let process_before = system_process::image_pids(process_name);

    logging::push(
        app,
        state,
        LogSource::Zapret,
        format!(
            "Launch requested: engine={}, preset={}, preset_id={}",
            preset.engine.display_name(),
            preset.relative_path,
            preset.id,
        ),
    );

    let (mut command, launch_summary) = match preset.engine {
        ZapretEngine::Classic => {
            let launch_path = managed_zapret_script(&preset).map_err(|error| {
                logged_start_failure(
                    app,
                    state,
                    LogSource::Zapret,
                    "Classic script preparation",
                    error,
                )
            })?;
            let launch_dir = Path::new(&preset.path)
                .parent()
                .unwrap_or_else(|| Path::new("."));
            let mut command = Command::new("cmd");
            command
                .arg("/D")
                .arg("/Q")
                .arg("/C")
                .arg(&launch_path)
                .current_dir(launch_dir);
            let summary = format!(
                "launcher=cmd.exe, script={}, working_dir={}, target={}",
                launch_path.display(),
                launch_dir.display(),
                process_name
            );
            (command, summary)
        }
        ZapretEngine::Zapret2 => {
            let root = paths::resources_zapret2_dir();
            validate_zapret2_resources(&root).map_err(|error| {
                logged_start_failure(
                    app,
                    state,
                    LogSource::Zapret,
                    "Zapret 2 resource validation",
                    error,
                )
            })?;
            let executable = root.join("exe").join("winws2.exe");
            if !executable.exists() {
                return Err(logged_start_failure(
                    app,
                    state,
                    LogSource::Zapret,
                    "Zapret 2 executable validation",
                    format!("winws2.exe not found: {}", executable.display()),
                ));
            }
            let arguments = zapret2_config_arguments(Path::new(&preset.path)).map_err(|error| {
                logged_start_failure(
                    app,
                    state,
                    LogSource::Zapret,
                    "Zapret 2 config parsing",
                    error,
                )
            })?;
            let summary = format!(
                "executable={}, working_dir={}, option_count={}, options={}",
                executable.display(),
                root.display(),
                arguments.len(),
                arguments.join(" ")
            );
            let mut command = Command::new(executable);
            command.args(arguments).current_dir(root);
            (command, summary)
        }
    };
    logging::push(
        app,
        state,
        LogSource::Zapret,
        format!("Launch preflight passed: {launch_summary}"),
    );
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .creation_flags(0x08000000);

    let mut child = command.spawn().map_err(|error| {
        logged_start_failure(
            app,
            state,
            LogSource::Zapret,
            "process creation",
            describe_process_error(process_name, &error),
        )
    })?;
    let pid = child.id();
    logging::push(
        app,
        state,
        LogSource::Zapret,
        format!("Launcher process created: PID {pid}"),
    );
    let mut owned_winws = if preset.engine == ZapretEngine::Zapret2 {
        vec![pid]
    } else {
        wait_for_owned_process(process_name, &process_before)
    };
    if let Some(status) = child.try_wait().map_err(|error| {
        logged_start_failure(app, state, LogSource::Zapret, "startup status check", error)
    })? {
        if preset.engine == ZapretEngine::Zapret2 || owned_winws.is_empty() {
            let details = read_child_output(&mut child);
            let message = format!(
                "{} exited during startup ({status}){}",
                process_name,
                if details.is_empty() {
                    String::new()
                } else {
                    format!(": {details}")
                }
            );
            return Err(logged_start_failure(
                app,
                state,
                LogSource::Zapret,
                "early process exit",
                message,
            ));
        }
    }
    if preset.engine == ZapretEngine::Classic && owned_winws.is_empty() {
        logging::push(
            app,
            state,
            LogSource::Zapret,
            "Classic launcher is alive, but winws.exe was not detected within 2 seconds; continuing to monitor the launcher output",
        );
    }
    if let Some(stdout) = child.stdout.take() {
        spawn_pipe_logger(app.clone(), LogSource::Zapret, stdout);
    }
    if let Some(stderr) = child.stderr.take() {
        spawn_pipe_logger(app.clone(), LogSource::Zapret, stderr);
    }
    owned_winws.sort_unstable();
    owned_winws.dedup();
    logging::push(
        app,
        state,
        LogSource::Zapret,
        format!(
            "Launch confirmed: launcher_pid={pid}, owned_{}_pids={}",
            process_name,
            if owned_winws.is_empty() {
                "none".to_string()
            } else {
                owned_winws
                    .iter()
                    .map(u32::to_string)
                    .collect::<Vec<_>>()
                    .join(",")
            }
        ),
    );

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
        runtime.active_zapret_engine = Some(preset.engine);
        write_owned_winws_marker(preset.engine, &runtime.zapret_winws_pids);
        runtime.app_state.zapret = status.clone();
        runtime.app_state.status = AppStatus::On;
        runtime.app_state.last_error = None;
    }
    emit_state(app, state);
    Ok(status)
}

pub fn stop_zapret(app: &AppHandle, state: &Mutex<RuntimeState>) -> Result<ServiceStatus, String> {
    let engine = state
        .lock()
        .unwrap()
        .active_zapret_engine
        .unwrap_or(ZapretEngine::Classic);
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
        logging::push(
            app,
            state,
            LogSource::Zapret,
            format!(
                "Stopping owned {} PID {}",
                engine.process_name(),
                owned_winws
                    .iter()
                    .map(u32::to_string)
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        );
        for pid in &owned_winws {
            let _ = system_process::kill_pid(*pid);
        }
        let still_running = system_process::running_pids(&owned_winws);
        if !still_running.is_empty() {
            return Err(format!(
                "Failed to stop owned {} PID {}. Run ZUI as administrator and try again.",
                engine.process_name(),
                still_running
                    .iter()
                    .map(u32::to_string)
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
    }
    state.lock().unwrap().zapret_winws_pids.clear();
    state.lock().unwrap().active_zapret_engine = None;
    clear_owned_winws_marker();

    refresh_status(app, state);
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
    paths::data_dir()
        .join("runtime")
        .join("zapret-owned-pids.txt")
}

fn write_owned_winws_marker(engine: ZapretEngine, pids: &[u32]) {
    if pids.is_empty() {
        clear_owned_winws_marker();
        return;
    }
    let path = owned_winws_marker_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let mut lines = vec![match engine {
        ZapretEngine::Classic => "classic".to_string(),
        ZapretEngine::Zapret2 => "zapret2".to_string(),
    }];
    lines.extend(pids.iter().map(u32::to_string));
    let text = lines.join("\n");
    let _ = std::fs::write(path, text);
}

fn load_owned_winws_marker() -> (ZapretEngine, Vec<u32>) {
    let path = owned_winws_marker_path();
    let Ok(text) = std::fs::read_to_string(path) else {
        return (ZapretEngine::Classic, Vec::new());
    };
    let mut lines = text.lines();
    let first = lines.next().unwrap_or_default().trim();
    let (engine, pid_lines): (ZapretEngine, Box<dyn Iterator<Item = &str>>) = match first {
        "zapret2" => (ZapretEngine::Zapret2, Box::new(lines)),
        "classic" => (ZapretEngine::Classic, Box::new(lines)),
        _ => (
            ZapretEngine::Classic,
            Box::new(std::iter::once(first).chain(lines)),
        ),
    };
    let pids = pid_lines
        .filter_map(|line| line.trim().parse::<u32>().ok())
        .filter(|pid| system_process::is_pid_running(*pid))
        .collect();
    (engine, pids)
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

fn zapret2_config_arguments(path: &Path) -> Result<Vec<String>, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|error| format!("Failed to read Zapret 2 preset {}: {error}", path.display()))?;
    let arguments = text
        .trim_start_matches('\u{feff}')
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();
    if arguments.is_empty() {
        return Err(format!(
            "Zapret 2 preset contains no options: {}",
            path.display()
        ));
    }
    Ok(arguments)
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

fn wait_for_owned_process(process_name: &str, before: &[u32]) -> Vec<u32> {
    let before: std::collections::HashSet<u32> = before.iter().copied().collect();
    for _ in 0..20 {
        let current = system_process::image_pids(process_name);
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

fn logged_start_failure(
    app: &AppHandle,
    state: &Mutex<RuntimeState>,
    source: LogSource,
    stage: &str,
    error: impl std::fmt::Display,
) -> String {
    let message = format!("Startup failed during {stage}: {error}");
    logging::push(app, state, source, &message);
    message
}

fn describe_process_error(process_name: &str, error: &std::io::Error) -> String {
    let hint = match error.kind() {
        std::io::ErrorKind::NotFound => "the executable or one of its required paths was not found",
        std::io::ErrorKind::PermissionDenied => {
            "access was denied; run ZUI as administrator and check antivirus protection"
        }
        _ if error.raw_os_error() == Some(740) => {
            "Windows requires elevation; run ZUI as administrator"
        }
        _ if error.raw_os_error() == Some(193) => {
            "the executable is not a valid Windows binary for this system"
        }
        _ => "Windows could not create the process",
    };
    format!(
        "could not start {process_name}: {error}; os_code={}; hint={hint}",
        error
            .raw_os_error()
            .map(|code| code.to_string())
            .unwrap_or_else(|| "none".to_string())
    )
}

fn read_child_output(child: &mut Child) -> String {
    let mut parts = Vec::new();
    if let Some(mut stderr) = child.stderr.take() {
        let mut text = String::new();
        if stderr.read_to_string(&mut text).is_ok() && !text.trim().is_empty() {
            parts.push(format!("stderr={}", text.trim()));
        }
    }
    if let Some(mut stdout) = child.stdout.take() {
        let mut text = String::new();
        if stdout.read_to_string(&mut text).is_ok() && !text.trim().is_empty() {
            parts.push(format!("stdout={}", text.trim()));
        }
    }
    parts.join("; ")
}

fn ensure_no_foreign_zapret_processes(state: &Mutex<RuntimeState>) -> Result<(), String> {
    let known = state.lock().unwrap().zapret_winws_pids.clone();
    let processes = system_process::running_processes_by_names(&["winws.exe", "winws2.exe"]);
    for process_name in ["winws.exe", "winws2.exe"] {
        let foreign: Vec<u32> = processes
            .iter()
            .filter(|process| process.image.eq_ignore_ascii_case(process_name))
            .map(|process| process.pid)
            .filter(|pid| !known.contains(pid))
            .collect();
        if !foreign.is_empty() {
            return Err(format!(
                "Foreign {} is already running: PID {}. Stop it before starting ZUI.",
                process_name,
                foreign
                    .iter()
                    .map(u32::to_string)
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
    }
    Ok(())
}

const ZAPRET2_ENGINE_VERSION: &str = "1.0.5";

fn validate_zapret2_resources(root: &Path) -> Result<(), String> {
    for relative in [
        "manifest.json",
        "exe/winws2.exe",
        "exe/cygwin1.dll",
        "exe/WinDivert.dll",
        "exe/WinDivert64.sys",
        "lua/zapret-lib.lua",
        "lua/zapret-antidpi.lua",
    ] {
        let path = root.join(relative);
        if !path.exists() {
            return Err(format!("Zapret 2 resource is missing: {}", path.display()));
        }
    }

    let library = std::fs::read_to_string(root.join("lua").join("zapret-lib.lua"))
        .map_err(|error| error.to_string())?;
    if !library.contains("NFQWS2_COMPAT_VER_REQUIRED=6") {
        return Err(format!(
            "Zapret 2 Lua runtime is incompatible with engine v{ZAPRET2_ENGINE_VERSION}"
        ));
    }

    let manifest: serde_json::Value = serde_json::from_slice(
        &std::fs::read(root.join("manifest.json")).map_err(|error| error.to_string())?,
    )
    .map_err(|error| format!("invalid Zapret 2 manifest: {error}"))?;
    if manifest
        .get("engineVersion")
        .and_then(|value| value.as_str())
        != Some(ZAPRET2_ENGINE_VERSION)
    {
        return Err(format!(
            "Zapret 2 manifest does not describe engine v{ZAPRET2_ENGINE_VERSION}"
        ));
    }
    Ok(())
}

pub fn start_tg_ws(
    app: &AppHandle,
    state: &Mutex<RuntimeState>,
    host: String,
    port: u16,
    secret: String,
) -> Result<ServiceStatus, String> {
    refresh_status(app, state);
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

    if let Err(error) = TcpListener::bind((&host[..], port)) {
        return Err(logged_start_failure(
            app,
            state,
            LogSource::TgWs,
            "listener preflight",
            format!(
                "cannot bind {host}:{port}: {error}; os_code={}; another process may be using the port or the address is not available",
                error
                    .raw_os_error()
                    .map(|code| code.to_string())
                    .unwrap_or_else(|| "none".to_string())
            ),
        ));
    }

    let mut profile = profiles::active_profile().map_err(|error| {
        logged_start_failure(app, state, LogSource::TgWs, "profile loading", error)
    })?;
    profile.tg_ws_host = host;
    profile.tg_ws_port = port;
    profile.tg_ws_secret = secret;
    logging::set_tg_ws_log_max_mb(profile.tg_ws_log_max_mb);
    let config_summary = runtime::tg_ws::configuration_summary(&profile).map_err(|error| {
        logged_start_failure(app, state, LogSource::TgWs, "configuration", error)
    })?;
    logging::push(
        app,
        state,
        LogSource::TgWs,
        format!(
            "Starting {} {}: {config_summary}; secret=configured-but-redacted",
            runtime::tg_ws::ENGINE_NAME,
            runtime::tg_ws::ENGINE_VERSION,
        ),
    );
    let handle = runtime::tg_ws::spawn(&profile).map_err(|error| {
        logged_start_failure(app, state, LogSource::TgWs, "embedded runtime", error)
    })?;
    logging::push(
        app,
        state,
        LogSource::TgWs,
        format!(
            "Listener ready on {}:{}; Telegram link generated (secret redacted)",
            handle.host, handle.port
        ),
    );

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
        if let Err(error) = runtime_handle.stop() {
            logging::push(
                app,
                state,
                LogSource::TgWs,
                format!("Shutdown failed: {error}"),
            );
            return Err(error);
        }
        logging::push(app, state, LogSource::TgWs, "Shutdown completed");
    }

    refresh_status(app, state);
    let status = state.lock().unwrap().app_state.tg_ws.clone();
    emit_state(app, state);
    Ok(status)
}

pub fn refresh_status(app: &AppHandle, state: &Mutex<RuntimeState>) {
    let (tracked_pids, zapret_child_alive, zapret_exit, tg_ws_running, tg_ws_failure) = {
        let mut runtime = state.lock().unwrap();
        let (child_alive, child_exit) = match runtime.zapret_child.as_mut() {
            Some(child) => match child.try_wait() {
                Ok(None) => (true, None),
                Ok(Some(status)) => (false, Some(format!("process exited with {status}"))),
                Err(error) => (
                    false,
                    Some(format!("could not query process status: {error}")),
                ),
            },
            None => (false, None),
        };
        let tg_running = runtime
            .tg_ws_runtime
            .as_ref()
            .map(|handle| handle.is_running())
            .unwrap_or(false);
        let tg_failure = runtime
            .tg_ws_runtime
            .as_ref()
            .filter(|_| !tg_running)
            .and_then(|handle| handle.failure())
            .or_else(|| {
                runtime
                    .tg_ws_runtime
                    .as_ref()
                    .filter(|_| !tg_running)
                    .map(|_| "tg-ws runtime stopped unexpectedly".to_string())
            });
        (
            runtime.zapret_winws_pids.clone(),
            child_alive,
            child_exit,
            tg_running,
            tg_failure,
        )
    };

    // tasklist can take hundreds of milliseconds on a busy Windows system.
    // Run it without holding RuntimeState so UI/status commands remain responsive.
    let running_pids: std::collections::HashSet<u32> = system_process::running_pids(&tracked_pids)
        .into_iter()
        .collect();

    let mut runtime = state.lock().unwrap();
    let previous_zapret_error = runtime.app_state.zapret.error.clone();
    let previous_tg_ws_error = runtime.app_state.tg_ws.error.clone();
    runtime
        .zapret_winws_pids
        .retain(|pid| !tracked_pids.contains(pid) || running_pids.contains(pid));
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
                runtime
                    .active_zapret_engine
                    .map(|engine| format!("{} preset process", engine.display_name()))
                    .unwrap_or_else(|| "preset process".into())
            } else {
                format!(
                    "owned {}",
                    runtime
                        .active_zapret_engine
                        .unwrap_or(ZapretEngine::Classic)
                        .process_name()
                )
            }),
            error: None,
        }
    } else if let Some(error) = zapret_exit {
        runtime.active_zapret_engine = None;
        ServiceStatus {
            service: ServiceName::Zapret,
            state: ServiceState::Error,
            pid: None,
            message: None,
            error: Some(format!("zapret stopped unexpectedly: {error}")),
        }
    } else {
        runtime.active_zapret_engine = None;
        ServiceStatus::stopped(ServiceName::Zapret)
    };

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
    } else if let Some(error) = tg_ws_failure {
        ServiceStatus {
            service: ServiceName::TgWs,
            state: ServiceState::Error,
            pid: None,
            message: None,
            error: Some(error),
        }
    } else {
        ServiceStatus::stopped(ServiceName::TgWs)
    };

    runtime.app_state.status = if zapret_child_alive || winws_running || tg_ws_running {
        AppStatus::On
    } else if matches!(runtime.app_state.zapret.state, ServiceState::Error)
        || matches!(runtime.app_state.tg_ws.state, ServiceState::Error)
    {
        AppStatus::Error
    } else {
        AppStatus::Off
    };
    runtime.app_state.last_error = runtime
        .app_state
        .zapret
        .error
        .clone()
        .or_else(|| runtime.app_state.tg_ws.error.clone());
    let zapret_error = runtime.app_state.zapret.error.clone();
    let tg_ws_error = runtime.app_state.tg_ws.error.clone();
    drop(runtime);

    if zapret_error != previous_zapret_error {
        if let Some(error) = zapret_error {
            logging::push(app, state, LogSource::Zapret, error);
        }
    }
    if tg_ws_error != previous_tg_ws_error {
        if let Some(error) = tg_ws_error {
            logging::push(app, state, LogSource::TgWs, error);
        }
    }
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
    let source = match service {
        Some(ServiceName::Zapret) => LogSource::Zapret,
        Some(ServiceName::TgWs) => LogSource::TgWs,
        None => LogSource::App,
    };
    logging::push(app, state, source, format!("Operation failed: {message}"));
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

#[cfg(test)]
mod tests {
    use super::{should_start_zapret, validate_zapret2_resources, zapret2_config_arguments};
    use crate::models::ZapretEngine;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn parses_zapret2_config_without_shell_splitting() {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("zui preset with spaces {stamp}"));
        std::fs::create_dir_all(&dir).expect("temp preset directory");
        let path = dir.join("Default old.txt");
        std::fs::write(
            &path,
            "# comment\n\n--lua-init=@lua/zapret-lib.lua\n--hostlist-domains=example.com\n",
        )
        .expect("temp preset");

        let arguments = zapret2_config_arguments(&path).expect("parsed arguments");
        assert_eq!(
            arguments,
            vec![
                "--lua-init=@lua/zapret-lib.lua",
                "--hostlist-domains=example.com"
            ]
        );

        let _ = std::fs::remove_file(path);
        let _ = std::fs::remove_dir(dir);
    }

    #[test]
    fn allows_tg_ws_only_when_zapret_is_not_configured() {
        assert_eq!(should_start_zapret(true, None, None, true), Ok(false));
        assert_eq!(
            should_start_zapret(true, Some(ZapretEngine::Classic), None, true),
            Ok(false)
        );
    }

    #[test]
    fn rejects_unconfigured_zapret_without_tg_ws() {
        assert_eq!(
            should_start_zapret(true, None, None, false),
            Err("No zapret engine selected".into())
        );
        assert_eq!(
            should_start_zapret(true, Some(ZapretEngine::Classic), None, false),
            Err("No zapret preset selected".into())
        );
    }

    #[test]
    fn starts_configured_zapret_with_or_without_tg_ws() {
        assert_eq!(
            should_start_zapret(
                true,
                Some(ZapretEngine::Zapret2),
                Some("zapret2/preset"),
                false
            ),
            Ok(true)
        );
        assert_eq!(
            should_start_zapret(
                true,
                Some(ZapretEngine::Classic),
                Some("classic/preset"),
                true
            ),
            Ok(true)
        );
    }

    #[test]
    fn bundled_zapret2_resources_match_the_supported_engine() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("resources")
            .join("zapret2");
        validate_zapret2_resources(&root).expect("bundled Zapret 2 resources");
    }
}
