use crate::models::{Profile, TgWsConnectivityKind, TgWsConnectivityProbe, TgWsConnectivityReport};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::sync::OnceLock;
use std::thread::JoinHandle;
use std::time::Duration;
use tauri::Manager;
use tg_ws_proxy_rs::{check, config::Config, default_domains, server};
use tokio::sync::oneshot;
use tracing_subscriber::fmt::MakeWriter;

pub const ENGINE_NAME: &str = "ZUI tg-ws engine";
pub const ENGINE_VERSION: &str = tg_ws_proxy_rs::VERSION;

static TG_WS_TRACE_SENDER: OnceLock<std::sync::mpsc::SyncSender<String>> = OnceLock::new();
static TG_WS_VERBOSE: AtomicBool = AtomicBool::new(false);
static TG_WS_DROPPED_LOG_LINES: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

struct TgWsMakeWriter;

struct TgWsTraceWriter {
    enabled: bool,
    bytes: Vec<u8>,
}

impl std::io::Write for TgWsTraceWriter {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if self.enabled {
            self.bytes.extend_from_slice(bytes);
        }
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl Drop for TgWsTraceWriter {
    fn drop(&mut self) {
        if !self.enabled || self.bytes.is_empty() {
            return;
        }
        let message = String::from_utf8_lossy(&self.bytes).trim().to_string();
        if !message.is_empty() {
            let message = censor_tg_ws_log_domains(&redact_tg_ws_log_secrets(&message));
            if let Some(sender) = TG_WS_TRACE_SENDER.get() {
                match sender.try_send(message) {
                    Ok(()) => {
                        let dropped = TG_WS_DROPPED_LOG_LINES.swap(0, Ordering::Relaxed);
                        if dropped > 0 {
                            let _ = sender.try_send(format!(
                                "tg-ws log queue recovered; {dropped} verbose lines were dropped to protect application memory"
                            ));
                        }
                    }
                    Err(std::sync::mpsc::TrySendError::Full(_)) => {
                        TG_WS_DROPPED_LOG_LINES.fetch_add(1, Ordering::Relaxed);
                    }
                    Err(std::sync::mpsc::TrySendError::Disconnected(_)) => {}
                }
            }
        }
    }
}

fn redact_tg_ws_log_secrets(message: &str) -> String {
    if let Some(index) = message.find("Secret:") {
        return format!("{}Secret: [REDACTED]", &message[..index]);
    }

    let mut redacted = message.to_string();
    let mut search_from = 0;
    while let Some(relative) = redacted[search_from..].find("secret=") {
        let value_start = search_from + relative + "secret=".len();
        let value_len = redacted[value_start..]
            .find(|character: char| character.is_whitespace() || character == '&')
            .unwrap_or(redacted.len() - value_start);
        redacted.replace_range(value_start..value_start + value_len, "[REDACTED]");
        search_from = value_start + "[REDACTED]".len();
    }
    redacted
}

fn censor_tg_ws_log_domains(message: &str) -> String {
    let mut result = String::with_capacity(message.len());
    let mut token = String::new();

    let flush = |result: &mut String, token: &mut String| {
        if is_log_domain(token) && !is_uncensored_log_domain(token) {
            let domain = token.trim_end_matches('.');
            let trailing_dots = token.len() - domain.len();
            let parts = domain.split('.').collect::<Vec<_>>();
            for (index, part) in parts.iter().enumerate() {
                if index > 0 {
                    result.push('.');
                }
                if index + 1 == parts.len() {
                    result.push_str(part);
                } else {
                    let keep = part.len() / 2;
                    result.push_str(&part[..keep]);
                    result.extend(std::iter::repeat_n('*', part.len() - keep));
                }
            }
            result.extend(std::iter::repeat_n('.', trailing_dots));
        } else {
            result.push_str(token);
        }
        token.clear();
    };

    for character in message.chars() {
        if character.is_ascii_alphanumeric() || matches!(character, '-' | '.') {
            token.push(character);
        } else {
            flush(&mut result, &mut token);
            result.push(character);
        }
    }
    flush(&mut result, &mut token);
    result
}

fn is_log_domain(value: &str) -> bool {
    let value = value.trim_end_matches('.');
    let mut labels = value.split('.');
    let Some(first) = labels.next() else {
        return false;
    };
    if first.is_empty() {
        return false;
    }
    let labels = std::iter::once(first).chain(labels).collect::<Vec<_>>();
    labels.len() >= 2
        && labels.iter().all(|label| {
            !label.is_empty()
                && label.len() <= 63
                && label
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
                && label
                    .as_bytes()
                    .first()
                    .is_some_and(u8::is_ascii_alphanumeric)
                && label
                    .as_bytes()
                    .last()
                    .is_some_and(u8::is_ascii_alphanumeric)
        })
        && labels
            .last()
            .is_some_and(|tld| tld.len() >= 2 && tld.bytes().all(|byte| byte.is_ascii_alphabetic()))
}

fn is_uncensored_log_domain(value: &str) -> bool {
    let normalized = value.trim_end_matches('.').to_ascii_lowercase();
    normalized == "telegram.org"
        || normalized.ends_with(".telegram.org")
        || normalized.ends_with(".log")
}

impl<'a> MakeWriter<'a> for TgWsMakeWriter {
    type Writer = TgWsTraceWriter;

