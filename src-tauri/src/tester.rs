use crate::models::{
    BaselineSnapshot, FailureStage, LogSource, ProbeCapability, ProbeChange, ProbeContext,
    ProbeStatus, ProbeStepResult, TestMode, TestPhase, TestProgress, TestResult, TestTargetConfig,
    TestTargetResult,
};
use crate::state::RuntimeState;
use crate::{json_storage, logging, paths, presets, services, settings, system_process};
use std::collections::HashMap;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::Semaphore;

#[path = "tester/baseline.rs"]
mod baseline;
#[path = "tester/dns.rs"]
mod dns;
#[path = "tester/lifecycle.rs"]
mod lifecycle;
#[path = "tester/manifest.rs"]
mod manifest;
#[path = "tester/protocol.rs"]
mod protocol;
#[path = "tester/scoring.rs"]
mod scoring;
#[path = "tester/service_probes.rs"]
mod service_probes;

#[derive(Clone)]
struct Target {
    id: String,
    name: String,
    service: String,
    value: String,
    probe_kind: manifest::ProbeKind,
    capability: ProbeCapability,
    weight: u16,
    required: bool,
    diagnostic: bool,
    allowed_statuses: Vec<u16>,
    content_types: Vec<String>,
    markers: Vec<String>,
}
const MAX_PARALLEL_TARGET_CHECKS: usize = 8;
const MAX_STORED_RESULTS: usize = 100;
const TEST_ABORT_PROCESS_EXITED: &str = "processExited";

pub fn load_results() -> Result<Vec<TestResult>, String> {
    paths::ensure_data_layout().map_err(|error| error.to_string())?;
    let path = paths::test_results_path();
    if !path.exists() {
        save_results(&[])?;
        return Ok(Vec::new());
    }
    let text = std::fs::read_to_string(&path).map_err(|error| error.to_string())?;
    match json_storage::parse::<Vec<TestResult>>(&text) {
        Ok(mut results) => {
            let was_migrated = results.iter_mut().any(migrate_result);
            let was_trimmed = trim_results(&mut results);
            if text.starts_with('\u{feff}') || was_trimmed || was_migrated {
                save_results(&results)?;
            }
            Ok(results)
        }
        Err(error) => {
            let backup = json_storage::backup_invalid(&path)?;
            save_results(&[])?;
            eprintln!(
                "Stored test results were reset after a JSON error ({error}); backup: {}",
                backup.display()
            );
            Ok(Vec::new())
        }
    }
}

pub fn save_results(results: &[TestResult]) -> Result<(), String> {
    paths::ensure_data_layout().map_err(|error| error.to_string())?;
    let text = serde_json::to_string_pretty(results).map_err(|error| error.to_string())?;
    json_storage::write_atomic(&paths::test_results_path(), text.as_bytes())
}

pub fn export_results(path: String) -> Result<(), String> {
    let results = load_results()?;
    let text = serde_json::to_string_pretty(&results).map_err(|error| error.to_string())?;
    json_storage::write_atomic(std::path::Path::new(&path), text.as_bytes())
}

pub fn import_results(path: String) -> Result<Vec<TestResult>, String> {
    let text = std::fs::read_to_string(path).map_err(|error| error.to_string())?;
    let imported = json_storage::parse::<Vec<TestResult>>(&text)
        .map_err(|error| format!("Invalid ZUI test results file: {error}"))?;
    let mut results = load_results()?;

    for mut result in imported {
        if result.id.trim().is_empty()
            || result.preset_id.trim().is_empty()
            || result.finished_at.trim().is_empty()
        {
            return Err("Invalid ZUI test result: required fields are missing".into());
        }
        migrate_result(&mut result);
        if let Some(existing) = results.iter_mut().find(|existing| existing.id == result.id) {
            *existing = result;
        } else {
            results.push(result);
        }
    }

    results.sort_by(|left, right| {
        left.finished_at
            .parse::<u64>()
            .unwrap_or_default()
            .cmp(&right.finished_at.parse::<u64>().unwrap_or_default())
    });
    trim_results(&mut results);
    save_results(&results)?;
    Ok(results)
}

fn migrate_result(result: &mut TestResult) -> bool {
    if result.schema_version >= 3 {
        return false;
    }
    if result.schema_version < 2 {
        let mut targets = Vec::new();
        for service in &mut result.services {
            for target in &mut service.targets {
                target.probe_status = if target.ok {
                    ProbeStatus::Passed
                } else {
                    ProbeStatus::Failed
                };
                target.context = ProbeContext::Preset;
                target.change = ProbeChange::NotCompared;
                target.failure_stage = (!target.ok).then_some(FailureStage::Http);
                target.reason_code = (!target.ok).then(|| "legacy_failure".into());
                target.diagnostic = target.label.contains("Ping");
                target.weight = if target.diagnostic { 0 } else { 1 };
                target.required = false;
                targets.push(target.clone());
            }
        }
        result.services = scoring::build_services(&targets);
    }
    let summary = scoring::summarize(&result.services, true);
    result.schema_version = 3;
    result.score = summary.score;
    result.recommendation = summary.recommendation;
    result.ok = summary.ok;
    result.total = summary.total;
    result.passed_weight = summary.passed_weight;
    result.total_weight = summary.total_weight;
    result.inconclusive = summary.inconclusive;
    result.regressions = summary.regressions;
    result.process_ok = true;
    true
}

