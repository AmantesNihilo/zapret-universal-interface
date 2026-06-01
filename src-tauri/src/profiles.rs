use crate::models::{Profile, ProfilesFile};
use crate::paths;

pub fn load_profiles() -> Result<ProfilesFile, String> {
    paths::ensure_data_layout().map_err(|error| error.to_string())?;
    let path = paths::profiles_path();
    if !path.exists() {
        let profiles = ProfilesFile::default();
        save_profiles(&profiles)?;
        return Ok(profiles);
    }

    let text = std::fs::read_to_string(path).map_err(|error| error.to_string())?;
    serde_json::from_str(&text).map_err(|error| error.to_string())
}

pub fn save_profiles(profiles: &ProfilesFile) -> Result<(), String> {
    paths::ensure_data_layout().map_err(|error| error.to_string())?;
    let text = serde_json::to_string_pretty(profiles).map_err(|error| error.to_string())?;
    std::fs::write(paths::profiles_path(), text).map_err(|error| error.to_string())
}

pub fn save_profile(profile: Profile) -> Result<ProfilesFile, String> {
    let mut file = load_profiles()?;
    if let Some(existing) = file.profiles.iter_mut().find(|item| item.id == profile.id) {
        *existing = profile;
    } else {
        file.profiles.push(profile);
    }
    save_profiles(&file)?;
    Ok(file)
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
