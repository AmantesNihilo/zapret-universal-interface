use crate::models::Profile;
use std::net::SocketAddr;
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::Duration;
use tg_ws_proxy_rs::{config::Config, default_domains, pool::WsPool, proxy};
use tokio::net::TcpListener;
use tokio::sync::{oneshot, Semaphore};

pub const ENGINE_NAME: &str = "tg-ws-proxy-rs";
pub const ENGINE_VERSION: &str = "1.5.0";

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
    let config = config_from_profile(profile)?;
    let host = config.host.clone();
    let port = config.port;
    let secret = config.link_secret();
    let link_host = advertised_link_host(&config.host);
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

fn config_from_profile(profile: &Profile) -> Result<Config, String> {
    let mut secret = normalize_secret(&profile.tg_ws_secret);
    if secret.is_empty() {
        secret = random_secret();
    }

    let cf_domains: Vec<String> = profile
        .tg_ws_cf_domains
        .iter()
        .map(|domain| domain.trim().to_string())
        .filter(|domain| !domain.is_empty())
        .collect();
    let cf_worker_domain = profile
        .tg_ws_cf_worker_domain
        .as_ref()
        .map(|domain| domain.trim().to_string())
        .filter(|domain| !domain.is_empty());

    let dc_ip = if cf_domains.is_empty() && !profile.tg_ws_default_domains {
        vec![
            (2, "149.154.167.220".to_string()),
            (4, "149.154.167.220".to_string()),
        ]
    } else {
        Vec::new()
    };

    Ok(Config {
        port: profile.tg_ws_port,
        host: profile.tg_ws_host.clone(),
        secret: Some(secret),
        listen_faketls_domain: None,
        dc_ip,
        buf_kb: 256,
        pool_size: 4,
        max_connections: None,
        verbose: false,
        skip_tls_verify: false,
        quiet: true,
        log_file: None,
        mtproto_proxies: Vec::new(),
        link_ip: None,
        cf_domains,
        cf_worker_domain,
        cf_priority: profile.tg_ws_cf_priority,
        cf_balance: profile.tg_ws_cf_balance,
        ws_connect_timeout: 10,
        ws_fail_probe_timeout: 2,
        ws_fail_cooldown: 30,
        ws_redirect_cooldown: 300,
        handshake_timeout: 10,
        tcp_fallback_timeout: 10,
        upstream_connect_timeout: 5,
        upstream_fail_cooldown: 60,
        cf_connect_timeout: 10,
        cf_fail_cooldown: 60,
        pool_max_age: 55,
        check: false,
        default_domains: profile.tg_ws_default_domains,
    })
}

async fn run_proxy(
    mut config: Config,
    ready_tx: std::sync::mpsc::Sender<Result<(), String>>,
    mut shutdown_rx: oneshot::Receiver<()>,
) -> Result<(), String> {
    let _ = rustls::crypto::ring::default_provider().install_default();

    if config.default_domains {
        let fetched = default_domains::fetch_default_domains().await;
        config.cf_domains.extend(fetched);
    }

    let addr: SocketAddr = format!("{}:{}", config.host, config.port)
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
    let pool = Arc::new(WsPool::new(
        config.pool_size,
        Duration::from_secs(config.pool_max_age),
    ));

    {
        let pool = pool.clone();
        let config = config.clone();
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
                let config = config.clone();
                let pool = pool.clone();
                tokio::spawn(async move {
                    let _permit = permit;
                    proxy::handle_client(stream, peer_addr, config, pool).await;
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

fn advertised_link_host(bind_host: &str) -> String {
    let host = bind_host.trim();
    match host {
        "" | "0.0.0.0" | "::" | "[::]" | "localhost" => "127.0.0.1".to_string(),
        _ => host
            .strip_prefix('[')
            .and_then(|value| value.strip_suffix(']'))
            .unwrap_or(host)
            .to_string(),
    }
}

fn is_valid_tg_secret(value: &str) -> bool {
    if hex::decode(value).is_err() {
        return false;
    }
    matches!(value.len(), 32 | 34) || (value.len() > 34 && value.starts_with("ee"))
}

fn random_secret() -> String {
    let bytes: [u8; 16] = rand::random();
    hex::encode(bytes)
}

fn soft_nofile_limit() -> usize {
    #[cfg(target_os = "linux")]
    {
        if let Ok(content) = std::fs::read_to_string("/proc/self/limits") {
            for line in content.lines() {
                if line.starts_with("Max open files") {
                    if let Some(soft_str) = line.split_whitespace().nth(3) {
                        if soft_str == "unlimited" {
                            return usize::MAX;
                        }
                        if let Ok(value) = soft_str.parse::<usize>() {
                            return value;
                        }
                    }
                }
            }
        }
    }

    1024
}

fn auto_max_connections(fd_limit: usize, pool_size: usize, dc_buckets: usize) -> usize {
    if fd_limit == usize::MAX {
        return 512;
    }
    let reserved = 1 + pool_size * dc_buckets * 2 + 32;
    (fd_limit.saturating_sub(reserved) / 2).max(4)
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

        assert_eq!(config.host, "127.0.0.1");
        assert_eq!(config.port, 1081);
        assert_eq!(
            config.dc_ip,
            vec![
                (2, "149.154.167.220".to_string()),
                (4, "149.154.167.220".to_string())
            ]
        );
        assert!(config.cf_domains.is_empty());
        assert!(!config.default_domains);
    }

    #[test]
    fn carries_cloudflare_routing_options() {
        let profile = Profile {
            tg_ws_default_domains: true,
            tg_ws_cf_domains: vec![
                " one.example ".to_string(),
                String::new(),
                "two.example".to_string(),
            ],
            tg_ws_cf_worker_domain: Some("worker.example".to_string()),
            tg_ws_cf_priority: true,
            tg_ws_cf_balance: true,
            ..Profile::default()
        };

        let config = config_from_profile(&profile).expect("profile config");

        assert!(config.default_domains);
        assert!(config.dc_ip.is_empty());
        assert_eq!(
            config.cf_domains,
            vec!["one.example".to_string(), "two.example".to_string()]
        );
        assert_eq!(config.cf_worker_domain, Some("worker.example".to_string()));
        assert!(config.cf_priority);
        assert!(config.cf_balance);
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
            advertised_link_host(&config.host),
            config.port,
            config.link_secret()
        );

        assert!(link.starts_with("tg://proxy?server=127.0.0.1&port=1081&secret="));
    }

    #[test]
    fn advertises_localhost_for_wildcard_bind() {
        assert_eq!(advertised_link_host("0.0.0.0"), "127.0.0.1");
        assert_eq!(advertised_link_host("::"), "127.0.0.1");
    }
}