pub fn run_quick_test(app: AppHandle, preset_id: String) -> Result<String, String> {
    let preset = presets::find_preset(&preset_id)?;
    let runtime_state = app.state::<Mutex<RuntimeState>>();
    {
        let mut runtime = runtime_state.lock().unwrap();
        if runtime.test_running {
            return Err("Preset test is already running".into());
        }
        if runtime.zapret_child.is_some()
            || system_process::is_running("winws.exe")
            || system_process::is_running("winws2.exe")
        {
            return Err("Stop zapret before running a preset test".into());
        }
        runtime.test_running = true;
        runtime.test_cancelled = false;
    }

    let test_id = format!("test-{}", unix_timestamp());
    let thread_app = app.clone();
    let thread_test_id = test_id.clone();

    std::thread::spawn(move || {
        let panic_app = thread_app.clone();
        let outcome = catch_unwind(AssertUnwindSafe(move || {
            let runtime_state = thread_app.state::<Mutex<RuntimeState>>();
            let _ = thread_app.emit("test_started", &thread_test_id);
            let targets = test_targets();
            let baseline = collect_baseline(
                &thread_app,
                &runtime_state,
                &thread_test_id,
                &preset,
                1,
                1,
                targets.clone(),
            );
            if is_cancelled(&runtime_state) {
                state_reset(&runtime_state);
                let _ = thread_app.emit("test_cancelled", "cancelled");
                logging::push(
                    &thread_app,
                    &runtime_state,
                    LogSource::Tests,
                    "Quick test cancelled during network baseline",
                );
                return;
            }
            let result = run_one_preset(
                &thread_app,
                &runtime_state,
                thread_test_id.clone(),
                preset,
                TestMode::Selected,
                1,
                1,
                targets,
                Some(baseline),
            );
            let cancelled = is_cancelled(&runtime_state);

            {
                let mut runtime = runtime_state.lock().unwrap();
                runtime.test_running = false;
                runtime.test_cancelled = false;
                if !cancelled {
                    runtime.test_results.push(result.clone());
                    trim_results(&mut runtime.test_results);
                    let _ = save_results(&runtime.test_results);
                }
            }

            if cancelled {
                let _ = thread_app.emit("test_cancelled", "cancelled");
            } else if let Some(reason) = test_abort_reason(&result) {
                let _ = thread_app.emit("test_finished", &result);
                let _ = thread_app.emit("test_aborted", reason);
                logging::push(
                    &thread_app,
                    &runtime_state,
                    LogSource::Tests,
                    "Quick test aborted because the zapret process exited outside ZUI",
                );
            } else {
                let _ = thread_app.emit("test_finished", &result);
            }
            logging::push(
                &thread_app,
                &runtime_state,
                LogSource::Tests,
                format!("Quick test finished: {}/{}", result.ok, result.total),
            );
        }));
        if outcome.is_err() {
            let runtime_state = panic_app.state::<Mutex<RuntimeState>>();
            state_reset(&runtime_state);
            let _ = panic_app.emit("test_cancelled", "failed");
            logging::push(
                &panic_app,
                &runtime_state,
                LogSource::Tests,
                "Preset test failed unexpectedly",
            );
        }
    });

    Ok(test_id)
}

pub fn run_best_preset_test(
    app: AppHandle,
    preset_ids: Vec<String>,
    max_count: usize,
) -> Result<String, String> {
    run_batch_preset_test(app, preset_ids, max_count, TestMode::Selected)
}

pub fn run_all_preset_test(app: AppHandle, preset_ids: Vec<String>) -> Result<String, String> {
    run_batch_preset_test(app, preset_ids, 500, TestMode::All)
}

pub fn run_selected_preset_test(app: AppHandle, preset_ids: Vec<String>) -> Result<String, String> {
    run_batch_preset_test(app, preset_ids, 500, TestMode::Selected)
}

fn run_batch_preset_test(
    app: AppHandle,
    preset_ids: Vec<String>,
    max_count: usize,
    mode: TestMode,
) -> Result<String, String> {
    let runtime_state = app.state::<Mutex<RuntimeState>>();
    {
        let mut runtime = runtime_state.lock().unwrap();
        if runtime.test_running {
            return Err("Preset test is already running".into());
        }
        if runtime.zapret_child.is_some()
            || system_process::is_running("winws.exe")
            || system_process::is_running("winws2.exe")
        {
            return Err("Stop zapret before running a preset test".into());
        }
        runtime.test_running = true;
        runtime.test_cancelled = false;
    }

    let available = presets::discover_presets()?;
    let available_by_id: HashMap<String, crate::models::Preset> = available
        .into_iter()
        .map(|preset| (preset.id.clone(), preset))
        .collect();
    let mut selected = Vec::new();
    for preset_id in preset_ids.into_iter().take(max_count.clamp(1, 500)) {
        if let Some(preset) = available_by_id.get(&preset_id) {
            selected.push(preset.clone());
        }
    }
    if selected.is_empty() {
        state_reset(&runtime_state);
        return Err("No presets selected for testing".into());
    }

    let batch_id = format!("batch-{}", unix_timestamp());
    let thread_app = app.clone();
    let thread_batch_id = batch_id.clone();
    let mode_label = test_mode_label(&mode).to_string();

    std::thread::spawn(move || {
        let panic_app = thread_app.clone();
        let outcome = catch_unwind(AssertUnwindSafe(move || {
            let runtime_state = thread_app.state::<Mutex<RuntimeState>>();
            let _ = thread_app.emit("test_started", &thread_batch_id);
            logging::push(
                &thread_app,
                &runtime_state,
                LogSource::Tests,
                format!("{mode_label} started: {} presets", selected.len()),
            );

            let preset_count = selected.len();
            let targets = test_targets();
            let mut baseline = collect_baseline(
                &thread_app,
                &runtime_state,
                &thread_batch_id,
                &selected[0],
                1,
                preset_count,
                targets.clone(),
            );

            let mut batch_results = Vec::new();
            let mut aborted_reason = None;
            for (preset_index, preset) in selected.into_iter().enumerate() {
                if is_cancelled(&runtime_state) {
                    break;
                }
                if baseline_needs_refresh(&baseline) {
                    logging::push(
                        &thread_app,
                        &runtime_state,
                        LogSource::Tests,
                        "Network baseline expired or the adapter configuration changed; refreshing",
                    );
                    baseline = collect_baseline(
                        &thread_app,
                        &runtime_state,
                        &thread_batch_id,
                        &preset,
                        preset_index + 1,
                        preset_count,
                        targets.clone(),
                    );
                }
                let result = run_one_preset(
                    &thread_app,
                    &runtime_state,
                    format!("{}-{}", thread_batch_id, batch_results.len() + 1),
                    preset,
                    mode.clone(),
                    preset_index + 1,
                    preset_count,
                    targets.clone(),
                    Some(baseline.clone()),
                );
                if is_cancelled(&runtime_state) {
                    break;
                }
                let _ = thread_app.emit("test_preset_finished", &result);
                {
                    let mut runtime = runtime_state.lock().unwrap();
                    runtime.test_results.push(result.clone());
                    trim_results(&mut runtime.test_results);
                    let _ = save_results(&runtime.test_results);
                }
                batch_results.push(result);
                if let Some(reason) = batch_results.last().and_then(test_abort_reason) {
                    aborted_reason = Some(reason);
                    break;
                }
            }

            batch_results.sort_by(|left, right| {
                right
                    .score
                    .cmp(&left.score)
                    .then_with(|| right.ok.cmp(&left.ok))
                    .then_with(|| left.preset_name.cmp(&right.preset_name))
            });

            let cancelled = is_cancelled(&runtime_state);
            state_reset(&runtime_state);
            if let Some(reason) = aborted_reason {
                let _ = thread_app.emit("test_aborted", reason);
                logging::push(
                    &thread_app,
                    &runtime_state,
                    LogSource::Tests,
                    format!("{mode_label} aborted because the zapret process exited outside ZUI"),
                );
            } else if cancelled {
                let _ = thread_app.emit("test_cancelled", "cancelled");
                logging::push(
                    &thread_app,
                    &runtime_state,
                    LogSource::Tests,
                    format!("{mode_label} cancelled"),
                );
            } else {
                let _ = thread_app.emit("test_batch_finished", &batch_results);
                logging::push(
                    &thread_app,
                    &runtime_state,
                    LogSource::Tests,
                    format!("{mode_label} finished"),
                );
            }
        }));
        if outcome.is_err() {
            let runtime_state = panic_app.state::<Mutex<RuntimeState>>();
            state_reset(&runtime_state);
            let _ = panic_app.emit("test_cancelled", "failed");
            logging::push(
                &panic_app,
                &runtime_state,
                LogSource::Tests,
                "Preset batch test failed unexpectedly",
            );
        }
    });

    Ok(batch_id)
}