    fn make_writer(&'a self) -> Self::Writer {
        TgWsTraceWriter {
            enabled: true,
            bytes: Vec::new(),
        }
    }

    fn make_writer_for(&'a self, metadata: &tracing::Metadata<'_>) -> Self::Writer {
        let verbose = TG_WS_VERBOSE.load(Ordering::Relaxed);
        let enabled = metadata.target().starts_with("tg_ws_proxy_rs")
            && (verbose || *metadata.level() <= tracing::Level::INFO);
        TgWsTraceWriter {
            enabled,
            bytes: Vec::new(),
        }
    }
}

/// Route the embedded engine's tracing events into ZUI's normal tg-ws log.
/// Safe to call more than once; the global subscriber is installed once.
pub fn install_logging(app: tauri::AppHandle) {
    if TG_WS_TRACE_SENDER.get().is_some() {
        return;
    }
    let (sender, receiver) = std::sync::mpsc::sync_channel::<String>(2048);
    if TG_WS_TRACE_SENDER.set(sender).is_err() {
        return;
    }
    let _ = tracing_subscriber::fmt()
        .with_writer(TgWsMakeWriter)
        .with_ansi(false)
        .without_time()
        .with_target(false)
        .with_max_level(tracing::Level::TRACE)
        .try_init();

    let _ = std::thread::Builder::new()
        .name("zui-tg-ws-log".into())
        .spawn(move || {
            while let Ok(message) = receiver.recv() {
                let state = app.state::<std::sync::Mutex<crate::state::RuntimeState>>();
                crate::logging::push(&app, &state, crate::models::LogSource::TgWs, message);
            }
        });
}

pub struct TgWsRuntimeHandle {
    shutdown_tx: Option<oneshot::Sender<()>>,
    shutdown_requested: Arc<AtomicBool>,
    thread: JoinHandle<()>,
    failure: Arc<std::sync::Mutex<Option<String>>>,
    pub host: String,
    pub port: u16,
    pub link: String,
}

impl TgWsRuntimeHandle {
    pub fn is_running(&self) -> bool {
        !self.thread.is_finished()
    }

    pub fn failure(&self) -> Option<String> {
        self.failure.lock().ok().and_then(|failure| failure.clone())
    }

    pub fn stop(mut self) -> Result<(), String> {
        self.shutdown_requested.store(true, Ordering::Release);
        if let Some(shutdown_tx) = self.shutdown_tx.take() {
            let _ = shutdown_tx.send(());
        }
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        while !self.thread.is_finished() && std::time::Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(25));
        }
        if !self.thread.is_finished() {
            return Err(
                "tg-ws runtime did not stop within 5 seconds; its thread was detached".to_string(),
            );
        }
        self.thread
            .join()
            .map_err(|_| "tg-ws runtime thread panicked".to_string())
    }
}

