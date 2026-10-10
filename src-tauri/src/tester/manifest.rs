use crate::models::{ProbeCapability, TestTargetConfig};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProbeKind {
    Http,
    Http1,
    Http2,
    Tls12,
    Tls13,
    DiscordGatewayApi,
    DiscordGatewayWebsocket,
    DiscordVoiceReadiness,
    YoutubeWeb,
    YoutubeMedia,
    ControlledJson,
    ControlledRedirect,
    ControlledRange,
    WebsocketEcho,
    QuicHttp3,
    UdpDns,
    Ping,
    Dns,
}

#[derive(Debug, Clone, Copy)]
pub struct TargetDefinition {
    pub id: &'static str,
    pub service: &'static str,
    pub name: &'static str,
    pub value: &'static str,
    pub kind: ProbeKind,
    pub capability: ProbeCapability,
    pub weight: u16,
    pub required: bool,
    pub diagnostic: bool,
    pub allowed_statuses: &'static [u16],
    pub content_types: &'static [&'static str],
    pub markers: &'static [&'static str],
}

const OK: &[u16] = &[200];
const OK_OR_PARTIAL: &[u16] = &[200, 206];
const HTML: &[&str] = &["text/html", "application/xhtml+xml"];
const JSON: &[&str] = &["application/json"];
const IMAGE: &[&str] = &["image/"];
const VIDEO_OR_AUDIO: &[&str] = &["video/", "audio/", "application/octet-stream"];