pub fn cancel_with_app(app: &AppHandle, state: &Mutex<RuntimeState>) {
    state.lock().unwrap().test_cancelled = true;
    let _ = app.emit("test_stopping", "stopping");
}

fn baseline_needs_refresh(snapshot: &BaselineSnapshot) -> bool {
    let captured = snapshot.captured_at.parse::<u64>().unwrap_or_default();
    let now = unix_timestamp().parse::<u64>().unwrap_or_default();
    now.saturating_sub(captured) >= 300
        || baseline::network_fingerprint() != snapshot.network_fingerprint
}

#[allow(clippy::too_many_arguments)]
fn run_one_preset(
    app: &AppHandle,
    state: &Mutex<RuntimeState>,
    test_id: String,
    preset: crate::models::Preset,
    mode: TestMode,
    preset_index: usize,
    preset_count: usize,
    targets: Vec<Target>,
    baseline: Option<BaselineSnapshot>,
) -> TestResult {
    let started_at = unix_timestamp();
    let total_checks = planned_check_count(&targets);
    let mut progress = TestProgress {
        test_id: test_id.clone(),
        preset_id: preset.id.clone(),
        preset_name: preset.name.clone(),
        engine: preset.engine,
        preset_index,
        preset_count,
        total_checks,
        completed_checks: 0,
        passed_checks: 0,
        failed_checks: 0,
        inconclusive_checks: 0,
        phase: TestPhase::Starting,
        current_target: None,
    };
    let _ = app.emit("test_preset_started", &preset);
    emit_progress(app, &progress);
    logging::push(
        app,
        state,
        LogSource::Tests,
        format!(
            "{} started: {}",
            test_mode_label(&mode),
            preset.relative_path
        ),
    );

    let mut target_results = Vec::new();
    let mut process_ok = false;
    let start_result = services::start_zapret(app, state, preset.id.clone());
    if let Err(error) = start_result {
        logging::push(app, state, LogSource::Tests, error.clone());
        target_results.push(startup_failure_target(&preset.relative_path, error));
        progress.completed_checks = total_checks;
        progress.failed_checks = total_checks;
        progress.phase = TestPhase::Finishing;
        emit_progress(app, &progress);
    } else {
        progress.phase = TestPhase::Warmup;
        emit_progress(app, &progress);
        match lifecycle::wait_until_ready(app, state) {
            Ok(pids) => {
                process_ok = true;
                logging::push(
                    app,
                    state,
                    LogSource::Tests,
                    format!(
                        "zapret readiness confirmed: PID {}",
                        pids.iter()
                            .map(u32::to_string)
                            .collect::<Vec<_>>()
                            .join(",")
                    ),
                );
                if !is_cancelled(state) {
                    progress.phase = TestPhase::Checking;
                    emit_progress(app, &progress);
                    target_results = run_targets(
                        app,
                        state,
                        targets,
                        &mut progress,
                        ProbeContext::Preset,
                        true,
                    );
                    process_ok = lifecycle::is_healthy(app, state);
                    if !process_ok
                        && !target_results
                            .iter()
                            .any(|target| target.failure_stage == Some(FailureStage::Process))
                    {
                        let failure = runtime_process_failure_target(&preset.relative_path);
                        let _ = app.emit("test_target_finished", &failure);
                        target_results.push(failure);
                    }
                }
            }
            Err(error) => {
                logging::push(app, state, LogSource::Tests, error.clone());
                target_results.push(process_failure_target(&preset.relative_path, error));
                progress.completed_checks = total_checks;
                progress.failed_checks = total_checks;
            }
        }
        progress.phase = TestPhase::Finishing;
        progress.current_target = None;
        emit_progress(app, &progress);
        let _ = services::stop_zapret(app, state);
    }

    scoring::apply_baseline_comparison(&mut target_results, baseline.as_ref());

    let result = build_result(
        test_id,
        preset.id.clone(),
        preset.name.clone(),
        preset.relative_path.clone(),
        preset.engine,
        mode.clone(),
        started_at,
        unix_timestamp(),
        target_results,
        baseline,
        process_ok,
    );

    logging::push(
        app,
        state,
        LogSource::Tests,
        format!(
            "{} finished: {}/{}",
            test_mode_label(&mode),
            result.ok,
            result.total
        ),
    );
    result
}