pub fn spawn(profile: &Profile) -> Result<TgWsRuntimeHandle, String> {
    TG_WS_VERBOSE.store(profile.tg_ws_verbose, Ordering::Relaxed);
    let config = config_from_profile(profile)?;
    let host = config.bind_host();
    let (ready_tx, ready_rx) = std::sync::mpsc::channel::<Result<server::ListenInfo, String>>();
    let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();
    let thread_config = config.clone();
    let failure = Arc::new(std::sync::Mutex::new(None::<String>));
    let thread_failure = Arc::clone(&failure);
    let shutdown_requested = Arc::new(AtomicBool::new(false));
    let thread_shutdown_requested = Arc::clone(&shutdown_requested);

    let thread = std::thread::Builder::new()
        .name("zui-tg-ws-runtime".into())
        .spawn(move || {
            let runtime = match tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .thread_name("zui-tg-ws-worker")
                .build()
            {
                Ok(runtime) => runtime,
                Err(error) => {
                    let message = format!("could not create Tokio runtime: {error}");
                    if let Ok(mut failure) = thread_failure.lock() {
                        *failure = Some(message.clone());
                    }
                    let _ = ready_tx.send(Err(message));
                    return;
                }
            };

            runtime.block_on(async move {
                let ready_tx = Arc::new(std::sync::Mutex::new(Some(ready_tx)));
                let listen_tx = Arc::clone(&ready_tx);
                let result = server::run_with_listen(
                    thread_config,
                    async {
                        let _ = shutdown_rx.await;
                    },
                    move |info| {
                        if let Some(sender) = listen_tx.lock().ok().and_then(|mut tx| tx.take()) {
                            let _ = sender.send(Ok(info));
                        }
                    },
                )
                .await;

                if let Err(error) = result {
                    let message = format!("tg-ws server stopped with error: {error}");
                    if let Ok(mut failure) = thread_failure.lock() {
                        *failure = Some(message.clone());
                    }
                    tracing::error!(error = %error, "tg-ws runtime stopped with error");
                    if let Some(sender) = ready_tx.lock().ok().and_then(|mut tx| tx.take()) {
                        let _ = sender.send(Err(message));
                    }
                } else {
                    let pending_sender = ready_tx.lock().ok().and_then(|mut tx| tx.take());
                    if thread_shutdown_requested.load(Ordering::Acquire) {
                        // A user-initiated stop is the normal terminal state,
                        // not a runtime failure to surface on the next status
                        // poll.  Keep startup cancellation actionable for the
                        // caller that is still waiting on the ready channel.
                        if let Some(sender) = pending_sender {
                            let _ =
                                sender
                                    .send(Err("tg-ws stopped before the listener became ready"
                                        .to_string()));
                        }
                    } else {
                        let message = if pending_sender.is_some() {
                            "tg-ws stopped before the listener became ready".to_string()
                        } else {
                            "tg-ws server stopped unexpectedly".to_string()
                        };
                        if let Ok(mut failure) = thread_failure.lock() {
                            *failure = Some(message.clone());
                        }
                        if let Some(sender) = pending_sender {
                            let _ = sender.send(Err(message));
                        }
                    }
                }
            });

            // Dropping a multi-thread Tokio runtime waits indefinitely for
            // blocking tasks (notably Windows DNS resolution) that cannot be
            // cancelled. During a busy proxy shutdown that made the owning
            // thread miss ZUI's five-second stop deadline even though the
            // server accept loop had already stopped. Bound that final drain;
            // ordinary async tasks are cancelled immediately, while an
            // in-flight OS resolver is allowed to finish in the background.
            runtime.shutdown_timeout(Duration::from_secs(1));
        })
        .map_err(|error| error.to_string())?;

    match ready_rx.recv_timeout(Duration::from_secs(30)) {
        Ok(Ok(info)) => Ok(TgWsRuntimeHandle {
            shutdown_tx: Some(shutdown_tx),
            shutdown_requested,
            thread,
            failure,
            host,
            port: info.addr.port(),
            link: info.tg_link,
        }),
        Ok(Err(error)) => {
            let _ = thread.join();
            Err(format!("tg-ws listener startup failed: {error}"))
        }
        Err(error) => {
            shutdown_requested.store(true, Ordering::Release);
            let _ = shutdown_tx.send(());
            let deadline = std::time::Instant::now() + Duration::from_secs(5);
            while !thread.is_finished() && std::time::Instant::now() < deadline {
                std::thread::sleep(Duration::from_millis(25));
            }
            let cleanup = if thread.is_finished() {
                let _ = thread.join();
                "runtime stopped"
            } else {
                "runtime did not stop within 5 seconds and was detached"
            };
            Err(format!(
                "tg-ws listener did not become ready within 30 seconds: {error}; {cleanup}"
            ))
        }
    }
}