pub const TARGETS: &[TargetDefinition] = &[
    TargetDefinition {
        id: "discord-gateway-api",
        service: "Discord",
        name: "Gateway API",
        value: "https://discord.com/api/v10/gateway",
        kind: ProbeKind::DiscordGatewayApi,
        capability: ProbeCapability::WebApi,
        weight: 14,
        required: true,
        diagnostic: false,
        allowed_statuses: OK,
        content_types: JSON,
        markers: &["\"url\""],
    },
    TargetDefinition {
        id: "discord-gateway-websocket",
        service: "Discord",
        name: "Gateway WebSocket",
        value: "wss://gateway.discord.gg/?v=10&encoding=json",
        kind: ProbeKind::DiscordGatewayWebsocket,
        capability: ProbeCapability::Websocket,
        weight: 18,
        required: true,
        diagnostic: false,
        allowed_statuses: &[],
        content_types: &[],
        markers: &[],
    },
    TargetDefinition {
        id: "discord-web",
        service: "Discord",
        name: "Discord Web",
        value: "https://discord.com/app",
        kind: ProbeKind::Http,
        capability: ProbeCapability::WebApi,
        weight: 10,
        required: true,
        diagnostic: false,
        allowed_statuses: OK,
        content_types: HTML,
        markers: &["discord"],
    },
    TargetDefinition {
        id: "discord-cdn",
        service: "Discord",
        name: "Discord CDN",
        value: "https://cdn.discordapp.com/embed/avatars/0.png",
        kind: ProbeKind::Http,
        capability: ProbeCapability::CdnMedia,
        weight: 8,
        required: false,
        diagnostic: false,
        allowed_statuses: OK,
        content_types: IMAGE,
        markers: &[],
    },
    TargetDefinition {
        id: "discord-voice-readiness",
        service: "Discord",
        name: "Voice UDP readiness",
        value: "UDP:discord-voice",
        kind: ProbeKind::DiscordVoiceReadiness,
        capability: ProbeCapability::UdpVoice,
        weight: 0,
        required: false,
        diagnostic: true,
        allowed_statuses: &[],
        content_types: &[],
        markers: &[],
    },
    TargetDefinition {
        id: "youtube-web",
        service: "YouTube",
        name: "YouTube Web",
        value: "https://www.youtube.com/",
        kind: ProbeKind::YoutubeWeb,
        capability: ProbeCapability::WebApi,
        weight: 14,
        required: true,
        diagnostic: false,
        allowed_statuses: OK,
        content_types: HTML,
        markers: &["youtube", "ytInitialData"],
    },
    TargetDefinition {
        id: "youtube-static",
        service: "YouTube",
        name: "YouTube image CDN",
        value: "https://i.ytimg.com/vi/jNQXAC9IVRw/hqdefault.jpg",
        kind: ProbeKind::Http,
        capability: ProbeCapability::CdnMedia,
        weight: 8,
        required: false,
        diagnostic: false,
        allowed_statuses: OK,
        content_types: IMAGE,
        markers: &[],
    },
    TargetDefinition {
        id: "youtube-media",
        service: "YouTube",
        name: "GoogleVideo media range",
        value: "https://www.youtube.com/watch?v=jNQXAC9IVRw",
        kind: ProbeKind::YoutubeMedia,
        capability: ProbeCapability::CdnMedia,
        weight: 16,
        required: true,
        diagnostic: false,
        allowed_statuses: OK_OR_PARTIAL,
        content_types: VIDEO_OR_AUDIO,
        markers: &[],
    },
    TargetDefinition {
        id: "youtube-http2",
        service: "YouTube",
        name: "YouTube HTTP/2",
        value: "https://www.youtube.com/",
        kind: ProbeKind::Http2,
        capability: ProbeCapability::Transport,
        weight: 6,
        required: false,
        diagnostic: false,
        allowed_statuses: OK,
        content_types: HTML,
        markers: &["youtube"],
    },
    TargetDefinition {
        id: "youtube-http3",
        service: "YouTube",
        name: "YouTube QUIC / HTTP/3 readiness",
        value: "https://www.youtube.com/",
        kind: ProbeKind::QuicHttp3,
        capability: ProbeCapability::Http3,
        weight: 0,
        required: false,
        diagnostic: true,
        allowed_statuses: &[],
        content_types: &[],
        markers: &[],
    },
    TargetDefinition {
        id: "protocol-http1",
        service: "Protocols",
        name: "HTTP/1.1",
        value: "https://www.cloudflare.com/cdn-cgi/trace",
        kind: ProbeKind::Http1,
        capability: ProbeCapability::Transport,
        weight: 4,
        required: false,
        diagnostic: false,
        allowed_statuses: OK,
        content_types: &["text/plain"],
        markers: &["http="],
    },
    TargetDefinition {
        id: "protocol-http2",
        service: "Protocols",
        name: "HTTP/2",
        value: "https://www.cloudflare.com/cdn-cgi/trace",
        kind: ProbeKind::Http2,
        capability: ProbeCapability::Transport,
        weight: 4,
        required: false,
        diagnostic: false,
        allowed_statuses: OK,
        content_types: &["text/plain"],
        markers: &["http="],
    },
    TargetDefinition {
        id: "protocol-tls12",
        service: "Protocols",
        name: "TLS 1.2",
        value: "https://www.cloudflare.com/cdn-cgi/trace",
        kind: ProbeKind::Tls12,
        capability: ProbeCapability::Transport,
        weight: 3,
        required: false,
        diagnostic: false,
        allowed_statuses: OK,
        content_types: &["text/plain"],
        markers: &["tls=TLSv1.2"],
    },
    TargetDefinition {
        id: "protocol-tls13",
        service: "Protocols",
        name: "TLS 1.3",
        value: "https://www.cloudflare.com/cdn-cgi/trace",
        kind: ProbeKind::Tls13,
        capability: ProbeCapability::Transport,
        weight: 3,
        required: false,
        diagnostic: false,
        allowed_statuses: OK,
        content_types: &["text/plain"],
        markers: &["tls=TLSv1.3"],
    },
    TargetDefinition {
        id: "protocol-http3",
        service: "Protocols",
        name: "QUIC / HTTP/3 readiness",
        value: "https://www.cloudflare.com/",
        kind: ProbeKind::QuicHttp3,
        capability: ProbeCapability::Http3,
        weight: 0,
        required: false,
        diagnostic: true,
        allowed_statuses: &[],
        content_types: &[],
        markers: &[],
    },
    TargetDefinition {
        id: "protocol-udp",
        service: "Protocols",
        name: "UDP readiness",
        value: "DNS:1.1.1.1:cloudflare.com",
        kind: ProbeKind::UdpDns,
        capability: ProbeCapability::UdpVoice,
        weight: 0,
        required: false,
        diagnostic: true,
        allowed_statuses: &[],
        content_types: &[],
        markers: &[],
    },
    TargetDefinition {
        id: "dns-system-discord",
        service: "DNS",
        name: "System DNS / Discord",
        value: "DNS:SYSTEM:discord.com",
        kind: ProbeKind::Dns,
        capability: ProbeCapability::Dns,
        weight: 8,
        required: true,
        diagnostic: false,
        allowed_statuses: &[],
        content_types: &[],
        markers: &[],
    },
    TargetDefinition {
        id: "dns-cloudflare-youtube",
        service: "DNS",
        name: "Cloudflare DNS / YouTube",
        value: "DNS:1.1.1.1:youtube.com",
        kind: ProbeKind::Dns,
        capability: ProbeCapability::Dns,
        weight: 6,
        required: false,
        diagnostic: false,
        allowed_statuses: &[],
        content_types: &[],
        markers: &[],
    },
    TargetDefinition {
        id: "route-cloudflare",
        service: "Protocols",
        name: "Cloudflare route",
        value: "PING:1.1.1.1",
        kind: ProbeKind::Ping,
        capability: ProbeCapability::Transport,
        weight: 0,
        required: false,
        diagnostic: true,
        allowed_statuses: &[],
        content_types: &[],
        markers: &[],
    },
];