fn collect_baseline(
    app: &AppHandle,
    state: &Mutex<RuntimeState>,
    test_id: &str,
    preset: &crate::models::Preset,
    preset_index: usize,
    preset_count: usize,
    targets: Vec<Target>,
) -> BaselineSnapshot {
    let total_checks = planned_check_count(&targets);
    let mut progress = TestProgress {
        test_id: test_id.to_string(),
        preset_id: preset.id.clone(),
        preset_name: "Baseline".into(),
        engine: preset.engine,
        preset_index,
        preset_count,
        total_checks,
        completed_checks: 0,
        passed_checks: 0,
        failed_checks: 0,
        inconclusive_checks: 0,
        phase: TestPhase::Baseline,
        current_target: None,
    };
    emit_progress(app, &progress);
    logging::push(app, state, LogSource::Tests, "Network baseline started");
    let results = run_targets(
        app,
        state,
        targets,
        &mut progress,
        ProbeContext::Baseline,
        false,
    );
    let snapshot = baseline::snapshot(unix_timestamp(), results);
    logging::push(
        app,
        state,
        LogSource::Tests,
        format!(
            "Network baseline finished: passed={}, failed={}, inconclusive={}, fingerprint={}",
            snapshot.passed, snapshot.failed, snapshot.inconclusive, snapshot.network_fingerprint
        ),
    );
    snapshot
}

fn run_targets(
    app: &AppHandle,
    state: &Mutex<RuntimeState>,
    targets: Vec<Target>,
    progress: &mut TestProgress,
    context: ProbeContext,
    monitor_process: bool,
) -> Vec<TestTargetResult> {
    let runtime = match tokio::runtime::Builder::new_multi_thread()
        .worker_threads(4)
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime,
        Err(error) => {
            logging::push(
                app,
                state,
                LogSource::Tests,
                format!("Async test runtime failed: {error}"),
            );
            progress.completed_checks = progress.total_checks;
            progress.failed_checks = progress.total_checks;
            progress.current_target = Some("Async test runtime".into());
            emit_progress(app, progress);
            return Vec::new();
        }
    };

    runtime.block_on(run_targets_async(
        app,
        state,
        targets,
        progress,
        context,
        monitor_process,
    ))
}

async fn run_targets_async(
    app: &AppHandle,
    state: &Mutex<RuntimeState>,
    targets: Vec<Target>,
    progress: &mut TestProgress,
    context: ProbeContext,
    monitor_process: bool,
) -> Vec<TestTargetResult> {
    let limiter = Arc::new(Semaphore::new(MAX_PARALLEL_TARGET_CHECKS));

    let mut tasks = tokio::task::JoinSet::new();
    for target in targets {
        if is_cancelled(state) {
            break;
        }
        let limiter = Arc::clone(&limiter);
        tasks.spawn(async move {
            let _permit = limiter.acquire_owned().await.ok();
            check_target(target).await
        });
    }

    let mut results = Vec::new();

    while let Some(joined) = tasks.join_next().await {
        if is_cancelled(state) {
            break;
        }

        if monitor_process && !lifecycle::is_healthy(app, state) {
            tasks.abort_all();
            let failure = runtime_process_failure_target("runtime");
            let _ = app.emit("test_target_finished", &failure);
            results.push(failure);
            progress.failed_checks = progress.failed_checks.saturating_add(1);
            progress.completed_checks = progress.total_checks;
            progress.current_target = Some("zapret process".into());
            emit_progress(app, progress);
            break;
        }

        let mut result = match joined {
            Ok(result) => result,
            Err(error) => {
                logging::push(
                    app,
                    state,
                    LogSource::Tests,
                    format!("Target check task failed: {error}"),
                );
                progress.completed_checks = progress.completed_checks.saturating_add(1);
                progress.failed_checks = progress.failed_checks.saturating_add(1);
                progress.current_target = Some("Internal check".into());
                emit_progress(app, progress);
                continue;
            }
        };
        result.context = context;
        let _ = app.emit("test_target_finished", &result);
        progress.completed_checks = progress.completed_checks.saturating_add(1);
        match result.probe_status {
            ProbeStatus::Passed => {
                progress.passed_checks = progress.passed_checks.saturating_add(1)
            }
            ProbeStatus::Failed => {
                progress.failed_checks = progress.failed_checks.saturating_add(1)
            }
            ProbeStatus::Inconclusive | ProbeStatus::Cancelled => {
                progress.inconclusive_checks = progress.inconclusive_checks.saturating_add(1)
            }
        }
        progress.current_target = Some(result.label.clone());
        emit_progress(app, progress);
        logging::push(
            app,
            state,
            LogSource::Tests,
            format!(
                "{} {} {}",
                result.service,
                match result.probe_status {
                    ProbeStatus::Passed => "passed",
                    ProbeStatus::Failed => "failed",
                    ProbeStatus::Inconclusive => "inconclusive",
                    ProbeStatus::Cancelled => "cancelled",
                },
                result.url
            ),
        );
        results.push(result);
    }

    results
}

