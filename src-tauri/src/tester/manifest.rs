use crate::models::TestTargetConfig;

#[derive(Debug, Clone, Copy)]
pub struct TargetDefinition {
    pub id: &'static str,
    pub service: &'static str,
    pub name: &'static str,
    pub value: &'static str,
    pub weight: u16,
    pub required: bool,
    pub diagnostic: bool,
}

pub const TARGETS: &[TargetDefinition] = &[
    TargetDefinition {
        id: "discord-web",
        service: "Discord",
        name: "Discord",
        value: "https://discord.com",
        weight: 12,
        required: true,
        diagnostic: false,
    },
    TargetDefinition {
        id: "discord-gateway",
        service: "Discord",
        name: "Discord Gateway",
        value: "https://gateway.discord.gg",
        weight: 16,
        required: true,
        diagnostic: false,
    },
    TargetDefinition {
        id: "discord-cdn",
        service: "Discord",
        name: "Discord CDN",
        value: "https://cdn.discordapp.com",
        weight: 8,
        required: false,
        diagnostic: false,
    },
    TargetDefinition {
        id: "discord-updates",
        service: "Discord",
        name: "Discord Updates",
        value: "https://updates.discord.com",
        weight: 6,
        required: false,
        diagnostic: false,
    },
    TargetDefinition {
        id: "youtube-web",
        service: "YouTube",
        name: "YouTube",
        value: "https://www.youtube.com",
        weight: 14,
        required: true,
        diagnostic: false,
    },
    TargetDefinition {
        id: "youtube-short",
        service: "YouTube",
        name: "YouTube Short",
        value: "https://youtu.be",
        weight: 6,
        required: false,
        diagnostic: false,
    },
    TargetDefinition {
        id: "youtube-images",
        service: "YouTube",
        name: "YouTube Images",
        value: "https://i.ytimg.com",
        weight: 8,
        required: false,
        diagnostic: false,
    },
    TargetDefinition {
        id: "youtube-video",
        service: "YouTube",
        name: "GoogleVideo",
        value: "https://redirector.googlevideo.com",
        weight: 14,
        required: true,
        diagnostic: false,
    },
    TargetDefinition {
        id: "google-web",
        service: "Google",
        name: "Google",
        value: "https://www.google.com",
        weight: 8,
        required: false,
        diagnostic: false,
    },
    TargetDefinition {
        id: "google-static",
        service: "Google",
        name: "Google Static",
        value: "https://www.gstatic.com",
        weight: 6,
        required: false,
        diagnostic: false,
    },
    TargetDefinition {
        id: "cloudflare-web",
        service: "Cloudflare",
        name: "Cloudflare",
        value: "https://www.cloudflare.com",
        weight: 8,
        required: false,
        diagnostic: false,
    },
    TargetDefinition {
        id: "cloudflare-cdn",
        service: "Cloudflare",
        name: "Cloudflare CDN",
        value: "https://cdnjs.cloudflare.com",
        weight: 6,
        required: false,
        diagnostic: false,
    },
    TargetDefinition {
        id: "dns-system-discord",
        service: "DNS",
        name: "System DNS / Discord",
        value: "DNS:SYSTEM:discord.com",
        weight: 8,
        required: true,
        diagnostic: false,
    },
    TargetDefinition {
        id: "dns-cloudflare-discord",
        service: "DNS",
        name: "Cloudflare DNS / Discord",
        value: "DNS:1.1.1.1:discord.com",
        weight: 6,
        required: false,
        diagnostic: false,
    },
    TargetDefinition {
        id: "dns-google-youtube",
        service: "DNS",
        name: "Google DNS / YouTube",
        value: "DNS:8.8.8.8:youtube.com",
        weight: 6,
        required: false,
        diagnostic: false,
    },
    TargetDefinition {
        id: "dns-quad9-discord",
        service: "DNS",
        name: "Quad9 / Discord",
        value: "DNS:9.9.9.9:discord.com",
        weight: 6,
        required: false,
        diagnostic: false,
    },
    TargetDefinition {
        id: "route-cloudflare",
        service: "Cloudflare",
        name: "Cloudflare route",
        value: "PING:1.1.1.1",
        weight: 0,
        required: false,
        diagnostic: true,
    },
    TargetDefinition {
        id: "route-google",
        service: "Google",
        name: "Google route",
        value: "PING:8.8.8.8",
        weight: 0,
        required: false,
        diagnostic: true,
    },
];

pub fn default_configs() -> Vec<TestTargetConfig> {
    TARGETS
        .iter()
        .map(|target| {
            debug_assert!(!target.id.is_empty());
            TestTargetConfig {
                service: target.service.to_string(),
                name: target.name.to_string(),
                value: target.value.to_string(),
                enabled: true,
            }
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

pub fn service_weight(service: &str) -> u32 {
    match service {
        "Discord" => 35,
        "YouTube" => 35,
        "DNS" => 15,
        "Google" => 8,
        "Cloudflare" => 7,
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
    if value.len() >= 5 && value[..5].eq_ignore_ascii_case("PING:") {
        let host = value[5..].trim();
        if host.is_empty() || host.chars().any(char::is_whitespace) {
            return Err("PING target must contain one host or IP address".into());
        }
        return Ok(());
    }
    if value.len() >= 4 && value[..4].eq_ignore_ascii_case("DNS:") {
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
        .map_err(|_| "Target must be a valid HTTP or HTTPS URL".to_string())?;
    if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none() {
        return Err("Target must be a valid HTTP or HTTPS URL".into());
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err("Target URL must not contain credentials".into());
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
        let resolver = &bracketed[..closing];
        let host = &bracketed[closing + 2..];
        return Ok((resolver, host));
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
}
