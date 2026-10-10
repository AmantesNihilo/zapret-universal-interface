use super::protocol::{self, HttpMode, HttpProbeOutcome, HttpProbeRequest};
use crate::models::{FailureStage, ProbeStatus, ProbeStepResult};
use futures_util::{SinkExt, StreamExt};
use quinn::{ClientConfig as QuinnClientConfig, Endpoint};
use rustls::RootCertStore;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::net::lookup_host;
use tokio_tungstenite::tungstenite::Message;

const PROBE_TIMEOUT: Duration = Duration::from_secs(8);

pub async fn discord_gateway_websocket(url: &str) -> SimpleProbeOutcome {
    let _ = rustls::crypto::ring::default_provider().install_default();
    let started = Instant::now();
    let connection =
        match tokio::time::timeout(PROBE_TIMEOUT, tokio_tungstenite::connect_async(url)).await {
            Ok(Ok(connection)) => connection,
            Ok(Err(error)) => {
                return SimpleProbeOutcome::failed(
                    started,
                    FailureStage::Websocket,
                    "websocket_upgrade_failed",
                    error.to_string(),
                )
            }
            Err(_) => {
                return SimpleProbeOutcome::failed(
                    started,
                    FailureStage::Websocket,
                    "websocket_timeout",
                    "WebSocket connection timed out",
                )
            }
        };
    let (mut socket, response) = connection;
    if response.status().as_u16() != 101 {
        return SimpleProbeOutcome::failed(
            started,
            FailureStage::Websocket,
            "websocket_upgrade_rejected",
            format!("Expected HTTP 101, got {}", response.status()),
        );
    }
    let message = match tokio::time::timeout(PROBE_TIMEOUT, socket.next()).await {
        Ok(Some(Ok(message))) => message,
        Ok(Some(Err(error))) => {
            return SimpleProbeOutcome::failed(
                started,
                FailureStage::Websocket,
                "websocket_read_failed",
                error.to_string(),
            )
        }
        Ok(None) => {
            return SimpleProbeOutcome::failed(
                started,
                FailureStage::Websocket,
                "websocket_closed",
                "Gateway closed before Hello",
            )
        }
        Err(_) => {
            return SimpleProbeOutcome::failed(
                started,
                FailureStage::Websocket,
                "websocket_hello_timeout",
                "Discord Gateway Hello timed out",
            )
        }
    };
    let payload = match message {
        Message::Text(text) => text.as_bytes().to_vec(),
        Message::Binary(bytes) => bytes.to_vec(),
        other => {
            return SimpleProbeOutcome::failed(
                started,
                FailureStage::Websocket,
                "websocket_unexpected_message",
                format!("Unexpected first message: {other:?}"),
            )
        }
    };
    let json: serde_json::Value = match serde_json::from_slice(&payload) {
        Ok(json) => json,
        Err(error) => {
            return SimpleProbeOutcome::failed(
                started,
                FailureStage::Content,
                "gateway_hello_invalid_json",
                error.to_string(),
            )
        }
    };
    if json.get("op").and_then(serde_json::Value::as_u64) != Some(10)
        || json
            .pointer("/d/heartbeat_interval")
            .and_then(serde_json::Value::as_u64)
            .is_none()
    {
        return SimpleProbeOutcome::failed(
            started,
            FailureStage::Content,
            "gateway_hello_invalid",
            "Discord Gateway did not send opcode 10 Hello",
        );
    }
    let _ = socket.close(None).await;
    SimpleProbeOutcome::passed(
        started,
        FailureStage::Websocket,
        "HTTP 101 and Discord Hello opcode 10 received",
    )
}