pub fn default_configs() -> Vec<TestTargetConfig> {
    TARGETS
        .iter()
        .map(|target| TestTargetConfig {
            service: target.service.to_string(),
            name: target.name.to_string(),
            value: target.value.to_string(),
            enabled: true,
        })
        .collect()
}

pub fn definition_for(service: &str, name: &str, value: &str) -> Option<&'static TargetDefinition> {
    TARGETS.iter().find(|target| {
        target.service.eq_ignore_ascii_case(service.trim())
            && target.name.eq_ignore_ascii_case(name.trim())
            && target.value.eq_ignore_ascii_case(value.trim())
    })
}

pub fn is_legacy_builtin(service: &str, name: &str, value: &str) -> bool {
    const LEGACY: &[(&str, &str, &str)] = &[
        ("Discord", "Discord", "https://discord.com"),
        ("Discord", "Discord Gateway", "https://gateway.discord.gg"),
        ("Discord", "Discord CDN", "https://cdn.discordapp.com"),
        ("Discord", "Discord Updates", "https://updates.discord.com"),
        ("YouTube", "YouTube", "https://www.youtube.com"),
        ("YouTube", "YouTube Short", "https://youtu.be"),
        ("YouTube", "YouTube Images", "https://i.ytimg.com"),
        (
            "YouTube",
            "GoogleVideo",
            "https://redirector.googlevideo.com",
        ),
        ("Google", "Google", "https://www.google.com"),
        ("Google", "Google Static", "https://www.gstatic.com"),
        ("Cloudflare", "Cloudflare", "https://www.cloudflare.com"),
        (
            "Cloudflare",
            "Cloudflare CDN",
            "https://cdnjs.cloudflare.com",
        ),
        ("DNS", "System DNS / Discord", "DNS:SYSTEM:discord.com"),
        ("DNS", "Cloudflare DNS / Discord", "DNS:1.1.1.1:discord.com"),
        ("DNS", "Google DNS / YouTube", "DNS:8.8.8.8:youtube.com"),
        ("DNS", "Quad9 / Discord", "DNS:9.9.9.9:discord.com"),
        ("Cloudflare", "Cloudflare route", "PING:1.1.1.1"),
        ("Google", "Google route", "PING:8.8.8.8"),
    ];
    LEGACY.iter().any(|candidate| {
        candidate.0.eq_ignore_ascii_case(service.trim())
            && candidate.1.eq_ignore_ascii_case(name.trim())
            && candidate.2.eq_ignore_ascii_case(value.trim())
    })
}

pub fn migrate_configs(configs: &[TestTargetConfig]) -> Vec<TestTargetConfig> {
    if configs.is_empty() {
        return Vec::new();
    }
    let mut migrated = default_configs();
    for config in configs {
        if let Some(definition) = definition_for(&config.service, &config.name, &config.value) {
            if let Some(current) = migrated.iter_mut().find(|current| {
                current.service.eq_ignore_ascii_case(definition.service)
                    && current.name.eq_ignore_ascii_case(definition.name)
                    && current.value.eq_ignore_ascii_case(definition.value)
            }) {
                current.enabled = config.enabled;
            }
        } else if !is_legacy_builtin(&config.service, &config.name, &config.value) {
            migrated.push(config.clone());
        }
    }
    migrated
}

pub fn service_weight(service: &str) -> u32 {
    match service {
        "Discord" => 38,
        "YouTube" => 38,
        "DNS" => 12,
        "Protocols" => 12,
        "Zapret" => 100,
        _ => 10,
    }
}

