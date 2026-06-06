use crate::models::{UpdateAsset, UpdateCheck};
use crate::paths;
use reqwest::blocking::Client;
use reqwest::header::{ACCEPT, USER_AGENT};
use serde::Deserialize;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;

const OWNER: &str = "AmantesNihilo";
const REPO: &str = "zapret-universal-interface";
const LATEST_RELEASE_URL: &str =
    "https://api.github.com/repos/AmantesNihilo/zapret-universal-interface/releases/latest";
const RELEASES_URL: &str = "https://github.com/AmantesNihilo/zapret-universal-interface/releases";
const DOWNLOAD_URL_PREFIX: &str =
    "https://github.com/AmantesNihilo/zapret-universal-interface/releases/download/";

#[derive(Debug, Deserialize)]
struct GithubRelease {
    tag_name: String,
    name: Option<String>,
    body: Option<String>,
    html_url: String,
    published_at: Option<String>,
    assets: Vec<GithubAsset>,
}

#[derive(Debug, Deserialize)]
struct GithubAsset {
    name: String,
    browser_download_url: String,
    size: u64,
}

pub fn check() -> Result<UpdateCheck, String> {
    let release = fetch_latest_release()?;
    Ok(build_update_check(release))
}

pub fn download_installer() -> Result<PathBuf, String> {
    let update = check()?;
    if update.distribution != "installed" {
        return Err("Automatic update is available only for installed builds.".into());
    }
    if !update.update_available {
        return Err("No update is available.".into());
    }

    let asset = update
        .installer_asset
        .as_ref()
        .ok_or_else(|| "No installer asset was found in the latest release.".to_string())?;
    validate_download_url(&asset.download_url)?;

    let file_name = sanitize_file_name(&asset.name);
    let target_dir = std::env::temp_dir()
        .join("ZUI")
        .join("updates")
        .join(update.latest_version.unwrap_or_else(|| "latest".into()));
    std::fs::create_dir_all(&target_dir).map_err(|error| error.to_string())?;
    let target = target_dir.join(file_name);

    let client = http_client(Duration::from_secs(180))?;
    let mut response = client
        .get(&asset.download_url)
        .header(USER_AGENT, user_agent())
        .send()
        .map_err(|error| error.to_string())?
        .error_for_status()
        .map_err(|error| error.to_string())?;

    let mut file = File::create(&target).map_err(|error| error.to_string())?;
    std::io::copy(&mut response, &mut file).map_err(|error| error.to_string())?;
    Ok(target)
}

pub fn launch_installer(path: &Path) -> Result<(), String> {
    if !path.exists() {
        return Err(format!(
            "Update installer was not found: {}",
            path.display()
        ));
    }

    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();

    let mut command = if extension == "msi" {
        let mut command = Command::new("msiexec");
        command.arg("/i").arg(path);
        command
    } else {
        Command::new(path)
    };

    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .creation_flags(0x08000000)
        .spawn()
        .map_err(|error| error.to_string())?;
    Ok(())
}

pub fn is_allowed_release_url(url: &str) -> bool {
    url == RELEASES_URL
        || url.starts_with(&format!("{RELEASES_URL}/"))
        || url.starts_with(DOWNLOAD_URL_PREFIX)
}

fn fetch_latest_release() -> Result<GithubRelease, String> {
    let client = http_client(Duration::from_secs(15))?;
    let text = client
        .get(LATEST_RELEASE_URL)
        .header(USER_AGENT, user_agent())
        .header(ACCEPT, "application/vnd.github+json")
        .send()
        .map_err(|error| error.to_string())?
        .error_for_status()
        .map_err(|error| error.to_string())?
        .text()
        .map_err(|error| error.to_string())?;

    serde_json::from_str(&text).map_err(|error| error.to_string())
}

fn build_update_check(release: GithubRelease) -> UpdateCheck {
    let current_version = env!("CARGO_PKG_VERSION").to_string();
    let latest_version = clean_version(&release.tag_name);
    let installer_asset = pick_installer_asset(&release.assets);
    let portable_asset = pick_portable_asset(&release.assets);
    let distribution = paths::distribution_mode().to_string();
    let update_available = is_newer_version(&latest_version, &current_version);
    let can_install = distribution == "installed" && update_available && installer_asset.is_some();

    UpdateCheck {
        update_available,
        current_version,
        latest_version: Some(latest_version),
        release_name: release.name,
        release_notes: release.body.map(trim_release_notes),
        release_url: Some(release.html_url),
        published_at: release.published_at,
        distribution,
        can_install,
        installer_asset,
        portable_asset,
    }
}