async fn check_target(target: Target) -> TestTargetResult {
    use manifest::ProbeKind;
    match target.probe_kind {
        ProbeKind::Ping => {
            check_ping_target(target.clone(), target.value[5..].trim().to_string()).await
        }
        ProbeKind::Dns | ProbeKind::UdpDns => {
            let value = target.value.clone();
            let (resolver, host) = manifest::parse_dns_target(&value).unwrap_or(("SYSTEM", ""));
            check_dns_target(target, resolver.to_string(), host.to_string()).await
        }
        ProbeKind::DiscordGatewayWebsocket => {
            let outcome = service_probes::discord_gateway_websocket(&target.value).await;
            result_from_simple(target, outcome)
        }
        ProbeKind::WebsocketEcho => {
            let outcome = service_probes::websocket_echo(&target.value).await;
            result_from_simple(target, outcome)
        }
        ProbeKind::DiscordVoiceReadiness => {
            result_from_simple(target, service_probes::discord_voice_readiness())
        }
        ProbeKind::QuicHttp3 => {
            let value = target.value.clone();
            let outcome = service_probes::quic_http3_readiness(&value).await;
            result_from_simple(target, outcome)
        }
        ProbeKind::YoutubeMedia => {
            let request = http_request(&target, protocol::HttpMode::Auto, None);
            let outcome = service_probes::youtube_media_probe(request).await;
            result_from_http(target, outcome)
        }
        ProbeKind::Http
        | ProbeKind::Http1
        | ProbeKind::Http2
        | ProbeKind::Tls12
        | ProbeKind::Tls13
        | ProbeKind::DiscordGatewayApi
        | ProbeKind::YoutubeWeb
        | ProbeKind::ControlledJson
        | ProbeKind::ControlledRedirect
        | ProbeKind::ControlledRange => {
            let mode = match target.probe_kind {
                ProbeKind::Http1 => protocol::HttpMode::Http1,
                ProbeKind::Http2 => protocol::HttpMode::Http2,
                ProbeKind::Tls12 => protocol::HttpMode::Tls12,
                ProbeKind::Tls13 => protocol::HttpMode::Tls13,
                _ => protocol::HttpMode::Auto,
            };
            let range =
                (target.probe_kind == ProbeKind::ControlledRange).then(|| "bytes=0-32767".into());
            let mut outcome = protocol::staged_http_probe(http_request(&target, mode, range)).await;
            if target.probe_kind == ProbeKind::DiscordGatewayApi
                && outcome.probe_status == ProbeStatus::Passed
            {
                let valid_gateway = serde_json::from_slice::<serde_json::Value>(&outcome.body)
                    .ok()
                    .and_then(|json| {
                        json.get("url")
                            .and_then(serde_json::Value::as_str)
                            .map(str::to_owned)
                    })
                    .is_some_and(|url| url.starts_with("wss://gateway.discord.gg"));
                if !valid_gateway {
                    outcome.probe_status = ProbeStatus::Failed;
                    outcome.failure_stage = Some(FailureStage::Content);
                    outcome.reason_code = Some("discord_gateway_json_invalid".into());
                    outcome.error =
                        Some("Gateway response has no valid Discord WebSocket URL".into());
                    outcome.steps.push(ProbeStepResult {
                        stage: FailureStage::Content,
                        status: ProbeStatus::Failed,
                        latency_ms: None,
                        detail: "Missing valid gateway URL field".into(),
                    });
                }
            }
            result_from_http(target, outcome)
        }
    }
}

fn http_request(
    target: &Target,
    mode: protocol::HttpMode,
    range: Option<String>,
) -> protocol::HttpProbeRequest {
    let allowed_final_hosts = reqwest::Url::parse(&target.value)
        .ok()
        .and_then(|url| url.host_str().map(base_domain))
        .into_iter()
        .collect();
    protocol::HttpProbeRequest {
        url: target.value.clone(),
        allowed_statuses: target.allowed_statuses.clone(),
        content_types: target.content_types.clone(),
        markers: target.markers.clone(),
        allowed_final_hosts,
        mode,
        range,
    }
}

fn base_domain(host: &str) -> String {
    for suffix in [
        "discord.com",
        "discordapp.com",
        "youtube.com",
        "ytimg.com",
        "googlevideo.com",
        "cloudflare.com",
        "cloudflare-quic.com",
    ] {
        if host.eq_ignore_ascii_case(suffix)
            || host.to_ascii_lowercase().ends_with(&format!(".{suffix}"))
        {
            return suffix.into();
        }
    }
    host.to_string()
}

fn result_from_http(target: Target, outcome: protocol::HttpProbeOutcome) -> TestTargetResult {
    TestTargetResult {
        target_id: target.id,
        service: target.service,
        label: target.name,
        url: target.value,
        ok: outcome.probe_status == ProbeStatus::Passed,
        probe_status: outcome.probe_status,
        context: ProbeContext::Preset,
        change: ProbeChange::NotCompared,
        failure_stage: outcome.failure_stage,
        reason_code: outcome.reason_code,
        weight: target.weight,
        required: target.required,
        diagnostic: target.diagnostic,
        capability: target.capability,
        steps: outcome.steps,
        status: outcome.status,
        latency_ms: Some(outcome.latency_ms),
        final_url: outcome.final_url,
        content_type: outcome.content_type,
        negotiated_protocol: outcome.negotiated_protocol,
        tls_version: outcome.tls_version,
        alpn: outcome.alpn,
        bytes_read: Some(outcome.bytes_read),
        error: outcome.error,
    }
}

fn result_from_simple(
    target: Target,
    outcome: service_probes::SimpleProbeOutcome,
) -> TestTargetResult {
    let negotiated_protocol = match target.capability {
        ProbeCapability::Http3 if outcome.status == ProbeStatus::Passed => Some("HTTP/3".into()),
        ProbeCapability::Websocket if outcome.status == ProbeStatus::Passed => {
            Some("WebSocket".into())
        }
        _ => None,
    };
    let alpn = (target.capability == ProbeCapability::Http3
        && outcome.status == ProbeStatus::Passed)
        .then(|| "h3".into());
    TestTargetResult {
        target_id: target.id,
        service: target.service,
        label: target.name,
        url: target.value,
        ok: outcome.status == ProbeStatus::Passed,
        probe_status: outcome.status,
        context: ProbeContext::Preset,
        change: ProbeChange::NotCompared,
        failure_stage: outcome.failure_stage,
        reason_code: outcome.reason_code,
        weight: target.weight,
        required: target.required,
        diagnostic: target.diagnostic,
        capability: target.capability,
        steps: outcome.steps,
        status: None,
        latency_ms: Some(outcome.latency_ms),
        final_url: None,
        content_type: None,
        negotiated_protocol,
        tls_version: None,
        alpn,
        bytes_read: None,
        error: outcome.error,
    }
}