pub fn configuration_summary(profile: &Profile) -> Result<String, String> {
    let config = config_from_profile(profile)?;
    let cf_mode = if !profile.tg_ws_cf_proxy_enabled {
        "disabled".to_string()
    } else if profile.tg_ws_cf_custom_enabled {
        format!("custom({})", config.cf_domains.join(","))
    } else if config.default_domains {
        "automatic-default-domains".to_string()
    } else {
        "enabled-without-domains".to_string()
    };
    let workers = if config.cf_worker_domains().is_empty() {
        "disabled".to_string()
    } else {
        config.cf_worker_domains().join(",")
    };
    Ok(format!(
        "listen={}:{}, dc_routes={}, cf_proxy={}, cf_workers={}, fronting={}, priority={}, balance={}, pool_size={}, socket_buffer_kib={}, force_test_dc={}, verbose={}",
        config.bind_host(),
        config.port,
        config
            .dc_ip
            .iter()
            .map(|(dc, ip)| format!("{dc}:{ip}"))
            .collect::<Vec<_>>()
            .join(","),
        cf_mode,
        workers,
        config.fronting_domain.as_deref().unwrap_or("disabled"),
        config.cf_priority,
        config.cf_balance,
        config.pool_size,
        config.buf_kb,
        config.force_test_dc,
        config.verbose,
    ))
}

