use crate::models::{Profile, ProfilesFile, ZapretEngine};
use crate::{json_storage, paths};

pub fn load_profiles() -> Result<ProfilesFile, String> {
    paths::ensure_data_layout().map_err(|error| error.to_string())?;
    let path = paths::profiles_path();
    if !path.exists() {
        let profiles = ProfilesFile::default();
        save_profiles(&profiles)?;
        return Ok(profiles);
    }

    let text = std::fs::read_to_string(&path).map_err(|error| error.to_string())?;
    let mut profiles: ProfilesFile = match json_storage::parse(&text) {
        Ok(profiles) => profiles,
        Err(error) => {
            let backup = json_storage::backup_invalid(&path)?;
            let profiles = ProfilesFile::default();
            save_profiles(&profiles)?;
            eprintln!(
                "Stored profiles were reset after a JSON error ({error}); backup: {}",
                backup.display()
            );
            return Ok(profiles);
        }
    };
    let mut migrated = text.starts_with('\u{feff}');
    if profiles.profiles.is_empty() {
        profiles = ProfilesFile::default();
        migrated = true;
    } else if !profiles
        .profiles
        .iter()
        .any(|profile| profile.id == profiles.active_profile_id)
    {
        profiles.active_profile_id = profiles.profiles[0].id.clone();
        migrated = true;
    }
    for profile in &mut profiles.profiles {
        if ensure_secret(profile) {
            migrated = true;
        }
        if profile.zapret_engine.is_none() && profile.zapret_preset_id.is_some() {
            profile.zapret_engine = Some(ZapretEngine::Classic);
            migrated = true;
        }
        // 2.1.0 profiles predate the explicit Flowseal routing switches.
        // Preserve custom values and enable the same automatic CF fallback the
        // original desktop application uses.
        if !text.contains("\"tgWsCfProxyEnabled\"") {
            profile.tg_ws_cf_proxy_enabled = true;
            profile.tg_ws_cf_custom_enabled = !profile.tg_ws_cf_domains.is_empty();
            profile.tg_ws_default_domains = profile.tg_ws_cf_domains.is_empty();
            profile.tg_ws_cf_worker_enabled = profile
                .tg_ws_cf_worker_domain
                .as_deref()
                .is_some_and(|value| !value.trim().is_empty());
            if profile.tg_ws_dc_ips.is_empty() {
                profile.tg_ws_dc_ips = vec![
                    "2:149.154.167.220".to_string(),
                    "4:149.154.167.220".to_string(),
                ];
            }
            migrated = true;
        }
    }
    if migrated {
        save_profiles(&profiles)?;
    }
    Ok(profiles)
}

pub fn save_profiles(profiles: &ProfilesFile) -> Result<(), String> {
    paths::ensure_data_layout().map_err(|error| error.to_string())?;
    let text = serde_json::to_string_pretty(profiles).map_err(|error| error.to_string())?;
    std::fs::write(paths::profiles_path(), text).map_err(|error| error.to_string())
}

pub fn save_profile(mut profile: Profile) -> Result<ProfilesFile, String> {
    ensure_secret(&mut profile);
    let mut file = load_profiles()?;
    if let Some(existing) = file.profiles.iter_mut().find(|item| item.id == profile.id) {
        *existing = profile;
    } else {
        file.profiles.push(profile);
    }
    save_profiles(&file)?;
    Ok(file)
}

fn generate_secret() -> String {
    hex::encode(rand::random::<[u8; 16]>())
}

fn ensure_secret(profile: &mut Profile) -> bool {
    if !profile.tg_ws_secret.trim().is_empty() {
        return false;
    }
    profile.tg_ws_secret = generate_secret();
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_legacy_secret_is_generated_once() {
        let mut profile = Profile {
            tg_ws_secret: String::new(),
            ..Profile::default()
        };

        assert!(ensure_secret(&mut profile));
        let generated = profile.tg_ws_secret.clone();
        assert_eq!(generated.len(), 32);
        assert!(generated.bytes().all(|byte| byte.is_ascii_hexdigit()));
        assert!(!ensure_secret(&mut profile));
        assert_eq!(profile.tg_ws_secret, generated);
    }
}

pub fn set_active_profile(profile_id: String) -> Result<ProfilesFile, String> {
    let mut file = load_profiles()?;
    if !file.profiles.iter().any(|profile| profile.id == profile_id) {
        return Err("Profile not found".into());
    }
    file.active_profile_id = profile_id;
    save_profiles(&file)?;
    Ok(file)
}

pub fn delete_profile(profile_id: String) -> Result<ProfilesFile, String> {
    let mut file = load_profiles()?;
    if file.profiles.len() <= 1 {
        return Err("Cannot delete the last profile".into());
    }
    if !file.profiles.iter().any(|profile| profile.id == profile_id) {
        return Err("Profile not found".into());
    }

    file.profiles.retain(|profile| profile.id != profile_id);
    if file.active_profile_id == profile_id {
        file.active_profile_id = file
            .profiles
            .first()
            .map(|profile| profile.id.clone())
            .ok_or_else(|| "No profiles remain".to_string())?;
    }
    save_profiles(&file)?;
    Ok(file)
}

pub fn active_profile() -> Result<Profile, String> {
    let file = load_profiles()?;
    file.profiles
        .into_iter()
        .find(|profile| profile.id == file.active_profile_id)
        .ok_or_else(|| "Active profile not found".into())
}