pub fn validate_config(config: &TestTargetConfig) -> Result<(), String> {
    let service = config.service.trim();
    let name = config.name.trim();
    let value = config.value.trim();
    if service.is_empty() {
        return Err("Target service is required".into());
    }
    if name.is_empty() {
        return Err("Target name is required".into());
    }
    if value.len() > 2048 {
        return Err("Target value is too long".into());
    }
    if value.eq_ignore_ascii_case("UDP:discord-voice") {
        return definition_for(service, name, value)
            .map(|_| ())
            .ok_or_else(|| {
                "Discord voice probe is available only as a built-in target".to_string()
            });
    }
    if value
        .get(..5)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("PING:"))
    {
        let host = value[5..].trim();
        if host.is_empty() || host.chars().any(char::is_whitespace) {
            return Err("PING target must contain one host or IP address".into());
        }
        return Ok(());
    }
    if value
        .get(..4)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("DNS:"))
    {
        let (resolver, host) = parse_dns_target(value)?;
        if resolver.trim().is_empty()
            || host.trim().is_empty()
            || host.chars().any(char::is_whitespace)
        {
            return Err("DNS target must contain a resolver and host".into());
        }
        if !resolver.eq_ignore_ascii_case("SYSTEM") {
            resolver
                .parse::<std::net::IpAddr>()
                .map_err(|_| "DNS resolver must be SYSTEM or an IP address".to_string())?;
        }
        return Ok(());
    }
    let url = reqwest::Url::parse(value)
        .map_err(|_| "Target must be a valid HTTP, HTTPS or WebSocket URL".to_string())?;
    if !matches!(url.scheme(), "http" | "https" | "wss") || url.host_str().is_none() {
        return Err("Target must be a valid HTTP, HTTPS or WebSocket URL".into());
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err("Target URL must not contain credentials".into());
    }
    if url.scheme() == "wss" && definition_for(service, name, value).is_none() {
        return Err("Custom WebSocket targets are not supported yet".into());
    }
    Ok(())
}

pub fn parse_dns_target(value: &str) -> Result<(&str, &str), String> {
    let rest = value
        .get(4..)
        .ok_or_else(|| "DNS target must use DNS:RESOLVER:HOST format".to_string())?;
    if let Some(bracketed) = rest.strip_prefix('[') {
        let closing = bracketed
            .find("]:")
            .ok_or_else(|| "Bracketed IPv6 DNS target must include a host".to_string())?;
        return Ok((&bracketed[..closing], &bracketed[closing + 2..]));
    }
    rest.split_once(':')
        .ok_or_else(|| "DNS target must use DNS:RESOLVER:HOST format".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_manifest_is_valid_and_unique() {
        let mut ids = std::collections::HashSet::new();
        for target in TARGETS {
            assert!(ids.insert(target.id));
            validate_config(&TestTargetConfig {
                service: target.service.into(),
                name: target.name.into(),
                value: target.value.into(),
                enabled: true,
            })
            .unwrap();
        }
    }

    #[test]
    fn rejects_credentials_and_invalid_dns_targets() {
        let config = |value: &str| TestTargetConfig {
            service: "Custom".into(),
            name: "Target".into(),
            value: value.into(),
            enabled: true,
        };
        assert!(validate_config(&config("https://user:pass@example.com")).is_err());
        assert!(validate_config(&config("DNS:not-an-ip:discord.com")).is_err());
        assert!(validate_config(&config("DNS:1.1.1.1:discord.com")).is_ok());
        assert!(validate_config(&config("DNS:[2606:4700:4700::1111]:discord.com")).is_ok());
    }

    #[test]
    fn migrates_stage_one_defaults_and_keeps_custom_targets() {
        let stored = vec![
            TestTargetConfig {
                service: "Discord".into(),
                name: "Discord".into(),
                value: "https://discord.com".into(),
                enabled: true,
            },
            TestTargetConfig {
                service: "Custom".into(),
                name: "Example".into(),
                value: "https://example.com".into(),
                enabled: true,
            },
        ];
        let migrated = migrate_configs(&stored);
        assert!(migrated
            .iter()
            .any(|target| target.name == "Gateway WebSocket"));
        assert!(migrated
            .iter()
            .any(|target| target.name == "GoogleVideo media range"));
        assert!(migrated.iter().any(|target| target.name == "Example"));
        assert!(!migrated
            .iter()
            .any(|target| target.name == "Discord" && target.value == "https://discord.com"));
    }
}
