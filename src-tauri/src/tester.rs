use crate::models::{
    BaselineSnapshot, FailureStage, LogSource, ProbeChange, ProbeContext, ProbeStatus, TestMode,
    TestPhase, TestProgress, TestResult, TestTargetConfig, TestTargetResult,
};
use crate::state::RuntimeState;
use crate::{json_storage, logging, paths, presets, services, settings, system_process};
use reqwest::{tls::Version, Method, Response};
use std::collections::HashMap;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
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
#[path = "tester/scoring.rs"]
mod scoring;

#[derive(Clone)]
struct Target {
    name: String,
    service: String,
    kind: TargetKind,
    weight: u16,
    required: bool,
    diagnostic: bool,
}

#[derive(Clone)]
enum TargetKind {
    Url(String),
    Ping(String),
    Dns { resolver: String, host: String },
}
const MAX_PARALLEL_TARGET_CHECKS: usize = 8;
const MAX_STORED_RESULTS: usize = 100;

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
    if result.schema_version >= 2 {
        return false;
    }
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
    let summary = scoring::summarize(&result.services, true);
    result.schema_version = 2;
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
            if cancelled {
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
    let checks = http_check_clients();
    let limiter = Arc::new(Semaphore::new(MAX_PARALLEL_TARGET_CHECKS));

    let mut tasks = tokio::task::JoinSet::new();
    for target in targets {
        if is_cancelled(state) {
            break;
        }

        match target.kind.clone() {
            TargetKind::Url(url) => {
                if checks.is_empty() {
                    let host = host_from_url(&url);
                    let unavailable_target = target.clone();
                    let unavailable_url = url.clone();
                    tasks.spawn(async move {
                        unavailable_http_target(
                            unavailable_target,
                            "HTTP client".into(),
                            unavailable_url,
                        )
                    });
                    if let Some(host) = host {
                        let limiter = Arc::clone(&limiter);
                        let target = target.clone();
                        tasks.spawn(async move {
                            let _permit = limiter.acquire_owned().await.ok();
                            check_ping_target(target, host).await
                        });
                    }
                    continue;
                }

                for check in &checks {
                    let limiter = Arc::clone(&limiter);
                    let mut target = target.clone();
                    target.weight = (target.weight / checks.len() as u16).max(1);
                    let url = url.clone();
                    let client = check.client.clone();
                    let label = check.label.to_string();
                    tasks.spawn(async move {
                        let _permit = limiter.acquire_owned().await.ok();
                        check_http_target(client, target, label, url).await
                    });
                }

                if let Some(host) = host_from_url(&url) {
                    let limiter = Arc::clone(&limiter);
                    let target = target.clone();
                    tasks.spawn(async move {
                        let _permit = limiter.acquire_owned().await.ok();
                        check_ping_target(target, host).await
                    });
                }
            }
            TargetKind::Ping(host) => {
                let limiter = Arc::clone(&limiter);
                tasks.spawn(async move {
                    let _permit = limiter.acquire_owned().await.ok();
                    check_ping_target(target, host).await
                });
            }
            TargetKind::Dns { resolver, host } => {
                let limiter = Arc::clone(&limiter);
                tasks.spawn(async move {
                    let _permit = limiter.acquire_owned().await.ok();
                    check_dns_target(target, resolver, host).await
                });
            }
        }
    }

    let mut results = Vec::new();

    while let Some(joined) = tasks.join_next().await {
        if is_cancelled(state) {
            break;
        }

        if monitor_process && !lifecycle::is_healthy(app, state) {
            tasks.abort_all();
            let failure = process_failure_target(
                "runtime",
                "zapret process exited during network checks".into(),
            );
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

struct HttpCheck {
    label: &'static str,
    client: reqwest::Client,
}

fn http_check_clients() -> Vec<HttpCheck> {
    [
        ("HTTP1.1", http_client(|builder| builder.http1_only())),
        (
            "TLS1.2",
            http_client(|builder| {
                builder
                    .min_tls_version(Version::TLS_1_2)
                    .max_tls_version(Version::TLS_1_2)
            }),
        ),
        (
            "TLS1.3",
            http_client(|builder| {
                builder
                    .min_tls_version(Version::TLS_1_3)
                    .max_tls_version(Version::TLS_1_3)
            }),
        ),
    ]
    .into_iter()
    .filter_map(|(label, client)| client.ok().map(|client| HttpCheck { label, client }))
    .collect()
}

fn http_client(
    configure: impl FnOnce(reqwest::ClientBuilder) -> reqwest::ClientBuilder,
) -> Result<reqwest::Client, reqwest::Error> {
    let builder = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .redirect(reqwest::redirect::Policy::limited(4))
        .user_agent(concat!("ZUI/", env!("CARGO_PKG_VERSION")));
    configure(builder).build()
}

async fn check_http_target(
    client: reqwest::Client,
    target: Target,
    label: String,
    url: String,
) -> TestTargetResult {
    let started = Instant::now();
    let label = format!("{} {label}", target.name);

    match send_http_probe(&client, &url).await {
        Ok(response) => {
            let status = response.status().as_u16();
            let probe_status = if status == 429 {
                ProbeStatus::Inconclusive
            } else if status < 500 {
                ProbeStatus::Passed
            } else {
                ProbeStatus::Failed
            };
            TestTargetResult {
                service: target.service,
                label,
                url,
                ok: probe_status == ProbeStatus::Passed,
                probe_status,
                context: ProbeContext::Preset,
                change: ProbeChange::NotCompared,
                failure_stage: (probe_status != ProbeStatus::Passed).then_some(FailureStage::Http),
                reason_code: match probe_status {
                    ProbeStatus::Inconclusive => Some("http_rate_limited".into()),
                    ProbeStatus::Failed => Some("http_server_error".into()),
                    _ => None,
                },
                weight: target.weight,
                required: target.required,
                diagnostic: target.diagnostic,
                status: Some(status),
                latency_ms: Some(started.elapsed().as_millis()),
                error: (probe_status != ProbeStatus::Passed).then(|| format!("HTTP {status}")),
            }
        }
        Err(error) => TestTargetResult {
            service: target.service,
            label,
            url,
            ok: false,
            probe_status: ProbeStatus::Failed,
            context: ProbeContext::Preset,
            change: ProbeChange::NotCompared,
            failure_stage: Some(classify_reqwest_failure(&error)),
            reason_code: Some(reqwest_reason_code(&error).into()),
            weight: target.weight,
            required: target.required,
            diagnostic: target.diagnostic,
            status: None,
            latency_ms: Some(started.elapsed().as_millis()),
            error: Some(error.to_string()),
        },
    }
}

async fn send_http_probe(client: &reqwest::Client, url: &str) -> Result<Response, reqwest::Error> {
    let mut last_error = None;
    for attempt in 0..2 {
        match client.request(Method::HEAD, url).send().await {
            Ok(response) if response.status().as_u16() != 405 => return Ok(response),
            Ok(_) => match client.request(Method::GET, url).send().await {
                Ok(response) => return Ok(response),
                Err(error) => last_error = Some(error),
            },
            Err(error) => last_error = Some(error),
        }
        if attempt == 0 {
            tokio::time::sleep(Duration::from_millis(250)).await;
        }
    }

    match client.request(Method::GET, url).send().await {
        Ok(response) => Ok(response),
        Err(error) => Err(last_error.unwrap_or(error)),
    }
}

fn classify_reqwest_failure(error: &reqwest::Error) -> FailureStage {
    let detail = error.to_string().to_ascii_lowercase();
    if detail.contains("dns") || detail.contains("resolve") {
        FailureStage::Dns
    } else if detail.contains("tls")
        || detail.contains("certificate")
        || detail.contains("handshake")
    {
        FailureStage::Tls
    } else if error.is_connect() {
        FailureStage::Tcp
    } else {
        FailureStage::Http
    }
}

fn reqwest_reason_code(error: &reqwest::Error) -> &'static str {
    if error.is_timeout() {
        "http_timeout"
    } else if error.is_connect() {
        "http_connect_failed"
    } else if error.is_redirect() {
        "http_redirect_failed"
    } else if error.is_builder() {
        "http_request_invalid"
    } else {
        "http_request_failed"
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
    }
}

fn process_failure_target(preset_path: &str, error: String) -> TestTargetResult {
    let mut target = startup_failure_target(preset_path, error);
    target.label = "zapret readiness".into();
    target.reason_code = Some("process_not_ready".into());
    target
}

fn unavailable_http_target(target: Target, label: String, url: String) -> TestTargetResult {
    TestTargetResult {
        service: target.service,
        label: format!("{} {label}", target.name),
        url,
        ok: false,
        probe_status: ProbeStatus::Inconclusive,
        context: ProbeContext::Preset,
        change: ProbeChange::NotCompared,
        failure_stage: Some(FailureStage::Internal),
        reason_code: Some("http_client_unavailable".into()),
        weight: target.weight,
        required: target.required,
        diagnostic: target.diagnostic,
        status: None,
        latency_ms: None,
        error: Some("HTTP test client is unavailable".into()),
    }
}

async fn check_dns_target(target: Target, resolver: String, host: String) -> TestTargetResult {
    let outcome = dns::probe(&resolver, &host).await;
    let ok = outcome.status == ProbeStatus::Passed;
    TestTargetResult {
        service: target.service,
        label: format!("{} DNS", target.name),
        url: format!("DNS:{resolver}:{host}"),
        ok,
        probe_status: outcome.status,
        context: ProbeContext::Preset,
        change: ProbeChange::NotCompared,
        failure_stage: (!ok).then_some(FailureStage::Dns),
        reason_code: outcome.reason_code,
        weight: target.weight,
        required: target.required,
        diagnostic: target.diagnostic,
        status: None,
        latency_ms: Some(outcome.latency_ms),
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
                }
            }
            Err(error) => TestTargetResult {
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
                status: None,
                latency_ms: Some(started.elapsed().as_millis()),
                error: Some(error.to_string()),
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
    load_custom_targets().unwrap_or_else(fallback_targets)
}

pub fn default_target_configs() -> Vec<TestTargetConfig> {
    manifest::default_configs()
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
    let targets: Vec<Target> = settings
        .test_targets
        .iter()
        .filter_map(target_from_config)
        .collect();
    (!settings.test_targets.is_empty()).then_some(targets)
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
    let kind = target_kind(value)?;
    let name = if config.name.trim().is_empty() {
        service
    } else {
        config.name.trim()
    };
    let definition = manifest::definition_for(service, name, value);
    let (weight, required, diagnostic) = if let Some(definition) = definition {
        (
            definition.weight,
            definition.required,
            definition.diagnostic,
        )
    } else {
        let (weight, diagnostic) = match &kind {
            TargetKind::Ping(_) => (0, true),
            TargetKind::Dns { .. } => (6, false),
            TargetKind::Url(_) => (10, false),
        };
        (weight, false, diagnostic)
    };
    Some(Target {
        name: name.into(),
        service: service.into(),
        kind,
        weight,
        required,
        diagnostic,
    })
}

fn fallback_targets() -> Vec<Target> {
    manifest::default_configs()
        .iter()
        .filter_map(target_from_config)
        .collect()
}

fn target_kind(value: &str) -> Option<TargetKind> {
    let value = value.trim();
    if value.len() >= 5 && value[..5].eq_ignore_ascii_case("PING:") {
        let host = value[5..].trim();
        if !host.is_empty() {
            return Some(TargetKind::Ping(host.into()));
        }
    }
    if value.len() >= 4 && value[..4].eq_ignore_ascii_case("DNS:") {
        let (resolver, host) = manifest::parse_dns_target(value).ok()?;
        if !resolver.trim().is_empty() && !host.trim().is_empty() {
            return Some(TargetKind::Dns {
                resolver: resolver.trim().into(),
                host: host.trim().into(),
            });
        }
    }
    if reqwest::Url::parse(value)
        .ok()
        .is_some_and(|url| matches!(url.scheme(), "http" | "https"))
    {
        return Some(TargetKind::Url(value.into()));
    }
    None
}

fn host_from_url(value: &str) -> Option<String> {
    reqwest::Url::parse(value)
        .ok()
        .and_then(|url| url.host_str().map(str::to_string))
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
        schema_version: 2,
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

fn planned_check_count(targets: &[Target]) -> u32 {
    let http_checks = http_check_clients().len().max(1) as u32;
    targets
        .iter()
        .map(|target| match target.kind {
            TargetKind::Url(_) => http_checks + 1,
            TargetKind::Ping(_) => 1,
            TargetKind::Dns { .. } => 1,
        })
        .sum()
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
        assert!(total > targets.len() as u32);
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
        assert_eq!(result.schema_version, 2);
        assert_eq!(
            result.services[0].targets[0].probe_status,
            ProbeStatus::Passed
        );
        assert_eq!(result.score, 100);
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