fn startup_failure_target(preset_path: &str, error: String) -> TestTargetResult {
    TestTargetResult {
        service: "Zapret".into(),
        label: "zapret start".into(),
        url: preset_path.into(),
        ok: false,
        probe_status: ProbeStatus::Failed,
        context: ProbeContext::Preset,
        change: ProbeChange::NotCompared,
        failure_stage: Some(FailureStage::Process),
        reason_code: Some("process_start_failed".into()),
        weight: 100,
        required: true,
        diagnostic: false,
        status: None,
        latency_ms: None,
        error: Some(error),
        ..Default::default()
    }
}

fn process_failure_target(preset_path: &str, error: String) -> TestTargetResult {
    let mut target = startup_failure_target(preset_path, error);
    target.label = "zapret readiness".into();
    target.reason_code = Some("process_not_ready".into());
    target
}

fn runtime_process_failure_target(preset_path: &str) -> TestTargetResult {
    let mut target = process_failure_target(
        preset_path,
        "zapret process exited outside ZUI during network checks".into(),
    );
    target.label = "zapret process".into();
    target.reason_code = Some("process_exited_external".into());
    target
}

async fn check_dns_target(target: Target, resolver: String, host: String) -> TestTargetResult {
    let outcome = dns::probe(&resolver, &host).await;
    let ok = outcome.status == ProbeStatus::Passed;
    let failure_stage = if target.probe_kind == manifest::ProbeKind::UdpDns {
        FailureStage::Udp
    } else {
        FailureStage::Dns
    };
    TestTargetResult {
        target_id: target.id.clone(),
        service: target.service,
        label: format!("{} DNS", target.name),
        url: format!("DNS:{resolver}:{host}"),
        ok,
        probe_status: outcome.status,
        context: ProbeContext::Preset,
        change: ProbeChange::NotCompared,
        failure_stage: (!ok).then_some(failure_stage),
        reason_code: outcome.reason_code,
        weight: target.weight,
        required: target.required,
        diagnostic: target.diagnostic,
        capability: target.capability,
        steps: vec![ProbeStepResult {
            stage: failure_stage,
            status: outcome.status,
            latency_ms: Some(outcome.latency_ms),
            detail: outcome.detail.clone(),
        }],
        status: None,
        latency_ms: Some(outcome.latency_ms),
        final_url: None,
        content_type: None,
        negotiated_protocol: None,
        tls_version: None,
        alpn: None,
        bytes_read: None,
        error: (!ok).then_some(outcome.detail),
    }
}

async fn check_ping_target(target: Target, host: String) -> TestTargetResult {
    let fallback_service = target.service.clone();
    let fallback_label = format!("{} Ping", target.name);
    let fallback_host = host.clone();
    tokio::task::spawn_blocking(move || {
        let started = Instant::now();
        let output = Command::new("ping")
            .args(["-n", "3", "-w", "1500", &host])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .creation_flags(0x08000000)
            .output();

        match output {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);
                TestTargetResult {
                    target_id: target.id.clone(),
                    service: target.service,
                    label: format!("{} Ping", target.name),
                    url: host,
                    ok: output.status.success(),
                    probe_status: if output.status.success() {
                        ProbeStatus::Passed
                    } else {
                        ProbeStatus::Failed
                    },
                    context: ProbeContext::Preset,
                    change: ProbeChange::NotCompared,
                    failure_stage: (!output.status.success()).then_some(FailureStage::Udp),
                    reason_code: (!output.status.success()).then(|| "icmp_failed".into()),
                    weight: 0,
                    required: false,
                    diagnostic: true,
                    capability: target.capability,
                    steps: vec![ProbeStepResult {
                        stage: FailureStage::Udp,
                        status: if output.status.success() {
                            ProbeStatus::Passed
                        } else {
                            ProbeStatus::Failed
                        },
                        latency_ms: parse_ping_average_ms(&stdout)
                            .or_else(|| Some(started.elapsed().as_millis())),
                        detail: if output.status.success() {
                            "ICMP reply received".into()
                        } else {
                            "ICMP request failed".into()
                        },
                    }],
                    status: None,
                    latency_ms: parse_ping_average_ms(&stdout)
                        .or_else(|| Some(started.elapsed().as_millis())),
                    error: if output.status.success() {
                        None
                    } else {
                        Some(first_non_empty_line(&stdout).unwrap_or_else(|| {
                            first_non_empty_line(&stderr).unwrap_or_else(|| "Ping timeout".into())
                        }))
                    },
                    ..Default::default()
                }
            }
            Err(error) => TestTargetResult {
                target_id: target.id.clone(),
                service: target.service,
                label: format!("{} Ping", target.name),
                url: host,
                ok: false,
                probe_status: ProbeStatus::Inconclusive,
                context: ProbeContext::Preset,
                change: ProbeChange::NotCompared,
                failure_stage: Some(FailureStage::Internal),
                reason_code: Some("ping_unavailable".into()),
                weight: 0,
                required: false,
                diagnostic: true,
                capability: target.capability,
                status: None,
                latency_ms: Some(started.elapsed().as_millis()),
                error: Some(error.to_string()),
                ..Default::default()
            },
        }
    })
    .await
    .unwrap_or_else(|error| TestTargetResult {
        service: fallback_service,
        label: fallback_label,
        url: fallback_host,
        ok: false,
        probe_status: ProbeStatus::Inconclusive,
        context: ProbeContext::Preset,
        change: ProbeChange::NotCompared,
        failure_stage: Some(FailureStage::Internal),
        reason_code: Some("ping_task_failed".into()),
        weight: 0,
        required: false,
        diagnostic: true,
        status: None,
        latency_ms: None,
        error: Some(error.to_string()),
        ..Default::default()
    })
}

fn parse_ping_average_ms(text: &str) -> Option<u128> {
    let marker = "Average =";
    let index = text.find(marker)?;
    let tail = &text[index + marker.len()..];
    let digits: String = tail
        .chars()
        .skip_while(|char| !char.is_ascii_digit())
        .take_while(|char| char.is_ascii_digit())
        .collect();
    digits.parse().ok()
}