pub(crate) fn config_from_profile(profile: &Profile) -> Result<Config, String> {
    let secret = normalize_secret(&profile.tg_ws_secret);
    if secret.is_empty() {
        return Err("tg-ws Secret is empty; generate and save a Secret first".to_string());
    }

    let cf_domains: Vec<String> =
        if profile.tg_ws_cf_proxy_enabled && profile.tg_ws_cf_custom_enabled {
            profile
                .tg_ws_cf_domains
                .iter()
                .map(|domain| normalize_domain(domain))
                .filter(|domain| !domain.is_empty())
                .collect()
        } else {
            Vec::new()
        };
    let cf_worker_domains = if profile.tg_ws_cf_worker_enabled {
        profile
            .tg_ws_cf_worker_domain
            .as_ref()
            .map(|domains| split_domain_list(domains))
            .unwrap_or_default()
    } else {
        Vec::new()
    };
    let fronting_domain = profile
        .tg_ws_fronting_domain
        .as_deref()
        .map(str::trim)
        .filter(|domain| !domain.is_empty())
        .map(str::to_string);

    let dc_ip = parse_dc_ip_list(&profile.tg_ws_dc_ips)?;

    let mut args = vec![
        "tg-ws-proxy".to_string(),
        "--port".to_string(),
        profile.tg_ws_port.to_string(),
        "--host".to_string(),
        profile.tg_ws_host.clone(),
        "--secret".to_string(),
        secret,
        "--buf-kb".to_string(),
        profile.tg_ws_buf_kb.max(4).to_string(),
        "--pool-size".to_string(),
        profile.tg_ws_pool_size.to_string(),
        "--link-ip".to_string(),
        advertised_link_host(&profile.tg_ws_host),
        "--no-outbound-proxy".to_string(),
    ];
    for (dc, ip) in dc_ip {
        args.push("--dc-ip".to_string());
        args.push(format!("{dc}:{ip}"));
    }
    for domain in cf_domains {
        args.push("--cf-domain".to_string());
        args.push(domain);
    }
    for domain in cf_worker_domains {
        args.push("--cf-worker-domain".to_string());
        args.push(domain);
    }
    if let Some(domain) = fronting_domain {
        args.push("--fronting-domain".to_string());
        args.push(domain);
    }
    if profile.tg_ws_force_test_dc {
        args.push("--force-test-dc".to_string());
    }
    if profile.tg_ws_verbose {
        args.push("--verbose".to_string());
    } else {
        args.push("--quiet".to_string());
    }
    if profile.tg_ws_cf_priority {
        args.push("--cf-priority".to_string());
    }
    if profile.tg_ws_cf_balance {
        args.push("--cf-balance".to_string());
    }
    if profile.tg_ws_cf_proxy_enabled
        && !profile.tg_ws_cf_custom_enabled
        && profile.tg_ws_default_domains
    {
        args.push("--default-domains".to_string());
    }

    Config::try_from_args(args).map_err(|error| format!("invalid tg-ws configuration: {error}"))
}

fn normalize_domain(value: &str) -> String {
    value
        .trim()
        .trim_start_matches("https://")
        .trim_start_matches("http://")
        .split('/')
        .next()
        .unwrap_or_default()
        .trim_end_matches('.')
        .to_string()
}

fn parse_dc_ip_list(values: &[String]) -> Result<Vec<(u32, String)>, String> {
    let mut parsed = Vec::new();
    for value in values
        .iter()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
    {
        let (dc, ip) = value
            .split_once(':')
            .ok_or_else(|| format!("Invalid Telegram DC route '{value}'; expected DC:IP"))?;
        let dc = dc
            .trim()
            .parse::<u32>()
            .map_err(|_| format!("Invalid Telegram DC number in '{value}'"))?;
        if !matches!(dc, 1 | 2 | 3 | 4 | 5 | 203) {
            return Err(format!("Unsupported Telegram DC {dc} in '{value}'"));
        }
        let ip = ip.trim();
        ip.parse::<std::net::IpAddr>()
            .map_err(|_| format!("Invalid Telegram DC IP in '{value}'"))?;
        parsed.retain(|(existing, _)| *existing != dc);
        parsed.push((dc, ip.to_string()));
    }
    Ok(parsed)
}

pub async fn test_cf_proxy(profile: &Profile) -> Result<TgWsConnectivityReport, String> {
    let _ = rustls::crypto::ring::default_provider().install_default();
    let config = config_from_profile(profile)?;
    let outbound = config.outbound_connector()?;
    let mut domains = if profile.tg_ws_cf_custom_enabled {
        profile
            .tg_ws_cf_domains
            .iter()
            .map(|domain| normalize_domain(domain))
            .filter(|domain| !domain.is_empty())
            .collect::<Vec<_>>()
    } else {
        default_domains::fetch_default_domains_with_outbound(&outbound).await
    };
    domains.dedup_by(|left, right| left.eq_ignore_ascii_case(right));
    if domains.is_empty() {
        return Err("No Cloudflare proxy domains configured".to_string());
    }

    let mut probes = Vec::new();
    // The upstream UI tries automatic domains from newest to oldest and stops
    // once one domain works for every DC. Custom domains are all reported.
    if !profile.tg_ws_cf_custom_enabled {
        domains.reverse();
    }
    let mut automatic_domain_ok = false;
    for domain in domains {
        let results = check::probe_cf_domain_all_dcs(
            &domain,
            config.skip_tls_verify,
            Duration::from_secs(config.cf_connect_timeout),
            &outbound,
        )
        .await;
        let domain_ok = results.iter().all(|result| result.ok);
        probes.extend(map_probes(&domain, results));
        if domain_ok && !profile.tg_ws_cf_custom_enabled {
            automatic_domain_ok = true;
            break;
        }
    }
    let mut report = connectivity_report(TgWsConnectivityKind::CfProxy, probes);
    if !profile.tg_ws_cf_custom_enabled {
        report.all_ok = automatic_domain_ok;
    }
    Ok(report)
}

