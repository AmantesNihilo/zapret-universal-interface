use crate::models::{FailureStage, ProbeStatus, ProbeStepResult};
use reqwest::{header, tls::Version, Client, Version as HttpVersion};
use rustls::{ClientConfig, RootCertStore};
use rustls_pki_types::ServerName;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::net::{lookup_host, TcpStream};
use tokio_rustls::TlsConnector;

pub const MAX_RESPONSE_BYTES: usize = 64 * 1024;
const CONNECT_TIMEOUT: Duration = Duration::from_secs(4);
const DNS_TIMEOUT: Duration = Duration::from_secs(8);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(8);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HttpMode {
    Auto,
    Http1,
    Http2,
    Tls12,
    Tls13,
}

#[derive(Debug, Clone)]
pub struct HttpProbeRequest {
    pub url: String,
    pub allowed_statuses: Vec<u16>,
    pub content_types: Vec<String>,
    pub markers: Vec<String>,
    pub allowed_final_hosts: Vec<String>,
    pub mode: HttpMode,
    pub range: Option<String>,
}

#[derive(Debug, Clone)]
pub struct HttpProbeOutcome {
    pub probe_status: ProbeStatus,
    pub failure_stage: Option<FailureStage>,
    pub reason_code: Option<String>,
    pub status: Option<u16>,
    pub latency_ms: u128,
    pub final_url: Option<String>,
    pub content_type: Option<String>,
    pub negotiated_protocol: Option<String>,
    pub tls_version: Option<String>,
    pub alpn: Option<String>,
    pub bytes_read: u64,
    pub body: Vec<u8>,
    pub error: Option<String>,
    pub steps: Vec<ProbeStepResult>,
}

impl HttpProbeOutcome {
    fn failed(
        started: Instant,
        stage: FailureStage,
        reason_code: impl Into<String>,
        error: impl Into<String>,
        steps: Vec<ProbeStepResult>,
    ) -> Self {
        Self {
            probe_status: ProbeStatus::Failed,
            failure_stage: Some(stage),
            reason_code: Some(reason_code.into()),
            status: None,
            latency_ms: started.elapsed().as_millis(),
            final_url: None,
            content_type: None,
            negotiated_protocol: None,
            tls_version: None,
            alpn: None,
            bytes_read: 0,
            body: Vec::new(),
            error: Some(error.into()),
            steps,
        }
    }
}