fn first_non_empty_line(text: &str) -> Option<String> {
    text.lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .map(str::to_string)
}

fn test_targets() -> Vec<Target> {
    let mut targets = load_custom_targets().unwrap_or_else(fallback_targets);
    targets.extend(configured_probe_targets());
    targets
}

fn configured_probe_targets() -> Vec<Target> {
    let Some(base) = option_env!("ZUI_PROBE_BASE_URL")
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return Vec::new();
    };
    let Ok(url) = reqwest::Url::parse(base) else {
        return Vec::new();
    };
    if url.scheme() != "https" || url.host_str().is_none() {
        return Vec::new();
    }
    let root = base.trim_end_matches('/');
    let definitions = [
        (
            "zui-probe-json",
            "Fixed JSON",
            format!("{root}/json"),
            manifest::ProbeKind::ControlledJson,
            ProbeCapability::WebApi,
            vec![200],
            vec!["application/json".into()],
            vec!["zui-probe-v1".into()],
        ),
        (
            "zui-probe-redirect",
            "Controlled redirect",
            format!("{root}/redirect"),
            manifest::ProbeKind::ControlledRedirect,
            ProbeCapability::WebApi,
            vec![200],
            vec!["application/json".into()],
            vec!["zui-probe-v1".into()],
        ),
        (
            "zui-probe-range",
            "Binary range",
            format!("{root}/bytes"),
            manifest::ProbeKind::ControlledRange,
            ProbeCapability::CdnMedia,
            vec![206],
            vec!["application/octet-stream".into()],
            Vec::new(),
        ),
        (
            "zui-probe-websocket",
            "WebSocket echo",
            format!("{}/ws", root.replacen("https://", "wss://", 1)),
            manifest::ProbeKind::WebsocketEcho,
            ProbeCapability::Websocket,
            vec![],
            vec![],
            vec![],
        ),
        (
            "zui-probe-http1",
            "Controlled HTTP/1.1",
            format!("{root}/diagnostic"),
            manifest::ProbeKind::Http1,
            ProbeCapability::Transport,
            vec![200],
            vec!["application/json".into()],
            vec!["zui-probe-v1".into()],
        ),
        (
            "zui-probe-http2",
            "Controlled HTTP/2",
            format!("{root}/diagnostic"),
            manifest::ProbeKind::Http2,
            ProbeCapability::Transport,
            vec![200],
            vec!["application/json".into()],
            vec!["zui-probe-v1".into()],
        ),
        (
            "zui-probe-http3",
            "Controlled QUIC / HTTP/3",
            root.to_string(),
            manifest::ProbeKind::QuicHttp3,
            ProbeCapability::Http3,
            vec![],
            vec![],
            vec![],
        ),
    ];
    definitions
        .into_iter()
        .map(
            |(
                id,
                name,
                value,
                probe_kind,
                capability,
                allowed_statuses,
                content_types,
                markers,
            )| Target {
                id: id.into(),
                name: name.into(),
                service: "ZUI Probe".into(),
                value,
                probe_kind,
                capability,
                weight: 0,
                required: false,
                diagnostic: true,
                allowed_statuses,
                content_types,
                markers,
            },
        )
        .collect()
}

pub fn default_target_configs() -> Vec<TestTargetConfig> {
    manifest::default_configs()
}

pub fn migrate_target_configs(configs: &[TestTargetConfig]) -> Vec<TestTargetConfig> {
    manifest::migrate_configs(configs)
}

pub fn validate_target_configs(configs: &[TestTargetConfig]) -> Result<(), String> {
    if configs.len() > 100 {
        return Err("No more than 100 test targets are allowed".into());
    }
    for config in configs {
        manifest::validate_config(config)?;
    }
    Ok(())
}

fn load_custom_targets() -> Option<Vec<Target>> {
    let settings = settings::load_settings().ok()?;
    if settings.test_targets.is_empty() {
        return None;
    }
    let mut targets = fallback_targets();
    for config in &settings.test_targets {
        if let Some(definition) =
            manifest::definition_for(&config.service, &config.name, &config.value)
        {
            targets.retain(|target| target.id != definition.id);
            if let Some(target) = target_from_config(config) {
                targets.push(target);
            }
            continue;
        }
        if manifest::is_legacy_builtin(&config.service, &config.name, &config.value) {
            continue;
        }
        if let Some(target) = target_from_config(config) {
            targets.push(target);
        }
    }
    Some(targets)
}

fn target_from_config(config: &TestTargetConfig) -> Option<Target> {
    if !config.enabled {
        return None;
    }
    let service = config.service.trim();
    let value = config.value.trim();
    if manifest::validate_config(config).is_err() {
        return None;
    }
    let name = if config.name.trim().is_empty() {
        service
    } else {
        config.name.trim()
    };
    let definition = manifest::definition_for(service, name, value);
    let (
        id,
        probe_kind,
        capability,
        weight,
        required,
        diagnostic,
        allowed_statuses,
        content_types,
        markers,
    ) = if let Some(definition) = definition {
        (
            definition.id.to_string(),
            definition.kind,
            definition.capability,
            definition.weight,
            definition.required,
            definition.diagnostic,
            definition.allowed_statuses.to_vec(),
            definition
                .content_types
                .iter()
                .map(|value| (*value).to_string())
                .collect(),
            definition
                .markers
                .iter()
                .map(|value| (*value).to_string())
                .collect(),
        )
    } else {
        let (probe_kind, capability, weight, diagnostic) = if value
            .get(..5)
            .is_some_and(|prefix| prefix.eq_ignore_ascii_case("PING:"))
        {
            (
                manifest::ProbeKind::Ping,
                ProbeCapability::Transport,
                0,
                true,
            )
        } else if value
            .get(..4)
            .is_some_and(|prefix| prefix.eq_ignore_ascii_case("DNS:"))
        {
            (manifest::ProbeKind::Dns, ProbeCapability::Dns, 6, false)
        } else if reqwest::Url::parse(value)
            .ok()
            .is_some_and(|url| matches!(url.scheme(), "http" | "https"))
        {
            (
                manifest::ProbeKind::Http,
                ProbeCapability::Generic,
                10,
                false,
            )
        } else {
            return None;
        };
        (
            format!("custom:{service}:{name}:{value}"),
            probe_kind,
            capability,
            weight,
            false,
            diagnostic,
            (200..=299).collect(),
            Vec::new(),
            Vec::new(),
        )
    };
    Some(Target {
        id,
        name: name.into(),
        service: service.into(),
        value: value.into(),
        probe_kind,
        capability,
        weight,
        required,
        diagnostic,
        allowed_statuses,
        content_types,
        markers,
    })
}