pub async fn test_cf_worker(profile: &Profile) -> Result<TgWsConnectivityReport, String> {
    let _ = rustls::crypto::ring::default_provider().install_default();
    let config = config_from_profile(profile)?;
    let outbound = config.outbound_connector()?;
    let domains = profile
        .tg_ws_cf_worker_domain
        .as_deref()
        .map(split_domain_list)
        .unwrap_or_default();
    if domains.is_empty() {
        return Err("No Cloudflare Worker domains configured".to_string());
    }

    let mut probes = Vec::new();
    for domain in domains {
        let results = check::probe_cf_worker_all_dcs(
            &domain,
            config.skip_tls_verify,
            Duration::from_secs(config.cf_connect_timeout),
            &outbound,
        )
        .await;
        probes.extend(map_probes(&domain, results));
    }
    Ok(connectivity_report(TgWsConnectivityKind::CfWorker, probes))
}

fn map_probes(
    domain: &str,
    probes: Vec<check::ConnectivityProbeResult>,
) -> Vec<TgWsConnectivityProbe> {
    probes
        .into_iter()
        .map(|probe| TgWsConnectivityProbe {
            domain: domain.to_string(),
            dc: probe.dc,
            target: probe.target,
            ok: probe.ok,
            latency_ms: probe.latency_ms,
            detail: probe.detail,
        })
        .collect()
}

fn connectivity_report(
    kind: TgWsConnectivityKind,
    probes: Vec<TgWsConnectivityProbe>,
) -> TgWsConnectivityReport {
    TgWsConnectivityReport {
        all_ok: !probes.is_empty() && probes.iter().all(|probe| probe.ok),
        kind,
        probes,
    }
}

fn split_domain_list(value: &str) -> Vec<String> {
    let mut domains = Vec::new();
    for domain in value
        .replace([',', ';'], " ")
        .split_whitespace()
        .map(str::trim)
        .filter(|domain| !domain.is_empty())
    {
        let domain = domain.to_string();
        if !domains
            .iter()
            .any(|existing: &String| existing.eq_ignore_ascii_case(&domain))
        {
            domains.push(domain);
        }
    }
    domains
}

fn advertised_link_host(bind_host: &str) -> String {
    match bind_host.trim() {
        "0.0.0.0" | "::" => "127.0.0.1".to_string(),
        host => host.to_string(),
    }
}

fn normalize_secret(raw: &str) -> String {
    let trimmed = raw.trim();
    if is_valid_tg_secret(trimmed) {
        return trimmed.to_string();
    }

    if trimmed.is_empty() {
        return String::new();
    }

    let mut bytes = [0u8; 16];
    for (index, value) in trimmed.as_bytes().iter().take(16).enumerate() {
        bytes[index] = *value;
    }
    hex::encode(bytes)
}

