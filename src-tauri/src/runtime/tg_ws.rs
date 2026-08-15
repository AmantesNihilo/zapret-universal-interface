use crate::models::{Profile, TgWsConnectivityKind, TgWsConnectivityProbe, TgWsConnectivityReport};
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::sync::OnceLock;
use std::thread::JoinHandle;
use std::time::Duration;
use tauri::Manager;
use tg_ws_proxy_rs::{
    check,
    config::Config,
    default_domains,
    limits::{auto_max_connections, soft_nofile_limit},
    pool::WsPool,
    proxy,
    runtime::Runtime,
};
use tokio::net::TcpListener;
use tokio::sync::{oneshot, Semaphore};
use tracing_subscriber::fmt::MakeWriter;

pub const ENGINE_NAME: &str = "tg-ws-proxy-rs";
pub const ENGINE_VERSION: &str = "2.2.5-zui.1";

static TG_WS_TRACE_SENDER: OnceLock<std::sync::mpsc::Sender<String>> = OnceLock::new();
static TG_WS_VERBOSE: AtomicBool = AtomicBool::new(false);

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
            if let Some(sender) = TG_WS_TRACE_SENDER.get() {
                let _ = sender.send(message);
            }
        }
    }
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
    let (sender, receiver) = std::sync::mpsc::channel::<String>();
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
    thread: JoinHandle<()>,
    pub host: String,
    pub port: u16,
    pub link: String,
}

impl TgWsRuntimeHandle {
    pub fn is_running(&self) -> bool {
        !self.thread.is_finished()
    }

    pub fn stop(mut self) -> Result<(), String> {
        if let Some(shutdown_tx) = self.shutdown_tx.take() {
            let _ = shutdown_tx.send(());
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
    let port = config.port;
    let secret = config.link_secret();
    let link_host = config.link_host();
    let link = format!(
        "tg://proxy?server={}&port={}&secret={}",
        link_host, config.port, secret
    );

    let (ready_tx, ready_rx) = std::sync::mpsc::channel::<Result<(), String>>();
    let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();
    let thread_config = config.clone();

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
                    let _ = ready_tx.send(Err(error.to_string()));
                    return;
                }
            };

            runtime.block_on(async move {
                let result = run_proxy(thread_config, ready_tx, shutdown_rx).await;
                if let Err(error) = result {
                    tracing::error!("tg-ws runtime stopped with error: {}", error);
                }
            });
        })
        .map_err(|error| error.to_string())?;

    match ready_rx.recv_timeout(Duration::from_secs(10)) {
        Ok(Ok(())) => Ok(TgWsRuntimeHandle {
            shutdown_tx: Some(shutdown_tx),
            thread,
            host,
            port,
            link,
        }),
        Ok(Err(error)) => {
            let _ = thread.join();
            Err(error)
        }
        Err(error) => {
            let _ = shutdown_tx.send(());
            let _ = thread.join();
            Err(format!("tg-ws runtime did not start: {error}"))
        }
    }
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

    Ok(Config {
        port: profile.tg_ws_port,
        host: Some(profile.tg_ws_host.clone()),
        secrets: vec![secret],
        listen_faketls_domain: None,
        dc_ip,
        buf_kb: profile.tg_ws_buf_kb.max(4),
        pool_size: profile.tg_ws_pool_size,
        force_test_dc: profile.tg_ws_force_test_dc,
        max_connections: None,
        verbose: profile.tg_ws_verbose,
        skip_tls_verify: false,
        quiet: !profile.tg_ws_verbose,
        log_file: None,
        mtproto_proxies: Vec::new(),
        link_ip: Some(advertised_link_host(&profile.tg_ws_host)),
        cf_domains,
        cf_worker_domains,
        cf_priority: profile.tg_ws_cf_priority,
        cf_balance: profile.tg_ws_cf_balance,
        ws_connect_timeout: 10,
        ws_fail_probe_timeout: 2,
        ws_fail_cooldown: 30,
        ws_redirect_cooldown: 300,
        ip_fail_cooldown: 3600,
        handshake_timeout: 10,
        tcp_fallback_timeout: 10,
        upstream_connect_timeout: 5,
        upstream_fail_cooldown: 60,
        cf_connect_timeout: 10,
        cf_fail_cooldown: 60,
        fronting_domain,
        fronting_cooldown: 1800,
        fronting_fail_cooldown: 60,
        pool_max_age: 55,
        check: false,
        default_domains: profile.tg_ws_cf_proxy_enabled
            && !profile.tg_ws_cf_custom_enabled
            && profile.tg_ws_default_domains,
        outbound_proxy: None,
        no_outbound_proxy: true,
        no_proxy: None,
    }
    .with_defaults())
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

async fn run_proxy(
    mut config: Config,
    ready_tx: std::sync::mpsc::Sender<Result<(), String>>,
    mut shutdown_rx: oneshot::Receiver<()>,
) -> Result<(), String> {
    let _ = rustls::crypto::ring::default_provider().install_default();

    let outbound = config.outbound_connector()?;
    let runtime = Arc::new(Runtime::new(outbound).with_fronting(
        config.fronting_domain.clone(),
        Duration::from_secs(config.fronting_cooldown),
    ));

    if config.default_domains {
        let fetched =
            default_domains::fetch_default_domains_with_outbound(runtime.outbound()).await;
        config.cf_domains.extend(fetched);
    }

    let bind_host = config.bind_host();
    let addr: SocketAddr = format!("{}:{}", bind_host, config.port)
        .parse::<SocketAddr>()
        .map_err(|error| error.to_string())?;
    let listener = TcpListener::bind(addr)
        .await
        .map_err(|error| format!("cannot bind {addr}: {error}"))?;

    let fd_limit = soft_nofile_limit();
    let dc_buckets = config.dc_redirects().len().max(1) * 2;
    let max_connections = config
        .max_connections
        .unwrap_or_else(|| auto_max_connections(fd_limit, config.pool_size, dc_buckets));
    let pool = Arc::new(WsPool::with_runtime(
        config.pool_size,
        Duration::from_secs(config.pool_max_age),
        Arc::clone(&runtime),
    ));
    let config = Arc::new(config);

    {
        let pool = pool.clone();
        let config = Arc::clone(&config);
        tokio::spawn(async move {
            pool.warmup(&config).await;
        });
    }

    let _ = ready_tx.send(Ok(()));
    let semaphore = Arc::new(Semaphore::new(max_connections));

    loop {
        let permit = tokio::select! {
            _ = &mut shutdown_rx => break,
            permit = Arc::clone(&semaphore).acquire_owned() => {
                permit.map_err(|_| "tg-ws runtime semaphore closed".to_string())?
            }
        };

        let accepted = tokio::select! {
            _ = &mut shutdown_rx => break,
            accepted = listener.accept() => accepted,
        };

        match accepted {
            Ok((stream, peer_addr)) => {
                let config = Arc::clone(&config);
                let pool = pool.clone();
                let runtime = Arc::clone(&runtime);
                tokio::spawn(async move {
                    let _permit = permit;
                    proxy::handle_client_with_runtime(stream, peer_addr, config, pool, runtime)
                        .await;
                });
            }
            Err(error) => {
                tracing::warn!("tg-ws accept error: {}", error);
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        }
    }

    Ok(())
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
}