fn fallback_targets() -> Vec<Target> {
    manifest::default_configs()
        .iter()
        .filter_map(target_from_config)
        .collect()
}

#[allow(clippy::too_many_arguments)]
fn build_result(
    id: String,
    preset_id: String,
    preset_name: String,
    preset_relative_path: String,
    engine: crate::models::ZapretEngine,
    mode: TestMode,
    started_at: String,
    finished_at: String,
    targets: Vec<TestTargetResult>,
    baseline: Option<BaselineSnapshot>,
    process_ok: bool,
) -> TestResult {
    let services = scoring::build_services(&targets);
    let summary = scoring::summarize(&services, process_ok);

    TestResult {
        schema_version: 3,
        id,
        preset_id,
        preset_name,
        engine,
        preset_version: preset_version_from_path(&preset_relative_path),
        mode,
        started_at,
        finished_at: finished_at.clone(),
        cached_at: finished_at,
        recommendation: summary.recommendation,
        score: summary.score,
        ok: summary.ok,
        total: summary.total,
        passed_weight: summary.passed_weight,
        total_weight: summary.total_weight,
        inconclusive: summary.inconclusive,
        regressions: summary.regressions,
        baseline,
        process_ok,
        services,
    }
}

fn preset_version_from_path(path: &str) -> String {
    let normalized = path.replace('\\', "/");
    let parts: Vec<&str> = normalized
        .split('/')
        .filter(|part| !part.is_empty())
        .collect();
    if parts.len() >= 2 {
        return format!("{}/{}", parts[0], parts[1]);
    }
    parts.first().copied().unwrap_or_default().to_string()
}

fn test_mode_label(mode: &TestMode) -> &'static str {
    match mode {
        TestMode::Selected => "Selected preset test",
        TestMode::All => "All presets test",
    }
}

fn test_abort_reason(result: &TestResult) -> Option<&'static str> {
    result
        .services
        .iter()
        .flat_map(|service| &service.targets)
        .any(|target| target.reason_code.as_deref() == Some("process_exited_external"))
        .then_some(TEST_ABORT_PROCESS_EXITED)
}

fn planned_check_count(targets: &[Target]) -> u32 {
    targets.len() as u32
}

fn emit_progress(app: &AppHandle, progress: &TestProgress) {
    let _ = app.emit("test_progress", progress);
}

fn trim_results(results: &mut Vec<TestResult>) -> bool {
    if results.len() <= MAX_STORED_RESULTS {
        return false;
    }
    let excess = results.len() - MAX_STORED_RESULTS;
    results.drain(0..excess);
    true
}

fn is_cancelled(state: &Mutex<RuntimeState>) -> bool {
    state.lock().unwrap().test_cancelled
}

fn state_reset(state: &Mutex<RuntimeState>) {
    let mut runtime = state.lock().unwrap();
    runtime.test_running = false;
    runtime.test_cancelled = false;
}

fn unix_timestamp() -> String {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        .to_string()
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
    fn planned_checks_are_stable_for_default_targets() {
        let targets = fallback_targets();
        let total = planned_check_count(&targets);
        assert_eq!(total, targets.len() as u32);
        assert!(total >= 15);
        assert_eq!(total, planned_check_count(&targets));
    }

    #[test]
    fn migrates_legacy_results_without_losing_successes() {
        let json = r#"{
          "id":"legacy-1","presetId":"preset","presetName":"Legacy","engine":"classic",
          "presetVersion":"2.2","mode":"selected","startedAt":"1","finishedAt":"2",
          "cachedAt":"2","recommendation":"recommended","score":100,"ok":1,"total":1,
          "services":[{"name":"Discord","status":"passed","ok":1,"total":1,"errors":[],
            "targets":[{"service":"Discord","label":"Discord HTTP1.1","url":"https://discord.com",
              "ok":true,"status":200,"latencyMs":10,"error":null}]}]
        }"#;
        let mut result: TestResult = serde_json::from_str(json).unwrap();
        assert!(migrate_result(&mut result));
        assert_eq!(result.schema_version, 3);
        assert_eq!(
            result.services[0].targets[0].probe_status,
            ProbeStatus::Passed
        );
        assert_eq!(result.score, 100);
    }

    #[test]
    fn external_process_exit_aborts_the_test_session() {
        let result = build_result(
            "test".into(),
            "preset".into(),
            "Preset".into(),
            "Zapret 2/preset.txt".into(),
            crate::models::ZapretEngine::Zapret2,
            TestMode::Selected,
            "1".into(),
            "2".into(),
            vec![runtime_process_failure_target("runtime")],
            None,
            false,
        );

        assert_eq!(test_abort_reason(&result), Some(TEST_ABORT_PROCESS_EXITED));
    }

    #[tokio::test]
    #[ignore = "requires live network access"]
    async fn live_stage_two_targets_return_structured_results() {
        for target in fallback_targets() {
            let result = check_target(target).await;
            println!(
                "{} / {} = {:?} ({:?})",
                result.service, result.label, result.probe_status, result.reason_code
            );
            assert!(!result.target_id.is_empty());
            assert!(!result.label.is_empty());
            assert!(!result.steps.is_empty());
            assert!(result.bytes_read.unwrap_or_default() <= protocol::MAX_RESPONSE_BYTES as u64);
        }
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