pub async fn websocket_echo(url: &str) -> SimpleProbeOutcome {
    let _ = rustls::crypto::ring::default_provider().install_default();
    let started = Instant::now();
    let connection =
        match tokio::time::timeout(PROBE_TIMEOUT, tokio_tungstenite::connect_async(url)).await {
            Ok(Ok(connection)) => connection,
            Ok(Err(error)) => {
                return SimpleProbeOutcome::failed(
                    started,
                    FailureStage::Websocket,
                    "websocket_upgrade_failed",
                    error.to_string(),
                )
            }
            Err(_) => {
                return SimpleProbeOutcome::failed(
                    started,
                    FailureStage::Websocket,
                    "websocket_timeout",
                    "WebSocket connection timed out",
                )
            }
        };
    let (mut socket, response) = connection;
    if response.status().as_u16() != 101 {
        return SimpleProbeOutcome::failed(
            started,
            FailureStage::Websocket,
            "websocket_upgrade_rejected",
            format!("Expected HTTP 101, got {}", response.status()),
        );
    }
    let nonce = format!("zui-{}", hex::encode(rand::random::<[u8; 8]>()));
    if let Err(error) = socket.send(Message::Text(nonce.clone())).await {
        return SimpleProbeOutcome::failed(
            started,
            FailureStage::Websocket,
            "websocket_send_failed",
            error.to_string(),
        );
    }
    let echoed = match tokio::time::timeout(PROBE_TIMEOUT, socket.next()).await {
        Ok(Some(Ok(Message::Text(text)))) => text.to_string(),
        Ok(Some(Ok(Message::Binary(bytes)))) => String::from_utf8_lossy(&bytes).into_owned(),
        Ok(Some(Ok(other))) => {
            return SimpleProbeOutcome::failed(
                started,
                FailureStage::Websocket,
                "websocket_unexpected_message",
                format!("Unexpected echo message: {other:?}"),
            )
        }
        Ok(Some(Err(error))) => {
            return SimpleProbeOutcome::failed(
                started,
                FailureStage::Websocket,
                "websocket_read_failed",
                error.to_string(),
            )
        }
        Ok(None) => {
            return SimpleProbeOutcome::failed(
                started,
                FailureStage::Websocket,
                "websocket_closed",
                "WebSocket closed before echo",
            )
        }
        Err(_) => {
            return SimpleProbeOutcome::failed(
                started,
                FailureStage::Websocket,
                "websocket_echo_timeout",
                "WebSocket echo timed out",
            )
        }
    };
    let _ = socket.close(None).await;
    if echoed != nonce {
        return SimpleProbeOutcome::failed(
            started,
            FailureStage::Content,
            "websocket_echo_mismatch",
            "WebSocket echo payload did not match",
        );
    }
    SimpleProbeOutcome::passed(
        started,
        FailureStage::Websocket,
        "HTTP 101 and bounded echo succeeded",
    )
}

pub async fn youtube_media_probe(seed: HttpProbeRequest) -> HttpProbeOutcome {
    let page = protocol::staged_http_probe(seed.clone()).await;
    if page.probe_status != ProbeStatus::Passed {
        return page;
    }
    let Some(media_url) = extract_googlevideo_url(&page.body) else {
        let mut result = page;
        result.probe_status = ProbeStatus::Inconclusive;
        result.failure_stage = Some(FailureStage::Content);
        result.reason_code = Some("youtube_media_url_unavailable".into());
        result.error = Some(
            "YouTube page did not expose a safe googlevideo media URL in the bounded response"
                .into(),
        );
        result.steps.push(ProbeStepResult {
            stage: FailureStage::Content,
            status: ProbeStatus::Inconclusive,
            latency_ms: None,
            detail: "Media URL was not present in the first 64 KiB".into(),
        });
        return result;
    };
    protocol::staged_http_probe(HttpProbeRequest {
        url: media_url,
        allowed_statuses: vec![200, 206],
        content_types: vec![
            "video/".into(),
            "audio/".into(),
            "application/octet-stream".into(),
        ],
        markers: Vec::new(),
        allowed_final_hosts: vec!["googlevideo.com".into()],
        mode: HttpMode::Auto,
        range: Some("bytes=0-32767".into()),
    })
    .await
}