pub async fn staged_http_probe(request: HttpProbeRequest) -> HttpProbeOutcome {
    let _ = rustls::crypto::ring::default_provider().install_default();
    let started = Instant::now();
    let mut steps = Vec::new();
    let parsed = match reqwest::Url::parse(&request.url) {
        Ok(parsed) => parsed,
        Err(error) => {
            return HttpProbeOutcome::failed(
                started,
                FailureStage::Http,
                "url_invalid",
                error.to_string(),
                steps,
            )
        }
    };
    let Some(host) = parsed.host_str().map(str::to_owned) else {
        return HttpProbeOutcome::failed(
            started,
            FailureStage::Dns,
            "host_missing",
            "URL has no host",
            steps,
        );
    };
    let port = parsed.port_or_known_default().unwrap_or(443);

    let dns_started = Instant::now();
    let addresses =
        match tokio::time::timeout(DNS_TIMEOUT, lookup_host((host.as_str(), port))).await {
            Ok(Ok(addresses)) => addresses.collect::<Vec<_>>(),
            Ok(Err(error)) => {
                steps.push(step(
                    FailureStage::Dns,
                    ProbeStatus::Failed,
                    dns_started,
                    error.to_string(),
                ));
                return HttpProbeOutcome::failed(
                    started,
                    FailureStage::Dns,
                    "dns_failed",
                    error.to_string(),
                    steps,
                );
            }
            Err(_) => {
                steps.push(step(
                    FailureStage::Dns,
                    ProbeStatus::Failed,
                    dns_started,
                    "DNS timeout",
                ));
                return HttpProbeOutcome::failed(
                    started,
                    FailureStage::Dns,
                    "dns_timeout",
                    "DNS lookup timed out",
                    steps,
                );
            }
        };
    if addresses.is_empty() {
        steps.push(step(
            FailureStage::Dns,
            ProbeStatus::Failed,
            dns_started,
            "No addresses",
        ));
        return HttpProbeOutcome::failed(
            started,
            FailureStage::Dns,
            "dns_empty",
            "DNS returned no addresses",
            steps,
        );
    }
    steps.push(step(
        FailureStage::Dns,
        ProbeStatus::Passed,
        dns_started,
        format!("{} address(es)", addresses.len()),
    ));

    let tcp_started = Instant::now();
    let tcp = match connect_any(&addresses).await {
        Ok(stream) => stream,
        Err(error) => {
            steps.push(step(
                FailureStage::Tcp,
                ProbeStatus::Failed,
                tcp_started,
                error.clone(),
            ));
            return HttpProbeOutcome::failed(
                started,
                FailureStage::Tcp,
                "tcp_connect_failed",
                error,
                steps,
            );
        }
    };
    steps.push(step(
        FailureStage::Tcp,
        ProbeStatus::Passed,
        tcp_started,
        format!("{}:{}", host, port),
    ));

    let mut tls_version = None;
    let mut alpn = None;
    if parsed.scheme() == "https" {
        let tls_started = Instant::now();
        match tls_handshake(tcp, &host, request.mode).await {
            Ok(details) => {
                tls_version = details.version.clone();
                alpn = details.alpn.clone();
                steps.push(step(
                    FailureStage::Tls,
                    ProbeStatus::Passed,
                    tls_started,
                    format!(
                        "{} / {}",
                        details.version.unwrap_or_else(|| "TLS".into()),
                        details.alpn.unwrap_or_else(|| "no ALPN".into())
                    ),
                ));
            }
            Err(error) => {
                steps.push(step(
                    FailureStage::Tls,
                    ProbeStatus::Failed,
                    tls_started,
                    error.clone(),
                ));
                return HttpProbeOutcome::failed(
                    started,
                    FailureStage::Tls,
                    "tls_handshake_failed",
                    error,
                    steps,
                );
            }
        }
    } else {
        drop(tcp);
    }

    let client = match build_client(request.mode) {
        Ok(client) => client,
        Err(error) => {
            return HttpProbeOutcome::failed(
                started,
                FailureStage::Internal,
                "http_client_unavailable",
                error.to_string(),
                steps,
            )
        }
    };
    let mut probe_url = parsed;
    probe_url
        .query_pairs_mut()
        .append_pair("zui_probe", &hex::encode(rand::random::<[u8; 8]>()));
    let mut builder = client
        .get(probe_url)
        .header(header::CACHE_CONTROL, "no-cache, no-store, max-age=0")
        .header(header::PRAGMA, "no-cache")
        .header(header::ACCEPT, "*/*");
    if let Some(range) = &request.range {
        builder = builder.header(header::RANGE, range);
    }

    let http_started = Instant::now();
    let mut response = match builder.send().await {
        Ok(response) => response,
        Err(error) => {
            let stage = classify_reqwest_failure(&error);
            let detail = safe_reqwest_error(&error);
            steps.push(step(
                stage,
                ProbeStatus::Failed,
                http_started,
                detail.clone(),
            ));
            return HttpProbeOutcome::failed(
                started,
                stage,
                reqwest_reason_code(&error),
                detail,
                steps,
            );
        }
    };
    let status = response.status().as_u16();
    let final_url = sanitize_url(response.url());
    let protocol = http_version_label(response.version()).to_string();
    let content_type = response
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);

    if !request.allowed_statuses.contains(&status) {
        let (probe_status, reason) = match status {
            401 => (ProbeStatus::Failed, "http_unauthorized"),
            403 => (ProbeStatus::Failed, "http_forbidden"),
            404 => (ProbeStatus::Failed, "http_not_found"),
            429 => (ProbeStatus::Inconclusive, "http_rate_limited"),
            _ => (ProbeStatus::Failed, "http_unexpected_status"),
        };
        steps.push(step(
            FailureStage::Http,
            probe_status,
            http_started,
            format!("HTTP {status}"),
        ));
        return HttpProbeOutcome {
            probe_status,
            failure_stage: Some(FailureStage::Http),
            reason_code: Some(reason.into()),
            status: Some(status),
            latency_ms: started.elapsed().as_millis(),
            final_url: Some(final_url),
            content_type,
            negotiated_protocol: Some(protocol),
            tls_version,
            alpn,
            bytes_read: 0,
            body: Vec::new(),
            error: Some(format!("Unexpected HTTP status {status}")),
            steps,
        };
    }
    if !final_host_allowed(response.url().host_str(), &request.allowed_final_hosts) {
        steps.push(step(
            FailureStage::Http,
            ProbeStatus::Failed,
            http_started,
            format!("Unexpected redirect to {final_url}"),
        ));
        return outcome_after_response(
            started,
            ProbeStatus::Failed,
            FailureStage::Http,
            "redirect_host_mismatch",
            format!("Unexpected final URL: {final_url}"),
            status,
            final_url,
            content_type,
            protocol,
            tls_version,
            alpn,
            Vec::new(),
            steps,
        );
    }
    if request.mode == HttpMode::Http1 && response.version() != HttpVersion::HTTP_11 {
        steps.push(step(
            FailureStage::Http,
            ProbeStatus::Failed,
            http_started,
            format!("Negotiated {protocol}"),
        ));
        return outcome_after_response(
            started,
            ProbeStatus::Failed,
            FailureStage::Http,
            "http1_not_negotiated",
            format!("Expected HTTP/1.1, got {protocol}"),
            status,
            final_url,
            content_type,
            protocol,
            tls_version,
            alpn,
            Vec::new(),
            steps,
        );
    }
    if request.mode == HttpMode::Http2 && response.version() != HttpVersion::HTTP_2 {
        steps.push(step(
            FailureStage::Http,
            ProbeStatus::Failed,
            http_started,
            format!("Negotiated {protocol}"),
        ));
        return outcome_after_response(
            started,
            ProbeStatus::Failed,
            FailureStage::Http,
            "http2_not_negotiated",
            format!("Expected HTTP/2, got {protocol}"),
            status,
            final_url,
            content_type,
            protocol,
            tls_version,
            alpn,
            Vec::new(),
            steps,
        );
    }
    steps.push(step(
        FailureStage::Http,
        ProbeStatus::Passed,
        http_started,
        format!("HTTP {status} / {protocol}"),
    ));

    let content_started = Instant::now();
    let body = match read_limited(&mut response).await {
        Ok(body) => body,
        Err(error) => {
            steps.push(step(
                FailureStage::Content,
                ProbeStatus::Failed,
                content_started,
                error.to_string(),
            ));
            return outcome_after_response(
                started,
                ProbeStatus::Failed,
                FailureStage::Content,
                "content_read_failed",
                error.to_string(),
                status,
                final_url,
                content_type,
                protocol,
                tls_version,
                alpn,
                Vec::new(),
                steps,
            );
        }
    };
    let body_text = String::from_utf8_lossy(&body).to_ascii_lowercase();
    if let Some(signature) = blocked_page_signature(&body_text) {
        steps.push(step(
            FailureStage::Content,
            ProbeStatus::Failed,
            content_started,
            format!("Block page: {signature}"),
        ));
        return outcome_with_content(
            started,
            ProbeStatus::Failed,
            "blocked_page",
            format!("Detected block page: {signature}"),
            status,
            final_url,
            content_type,
            protocol,
            tls_version,
            alpn,
            body,
            steps,
        );
    }
    if let Some(signature) = captcha_signature(&body_text) {
        steps.push(step(
            FailureStage::Content,
            ProbeStatus::Inconclusive,
            content_started,
            format!("CAPTCHA: {signature}"),
        ));
        return outcome_with_content(
            started,
            ProbeStatus::Inconclusive,
            "captcha_detected",
            format!("CAPTCHA or challenge detected: {signature}"),
            status,
            final_url,
            content_type,
            protocol,
            tls_version,
            alpn,
            body,
            steps,
        );
    }
    if !request.content_types.is_empty()
        && !request.content_types.iter().any(|expected| {
            content_type.as_deref().is_some_and(|actual| {
                actual
                    .to_ascii_lowercase()
                    .starts_with(&expected.to_ascii_lowercase())
            })
        })
    {
        let actual = content_type.clone().unwrap_or_else(|| "missing".into());
        steps.push(step(
            FailureStage::Content,
            ProbeStatus::Failed,
            content_started,
            format!("Content-Type {actual}"),
        ));
        return outcome_with_content(
            started,
            ProbeStatus::Failed,
            "content_type_mismatch",
            format!("Unexpected Content-Type: {actual}"),
            status,
            final_url,
            content_type,
            protocol,
            tls_version,
            alpn,
            body,
            steps,
        );
    }
    if !request.markers.is_empty()
        && !request
            .markers
            .iter()
            .any(|marker| body_text.contains(&marker.to_ascii_lowercase()))
    {
        steps.push(step(
            FailureStage::Content,
            ProbeStatus::Failed,
            content_started,
            "Expected response marker is missing",
        ));
        return outcome_with_content(
            started,
            ProbeStatus::Failed,
            "content_marker_missing",
            "Expected response marker is missing".into(),
            status,
            final_url,
            content_type,
            protocol,
            tls_version,
            alpn,
            body,
            steps,
        );
    }
    steps.push(step(
        FailureStage::Content,
        ProbeStatus::Passed,
        content_started,
        format!("{} bytes inspected", body.len()),
    ));

    HttpProbeOutcome {
        probe_status: ProbeStatus::Passed,
        failure_stage: None,
        reason_code: None,
        status: Some(status),
        latency_ms: started.elapsed().as_millis(),
        final_url: Some(final_url),
        content_type,
        negotiated_protocol: Some(protocol),
        tls_version,
        alpn,
        bytes_read: body.len() as u64,
        body,
        error: None,
        steps,
    }
}