fn is_valid_tg_secret(value: &str) -> bool {
    if hex::decode(value).is_err() {
        return false;
    }
    matches!(value.len(), 32 | 34) || (value.len() > 34 && value.starts_with("ee"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_valid_hex_secret() {
        let secret = "0123456789abcdef0123456789abcdef";

        assert_eq!(normalize_secret(secret), secret);
    }

    #[test]
    fn converts_plain_secret_to_telegram_hex_secret() {
        assert_eq!(
            normalize_secret("zui-secret"),
            "7a75692d736563726574000000000000"
        );
    }

    #[test]
    fn redacts_banner_secret_from_engine_logs() {
        assert_eq!(
            redact_tg_ws_log_secrets(" INFO   Secret:        0123456789abcdef0123456789abcdef"),
            " INFO   Secret: [REDACTED]"
        );
    }

    #[test]
    fn redacts_secret_inside_telegram_links() {
        assert_eq!(
            redact_tg_ws_log_secrets("tg://proxy?server=127.0.0.1&port=1443&secret=deadbeef next"),
            "tg://proxy?server=127.0.0.1&port=1443&secret=[REDACTED] next"
        );
    }

    #[test]
    fn censors_non_telegram_domains_like_flowseal_1_10_2() {
        assert_eq!(
            censor_tg_ws_log_domains(
                "CF kws4.pclead.co.uk failed; Telegram kws4.web.telegram.org; 149.154.167.220"
            ),
            "CF kw**.pcl***.c*.uk failed; Telegram kws4.web.telegram.org; 149.154.167.220"
        );
    }

    #[test]
    fn builds_direct_dc_config_by_default() {
        let profile = Profile::default();
        let config = config_from_profile(&profile).expect("profile config");

        assert_eq!(config.host.as_deref(), Some("127.0.0.1"));
        assert_eq!(config.port, 1443);
        assert_eq!(
            config.dc_ip,
            vec![
                (2, "149.154.167.220".to_string()),
                (4, "149.154.167.220".to_string())
            ]
        );
        assert!(config.cf_domains.is_empty());
        assert!(config.default_domains);
        assert_eq!(config.buf_kb, 256);
        assert_eq!(config.pool_size, 4);
    }

    #[test]
    fn carries_cloudflare_routing_options() {
        let profile = Profile {
            tg_ws_cf_custom_enabled: true,
            tg_ws_default_domains: true,
            tg_ws_cf_domains: vec![
                " one.example ".to_string(),
                String::new(),
                "two.example".to_string(),
            ],
            tg_ws_cf_worker_domain: Some("worker.example".to_string()),
            tg_ws_cf_worker_enabled: true,
            tg_ws_cf_priority: true,
            tg_ws_cf_balance: true,
            ..Profile::default()
        };

        let config = config_from_profile(&profile).expect("profile config");

        assert!(!config.default_domains);
        assert_eq!(config.dc_ip.len(), 2);
        assert_eq!(
            config.cf_domains,
            vec!["one.example".to_string(), "two.example".to_string()]
        );
        assert_eq!(config.cf_worker_domains(), &["worker.example".to_string()]);
        assert!(config.cf_priority);
        assert!(config.cf_balance);
    }

    #[test]
    fn carries_multiple_worker_domains_from_profile_string() {
        let profile = Profile {
            tg_ws_cf_worker_enabled: true,
            tg_ws_cf_worker_domain: Some(
                "one.user.workers.dev, two.user.workers.dev; one.user.workers.dev".to_string(),
            ),
            ..Profile::default()
        };

        let config = config_from_profile(&profile).expect("config");

        assert_eq!(
            config.cf_worker_domains(),
            &[
                "one.user.workers.dev".to_string(),
                "two.user.workers.dev".to_string(),
            ]
        );
    }

    #[test]
    fn rejects_malformed_dc_routes() {
        let profile = Profile {
            tg_ws_dc_ips: vec!["2:not-an-ip".to_string()],
            ..Profile::default()
        };

        assert!(config_from_profile(&profile).is_err());
    }

    #[test]
    fn disabling_cf_removes_both_automatic_and_custom_proxy_domains() {
        let profile = Profile {
            tg_ws_cf_proxy_enabled: false,
            tg_ws_cf_custom_enabled: true,
            tg_ws_default_domains: true,
            tg_ws_cf_domains: vec!["one.example".to_string()],
            ..Profile::default()
        };
        let config = config_from_profile(&profile).expect("config");

        assert!(!config.default_domains);
        assert!(config.cf_domains.is_empty());
    }

    #[test]
    fn uses_configured_local_host_in_telegram_link() {
        let profile = Profile {
            tg_ws_host: "127.0.0.1".to_string(),
            tg_ws_port: 1081,
            ..Profile::default()
        };
        let config = config_from_profile(&profile).expect("profile config");

        let link = format!(
            "tg://proxy?server={}&port={}&secret={}",
            config.link_host(),
            config.port,
            config.link_secret()
        );

        assert!(link.starts_with("tg://proxy?server=127.0.0.1&port=1081&secret="));
    }

    #[test]
    fn wildcard_listener_advertises_a_connectable_loopback_address() {
        let profile = Profile {
            tg_ws_host: "0.0.0.0".to_string(),
            ..Profile::default()
        };
        let config = config_from_profile(&profile).expect("profile config");

        assert_eq!(config.bind_host(), "0.0.0.0");
        assert_eq!(config.link_host(), "127.0.0.1");
    }

    #[test]
    fn carries_upstream_fronting_defaults() {
        let config = config_from_profile(&Profile::default()).expect("profile config");

        assert_eq!(config.fronting_domain.as_deref(), Some("sprinthost.ru"));
        assert_eq!(config.ip_fail_cooldown, 3600);
        assert_eq!(config.fronting_cooldown, 1800);
    }

    #[test]
    fn carries_force_test_dc_and_real_performance_settings() {
        let profile = Profile {
            tg_ws_force_test_dc: true,
            tg_ws_buf_kb: 512,
            tg_ws_pool_size: 7,
            tg_ws_verbose: true,
            ..Profile::default()
        };
        let config = config_from_profile(&profile).expect("profile config");

        assert!(config.force_test_dc);
        assert_eq!(config.buf_kb, 512);
        assert_eq!(config.pool_size, 7);
        assert!(config.verbose);
        assert!(!config.quiet);
    }

    #[test]
    fn embedded_runtime_stops_with_an_active_client_before_ui_deadline() {
        let profile = Profile {
            tg_ws_port: 0,
            tg_ws_pool_size: 0,
            tg_ws_cf_proxy_enabled: false,
            ..Profile::default()
        };
        let handle = spawn(&profile).expect("embedded tg-ws starts");
        let _client =
            std::net::TcpStream::connect(("127.0.0.1", handle.port)).expect("client connects");
        let failure = Arc::clone(&handle.failure);

        let started = std::time::Instant::now();
        handle.stop().expect("embedded tg-ws stops cleanly");

        assert!(
            started.elapsed() < Duration::from_secs(4),
            "shutdown must finish before the five-second UI deadline"
        );
        assert_eq!(
            failure.lock().expect("failure lock").as_deref(),
            None,
            "a requested stop must not be reported as an unexpected crash"
        );
    }

    #[test]
    fn embedded_runtime_stops_while_pool_and_domain_refresh_are_active() {
        let profile = Profile {
            tg_ws_port: 0,
            // Keep the production defaults enabled: the pool starts direct
            // WS handshakes while the CF list refresh resolves GitHub. These
            // were the uncancellable tasks behind the reported five-second
            // shutdown failure on Windows.
            tg_ws_pool_size: 4,
            tg_ws_cf_proxy_enabled: true,
            tg_ws_default_domains: true,
            ..Profile::default()
        };
        let handle = spawn(&profile).expect("embedded tg-ws starts without waiting for refresh");
        let failure = Arc::clone(&handle.failure);

        let started = std::time::Instant::now();
        handle
            .stop()
            .expect("embedded tg-ws stops while background I/O is active");

        assert!(
            started.elapsed() < Duration::from_secs(4),
            "background DNS and pool work must not outlive the UI deadline"
        );
        assert_eq!(failure.lock().expect("failure lock").as_deref(), None);
    }
}