pub async fn quic_http3_readiness(url: &str) -> SimpleProbeOutcome {
    let _ = rustls::crypto::ring::default_provider().install_default();
    let started = Instant::now();
    let parsed = match reqwest::Url::parse(url) {
        Ok(parsed) => parsed,
        Err(error) => {
            return SimpleProbeOutcome::inconclusive(
                started,
                FailureStage::Quic,
                "quic_url_invalid",
                error.to_string(),
            )
        }
    };
    let Some(host) = parsed.host_str().map(str::to_owned) else {
        return SimpleProbeOutcome::inconclusive(
            started,
            FailureStage::Quic,
            "quic_host_missing",
            "QUIC URL has no host",
        );
    };
    let addresses =
        match tokio::time::timeout(Duration::from_secs(4), lookup_host((host.as_str(), 443))).await
        {
            Ok(Ok(addresses)) => addresses.collect::<Vec<_>>(),
            Ok(Err(error)) => {
                return SimpleProbeOutcome::failed(
                    started,
                    FailureStage::Dns,
                    "quic_dns_failed",
                    error.to_string(),
                )
            }
            Err(_) => {
                return SimpleProbeOutcome::failed(
                    started,
                    FailureStage::Dns,
                    "quic_dns_timeout",
                    "QUIC DNS lookup timed out",
                )
            }
        };
    let Some(remote) = addresses.first().copied() else {
        return SimpleProbeOutcome::failed(
            started,
            FailureStage::Dns,
            "quic_dns_empty",
            "QUIC DNS returned no addresses",
        );
    };
    let bind = match remote.ip() {
        IpAddr::V4(_) => SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0),
        IpAddr::V6(_) => SocketAddr::new(IpAddr::V6(Ipv6Addr::UNSPECIFIED), 0),
    };
    let mut roots = RootCertStore::empty();
    roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    let mut tls = rustls::ClientConfig::builder()
        .with_root_certificates(roots)
        .with_no_client_auth();
    tls.alpn_protocols = vec![b"h3".to_vec()];
    let crypto = match quinn::crypto::rustls::QuicClientConfig::try_from(tls) {
        Ok(crypto) => crypto,
        Err(error) => {
            return SimpleProbeOutcome::inconclusive(
                started,
                FailureStage::Quic,
                "quic_client_unavailable",
                error.to_string(),
            )
        }
    };
    let mut endpoint = match Endpoint::client(bind) {
        Ok(endpoint) => endpoint,
        Err(error) => {
            return SimpleProbeOutcome::inconclusive(
                started,
                FailureStage::Quic,
                "quic_endpoint_unavailable",
                error.to_string(),
            )
        }
    };
    endpoint.set_default_client_config(QuinnClientConfig::new(Arc::new(crypto)));
    let connecting = match endpoint.connect(remote, &host) {
        Ok(connecting) => connecting,
        Err(error) => {
            return SimpleProbeOutcome::failed(
                started,
                FailureStage::Quic,
                "quic_connect_failed",
                error.to_string(),
            )
        }
    };
    match tokio::time::timeout(PROBE_TIMEOUT, connecting).await {
        Ok(Ok(connection)) => {
            connection.close(0u32.into(), b"ZUI readiness probe");
            endpoint.close(0u32.into(), b"ZUI readiness probe");
            SimpleProbeOutcome::passed(
                started,
                FailureStage::Quic,
                "QUIC handshake completed with h3 ALPN",
            )
        }
        Ok(Err(error)) => SimpleProbeOutcome::failed(
            started,
            FailureStage::Quic,
            "quic_handshake_failed",
            error.to_string(),
        ),
        Err(_) => SimpleProbeOutcome::failed(
            started,
            FailureStage::Quic,
            "quic_timeout",
            "QUIC handshake timed out",
        ),
    }
}

pub fn discord_voice_readiness() -> SimpleProbeOutcome {
    SimpleProbeOutcome {
        status: ProbeStatus::Inconclusive,
        failure_stage: Some(FailureStage::Udp),
        reason_code: Some("discord_voice_account_required".into()),
        latency_ms: 0,
        error: Some("Discord voice discovery requires an authenticated voice session; no account data was used".into()),
        steps: vec![ProbeStepResult {
            stage: FailureStage::Udp,
            status: ProbeStatus::Inconclusive,
            latency_ms: None,
            detail: "Skipped to avoid using an account token or personal data".into(),
        }],
    }
}

#[derive(Debug, Clone)]
pub struct SimpleProbeOutcome {
    pub status: ProbeStatus,
    pub failure_stage: Option<FailureStage>,
    pub reason_code: Option<String>,
    pub latency_ms: u128,
    pub error: Option<String>,
    pub steps: Vec<ProbeStepResult>,
}