#[allow(clippy::too_many_arguments)]
fn outcome_with_content(
    started: Instant,
    probe_status: ProbeStatus,
    reason: &str,
    error: String,
    status: u16,
    final_url: String,
    content_type: Option<String>,
    protocol: String,
    tls_version: Option<String>,
    alpn: Option<String>,
    body: Vec<u8>,
    steps: Vec<ProbeStepResult>,
) -> HttpProbeOutcome {
    outcome_after_response(
        started,
        probe_status,
        FailureStage::Content,
        reason,
        error,
        status,
        final_url,
        content_type,
        protocol,
        tls_version,
        alpn,
        body,
        steps,
    )
}

#[allow(clippy::too_many_arguments)]
fn outcome_after_response(
    started: Instant,
    probe_status: ProbeStatus,
    failure_stage: FailureStage,
    reason: &str,
    error: String,
    status: u16,
    final_url: String,
    content_type: Option<String>,
    protocol: String,
    tls_version: Option<String>,
    alpn: Option<String>,
    body: Vec<u8>,
    steps: Vec<ProbeStepResult>,
) -> HttpProbeOutcome {
    HttpProbeOutcome {
        probe_status,
        failure_stage: Some(failure_stage),
        reason_code: Some(reason.into()),
        status: Some(status),
        latency_ms: started.elapsed().as_millis(),
        final_url: Some(final_url),
        content_type,
        negotiated_protocol: Some(protocol),
        tls_version,
        alpn,
        bytes_read: body.len() as u64,
        body,
        error: Some(error),
        steps,
    }
}