fn pick_installer_asset(assets: &[GithubAsset]) -> Option<UpdateAsset> {
    assets
        .iter()
        .filter(|asset| {
            let name = asset.name.to_ascii_lowercase();
            name.ends_with(".exe")
                && name.contains("setup")
                && (name.contains("zui") || name.contains("z2"))
        })
        .max_by_key(|asset| score_installer_name(&asset.name))
        .or_else(|| {
            assets
                .iter()
                .find(|asset| asset.name.to_ascii_lowercase().ends_with(".msi"))
        })
        .map(to_update_asset)
}

fn pick_portable_asset(assets: &[GithubAsset]) -> Option<UpdateAsset> {
    assets
        .iter()
        .find(|asset| {
            let name = asset.name.to_ascii_lowercase();
            name.contains("portable") && (name.ends_with(".zip") || name.ends_with(".7z"))
        })
        .map(to_update_asset)
}

fn score_installer_name(name: &str) -> u8 {
    let lower = name.to_ascii_lowercase();
    let mut score = 0;
    if lower.contains("zui") {
        score += 4;
    }
    if lower.contains("x64") {
        score += 2;
    }
    if lower.ends_with("-setup.exe") || lower.contains("setup") {
        score += 2;
    }
    score
}

fn to_update_asset(asset: &GithubAsset) -> UpdateAsset {
    let lower = asset.name.to_ascii_lowercase();
    let kind = if lower.ends_with(".msi") {
        "msi"
    } else if lower.contains("portable") {
        "portable"
    } else {
        "setup"
    };

    UpdateAsset {
        name: asset.name.clone(),
        download_url: asset.browser_download_url.clone(),
        size: asset.size,
        kind: kind.into(),
    }
}

fn clean_version(tag: &str) -> String {
    tag.trim()
        .trim_start_matches('v')
        .trim_start_matches('V')
        .trim()
        .to_string()
}

fn is_newer_version(latest: &str, current: &str) -> bool {
    let latest_parts = parse_version_parts(latest);
    let current_parts = parse_version_parts(current);
    match (latest_parts, current_parts) {
        (Some(latest_parts), Some(current_parts)) => latest_parts > current_parts,
        _ => latest != current,
    }
}

fn parse_version_parts(value: &str) -> Option<Vec<u64>> {
    let raw = value
        .split(['-', '+'])
        .next()
        .unwrap_or(value)
        .trim()
        .trim_start_matches('v')
        .trim_start_matches('V');
    let normalized = raw
        .chars()
        .take_while(|ch| ch.is_ascii_digit() || *ch == '.')
        .collect::<String>();
    let normalized = normalized.trim_matches('.');
    if normalized.is_empty() {
        return None;
    }
    let mut parts = Vec::new();
    for part in normalized.split('.') {
        parts.push(part.parse::<u64>().ok()?);
    }
    while parts.len() < 3 {
        parts.push(0);
    }
    Some(parts)
}

fn trim_release_notes(value: String) -> String {
    const LIMIT: usize = 2200;
    let trimmed = value.trim();
    if trimmed.chars().count() <= LIMIT {
        return trimmed.to_string();
    }
    let mut result: String = trimmed.chars().take(LIMIT).collect();
    result.push_str("...");
    result
}

fn sanitize_file_name(value: &str) -> String {
    let sanitized: String = value
        .chars()
        .map(|ch| match ch {
            '\\' | '/' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            _ => ch,
        })
        .collect();
    if sanitized.trim().is_empty() {
        "ZUI-update.exe".into()
    } else {
        sanitized
    }
}

fn validate_download_url(url: &str) -> Result<(), String> {
    if url.starts_with(DOWNLOAD_URL_PREFIX) {
        Ok(())
    } else {
        Err("Update asset URL is not allowed.".into())
    }
}

fn http_client(timeout: Duration) -> Result<Client, String> {
    Client::builder()
        .timeout(timeout)
        .build()
        .map_err(|error| error.to_string())
}

fn user_agent() -> String {
    format!("ZUI/{} ({OWNER}/{REPO})", env!("CARGO_PKG_VERSION"))
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
    use super::is_newer_version;

    #[test]
    fn compares_standard_versions() {
        assert!(is_newer_version("2.0.1", "2.0.0"));
        assert!(is_newer_version("v2.1.0", "2.0.9"));
        assert!(!is_newer_version("2.0.0", "2.0.0"));
        assert!(!is_newer_version("1.9.9", "2.0.0"));
    }

    #[test]
    fn ignores_release_suffixes_for_ordering() {
        assert!(!is_newer_version("v1.5.0_hotfix", "2.0.0"));
        assert!(is_newer_version("v2.0.1_hotfix", "2.0.0"));
    }
}