impl SimpleProbeOutcome {
    fn passed(started: Instant, stage: FailureStage, detail: impl Into<String>) -> Self {
        Self {
            status: ProbeStatus::Passed,
            failure_stage: None,
            reason_code: None,
            latency_ms: started.elapsed().as_millis(),
            error: None,
            steps: vec![ProbeStepResult {
                stage,
                status: ProbeStatus::Passed,
                latency_ms: Some(started.elapsed().as_millis()),
                detail: detail.into(),
            }],
        }
    }

    fn failed(
        started: Instant,
        stage: FailureStage,
        reason: impl Into<String>,
        error: impl Into<String>,
    ) -> Self {
        let error = error.into();
        Self {
            status: ProbeStatus::Failed,
            failure_stage: Some(stage),
            reason_code: Some(reason.into()),
            latency_ms: started.elapsed().as_millis(),
            error: Some(error.clone()),
            steps: vec![ProbeStepResult {
                stage,
                status: ProbeStatus::Failed,
                latency_ms: Some(started.elapsed().as_millis()),
                detail: error,
            }],
        }
    }

    fn inconclusive(
        started: Instant,
        stage: FailureStage,
        reason: impl Into<String>,
        error: impl Into<String>,
    ) -> Self {
        let error = error.into();
        Self {
            status: ProbeStatus::Inconclusive,
            failure_stage: Some(stage),
            reason_code: Some(reason.into()),
            latency_ms: started.elapsed().as_millis(),
            error: Some(error.clone()),
            steps: vec![ProbeStepResult {
                stage,
                status: ProbeStatus::Inconclusive,
                latency_ms: Some(started.elapsed().as_millis()),
                detail: error,
            }],
        }
    }
}

fn extract_googlevideo_url(body: &[u8]) -> Option<String> {
    let text = String::from_utf8_lossy(body);
    for marker in ["\"url\":\"https://", "https://"] {
        let mut offset = 0;
        while let Some(index) = text[offset..].find(marker) {
            let start = offset
                + index
                + if marker.starts_with('"') {
                    "\"url\":\"".len()
                } else {
                    0
                };
            let tail = &text[start..];
            let end = tail.find(['"', '<', ' ']).unwrap_or(tail.len()).min(8192);
            let candidate = tail[..end]
                .replace("\\u0026", "&")
                .replace("\\u003d", "=")
                .replace("\\u0025", "%")
                .replace("\\/", "/");
            if let Ok(url) = reqwest::Url::parse(&candidate) {
                if url.scheme() == "https"
                    && url.host_str().is_some_and(|host| {
                        host.eq_ignore_ascii_case("googlevideo.com")
                            || host.to_ascii_lowercase().ends_with(".googlevideo.com")
                    })
                    && url.username().is_empty()
                    && url.password().is_none()
                {
                    return Some(url.to_string());
                }
            }
            offset = start.saturating_add(end).min(text.len());
            if offset >= text.len() {
                break;
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_only_safe_googlevideo_urls() {
        let body =
            br#"{"url":"https://r1---sn-test.googlevideo.com/videoplayback?id=1\u0026range=0-1"}"#;
        let url = extract_googlevideo_url(body).unwrap();
        assert!(url.starts_with("https://r1---sn-test.googlevideo.com/"));
        assert!(url.contains("&range=0-1"));
        assert!(extract_googlevideo_url(br#"{"url":"https://evil.test/video"}"#).is_none());
    }

    #[tokio::test]
    #[ignore = "requires live Discord access"]
    async fn live_discord_gateway_returns_hello() {
        let outcome =
            discord_gateway_websocket("wss://gateway.discord.gg/?v=10&encoding=json").await;
        assert_eq!(outcome.status, ProbeStatus::Passed, "{:?}", outcome.error);
    }

    #[tokio::test]
    #[ignore = "requires live QUIC access"]
    async fn live_cloudflare_quic_handshake_succeeds() {
        let outcome = quic_http3_readiness("https://www.cloudflare.com/").await;
        assert_eq!(outcome.status, ProbeStatus::Passed, "{:?}", outcome.error);
    }
}