async fn connect_any(addresses: &[SocketAddr]) -> Result<TcpStream, String> {
    let mut last_error = "No address was reachable".to_string();
    for address in addresses.iter().take(4) {
        match tokio::time::timeout(CONNECT_TIMEOUT, TcpStream::connect(address)).await {
            Ok(Ok(stream)) => return Ok(stream),
            Ok(Err(error)) => last_error = error.to_string(),
            Err(_) => last_error = "TCP connect timed out".into(),
        }
    }
    Err(last_error)
}

struct TlsDetails {
    version: Option<String>,
    alpn: Option<String>,
}

async fn tls_handshake(tcp: TcpStream, host: &str, mode: HttpMode) -> Result<TlsDetails, String> {
    let roots = RootCertStore::from_iter(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    let builder = match mode {
        HttpMode::Tls12 => ClientConfig::builder_with_protocol_versions(&[&rustls::version::TLS12]),
        HttpMode::Tls13 => ClientConfig::builder_with_protocol_versions(&[&rustls::version::TLS13]),
        _ => ClientConfig::builder(),
    };
    let mut config = builder.with_root_certificates(roots).with_no_client_auth();
    config.alpn_protocols = match mode {
        HttpMode::Http1 => vec![b"http/1.1".to_vec()],
        HttpMode::Http2 => vec![b"h2".to_vec()],
        _ => vec![b"h2".to_vec(), b"http/1.1".to_vec()],
    };
    let server_name = ServerName::try_from(host.to_owned()).map_err(|error| error.to_string())?;
    let stream = tokio::time::timeout(
        REQUEST_TIMEOUT,
        TlsConnector::from(Arc::new(config)).connect(server_name, tcp),
    )
    .await
    .map_err(|_| "TLS handshake timed out".to_string())?
    .map_err(|error| error.to_string())?;
    let connection = stream.get_ref().1;
    Ok(TlsDetails {
        version: connection.protocol_version().map(|version| match version {
            rustls::ProtocolVersion::TLSv1_2 => "TLS 1.2".to_string(),
            rustls::ProtocolVersion::TLSv1_3 => "TLS 1.3".to_string(),
            other => format!("{other:?}"),
        }),
        alpn: connection
            .alpn_protocol()
            .map(|value| String::from_utf8_lossy(value).into_owned()),
    })
}

fn build_client(mode: HttpMode) -> Result<Client, reqwest::Error> {
    let builder = Client::builder()
        .timeout(REQUEST_TIMEOUT)
        .redirect(reqwest::redirect::Policy::limited(4))
        .user_agent(concat!("ZUI/", env!("CARGO_PKG_VERSION")));
    let builder = match mode {
        HttpMode::Http1 => builder.http1_only(),
        HttpMode::Http2 => builder.http2_prior_knowledge(),
        HttpMode::Tls12 => builder
            .min_tls_version(Version::TLS_1_2)
            .max_tls_version(Version::TLS_1_2),
        HttpMode::Tls13 => builder
            .min_tls_version(Version::TLS_1_3)
            .max_tls_version(Version::TLS_1_3),
        HttpMode::Auto => builder,
    };
    builder.build()
}

async fn read_limited(response: &mut reqwest::Response) -> Result<Vec<u8>, reqwest::Error> {
    let mut body = Vec::with_capacity(8192);
    while let Some(chunk) = response.chunk().await? {
        let remaining = MAX_RESPONSE_BYTES.saturating_sub(body.len());
        if remaining == 0 {
            break;
        }
        body.extend_from_slice(&chunk[..chunk.len().min(remaining)]);
        if body.len() >= MAX_RESPONSE_BYTES {
            break;
        }
    }
    Ok(body)
}

fn final_host_allowed(host: Option<&str>, allowed: &[String]) -> bool {
    let Some(host) = host else {
        return false;
    };
    allowed.iter().any(|candidate| {
        host.eq_ignore_ascii_case(candidate)
            || host
                .to_ascii_lowercase()
                .ends_with(&format!(".{}", candidate.to_ascii_lowercase()))
    })
}

fn blocked_page_signature(body: &str) -> Option<&'static str> {
    let body = body.to_ascii_lowercase();
    [
        "access to this site has been blocked",
        "internet access is blocked",
        "website is blocked",
        "доступ к ресурсу ограничен",
        "доступ к сайту заблокирован",
        "captive portal",
    ]
    .into_iter()
    .find(|signature| body.contains(signature))
}

fn captcha_signature(body: &str) -> Option<&'static str> {
    let body = body.to_ascii_lowercase();
    [
        "g-recaptcha",
        "hcaptcha",
        "cf-chl-",
        "captcha challenge",
        "verify you are human",
    ]
    .into_iter()
    .find(|signature| body.contains(signature))
}

fn step(
    stage: FailureStage,
    status: ProbeStatus,
    started: Instant,
    detail: impl Into<String>,
) -> ProbeStepResult {
    ProbeStepResult {
        stage,
        status,
        latency_ms: Some(started.elapsed().as_millis()),
        detail: detail.into(),
    }
}

fn http_version_label(version: HttpVersion) -> &'static str {
    match version {
        HttpVersion::HTTP_09 => "HTTP/0.9",
        HttpVersion::HTTP_10 => "HTTP/1.0",
        HttpVersion::HTTP_11 => "HTTP/1.1",
        HttpVersion::HTTP_2 => "HTTP/2",
        HttpVersion::HTTP_3 => "HTTP/3",
        _ => "HTTP",
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

fn sanitize_url(url: &reqwest::Url) -> String {
    let mut sanitized = url.clone();
    if sanitized.host_str().is_some_and(|host| {
        host.eq_ignore_ascii_case("googlevideo.com")
            || host.to_ascii_lowercase().ends_with(".googlevideo.com")
    }) {
        sanitized.set_query(None);
        sanitized.set_fragment(None);
        return sanitized.to_string();
    }
    let pairs = sanitized
        .query_pairs()
        .filter(|(key, _)| key != "zui_probe")
        .map(|(key, value)| (key.into_owned(), value.into_owned()))
        .collect::<Vec<_>>();
    sanitized.set_query(None);
    if !pairs.is_empty() {
        sanitized.query_pairs_mut().extend_pairs(pairs);
    }
    sanitized.to_string()
}

fn safe_reqwest_error(error: &reqwest::Error) -> String {
    let detail = error.to_string();
    error
        .url()
        .map(|url| detail.replace(url.as_str(), &sanitize_url(url)))
        .unwrap_or(detail)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    async fn local_probe(status: u16, content_type: &str, body: &str) -> HttpProbeOutcome {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let content_type = content_type.to_string();
        let body = body.to_string();
        tokio::spawn(async move {
            let (preflight, _) = listener.accept().await.unwrap();
            drop(preflight);
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut request = [0u8; 2048];
            let _ = stream.read(&mut request).await;
            let reason = if status == 200 { "OK" } else { "Error" };
            let response = format!("HTTP/1.1 {status} {reason}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
            stream.write_all(response.as_bytes()).await.unwrap();
        });
        staged_http_probe(HttpProbeRequest {
            url: format!("http://{address}/probe"),
            allowed_statuses: vec![200],
            content_types: vec!["text/html".into()],
            markers: vec!["expected-marker".into()],
            allowed_final_hosts: vec!["127.0.0.1".into()],
            mode: HttpMode::Http1,
            range: None,
        })
        .await
    }

    #[test]
    fn recognizes_block_pages_and_captcha() {
        assert_eq!(
            blocked_page_signature("<h1>Website is blocked</h1>"),
            Some("website is blocked")
        );
        assert_eq!(
            captcha_signature("<div class=g-recaptcha>"),
            Some("g-recaptcha")
        );
        assert_eq!(blocked_page_signature("normal youtube page"), None);
    }

    #[test]
    fn final_host_allows_subdomains_only() {
        let allowed = vec!["youtube.com".to_string()];
        assert!(final_host_allowed(Some("www.youtube.com"), &allowed));
        assert!(!final_host_allowed(Some("youtube.com.evil.test"), &allowed));
    }

    #[test]
    fn strips_probe_and_signed_media_queries_from_saved_urls() {
        let normal = reqwest::Url::parse("https://example.com/path?a=1&zui_probe=secret").unwrap();
        assert_eq!(sanitize_url(&normal), "https://example.com/path?a=1");
        let media = reqwest::Url::parse("https://r1.googlevideo.com/videoplayback?sig=secret&id=1")
            .unwrap();
        assert_eq!(
            sanitize_url(&media),
            "https://r1.googlevideo.com/videoplayback"
        );
    }

    #[tokio::test]
    async fn http_200_block_page_is_not_successful() {
        let outcome = local_probe(
            200,
            "text/html",
            "<h1>Website is blocked</h1> expected-marker",
        )
        .await;
        assert_eq!(outcome.probe_status, ProbeStatus::Failed);
        assert_eq!(outcome.reason_code.as_deref(), Some("blocked_page"));
    }

    #[tokio::test]
    async fn forbidden_status_is_not_successful() {
        let outcome = local_probe(403, "text/html", "expected-marker").await;
        assert_eq!(outcome.probe_status, ProbeStatus::Failed);
        assert_eq!(outcome.reason_code.as_deref(), Some("http_forbidden"));
    }

    #[tokio::test]
    async fn expected_bounded_response_passes() {
        let outcome = local_probe(200, "text/html; charset=utf-8", "expected-marker").await;
        assert_eq!(outcome.probe_status, ProbeStatus::Passed);
        assert_eq!(outcome.negotiated_protocol.as_deref(), Some("HTTP/1.1"));
        assert!(outcome.bytes_read <= MAX_RESPONSE_BYTES as u64);
    }

    #[tokio::test]
    #[ignore = "requires live network access"]
    async fn live_cloudflare_http2_probe_negotiates_http2() {
        let outcome = staged_http_probe(HttpProbeRequest {
            url: "https://www.cloudflare.com/cdn-cgi/trace".into(),
            allowed_statuses: vec![200],
            content_types: vec!["text/plain".into()],
            markers: vec!["http=".into()],
            allowed_final_hosts: vec!["cloudflare.com".into()],
            mode: HttpMode::Http2,
            range: None,
        })
        .await;
        assert_eq!(
            outcome.probe_status,
            ProbeStatus::Passed,
            "{:?}",
            outcome.error
        );
        assert_eq!(outcome.negotiated_protocol.as_deref(), Some("HTTP/2"));
    }
}
