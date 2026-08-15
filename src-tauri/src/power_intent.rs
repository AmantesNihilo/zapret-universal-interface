use crate::paths;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PowerIntent {
    enabled: bool,
    profile_id: String,
}

pub fn should_restore(active_profile_id: &str) -> bool {
    should_restore_at(&paths::power_intent_path(), active_profile_id)
}

fn should_restore_at(path: &std::path::Path, active_profile_id: &str) -> bool {
    let Ok(text) = std::fs::read_to_string(path) else {
        return false;
    };
    serde_json::from_str::<PowerIntent>(&text)
        .map(|intent| intent.enabled && intent.profile_id == active_profile_id)
        .unwrap_or(false)
}

pub fn remember_started(profile_id: &str) -> Result<(), String> {
    remember_started_at(&paths::power_intent_path(), profile_id)
}

fn remember_started_at(path: &std::path::Path, profile_id: &str) -> Result<(), String> {
    let intent = PowerIntent {
        enabled: true,
        profile_id: profile_id.to_string(),
    };
    let text = serde_json::to_string_pretty(&intent).map_err(|error| error.to_string())?;
    std::fs::write(path, text).map_err(|error| error.to_string())
}

pub fn clear() -> Result<(), String> {
    clear_at(&paths::power_intent_path())
}

fn clear_at(path: &std::path::Path) -> Result<(), String> {
    if path.exists() {
        std::fs::remove_file(path).map_err(|error| error.to_string())?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn power_intent_uses_camel_case_profile_id() {
        let value = serde_json::to_value(PowerIntent {
            enabled: true,
            profile_id: "default".to_string(),
        })
        .expect("serialize intent");

        assert_eq!(value["enabled"], true);
        assert_eq!(value["profileId"], "default");
    }

    #[test]
    fn started_profile_is_restored_only_until_explicit_clear() {
        let path =
            std::env::temp_dir().join(format!("zui-power-intent-{}.json", rand::random::<u64>()));

        remember_started_at(&path, "profile-a").expect("write power intent");
        assert!(should_restore_at(&path, "profile-a"));
        assert!(!should_restore_at(&path, "profile-b"));

        clear_at(&path).expect("clear power intent");
        assert!(!should_restore_at(&path, "profile-a"));
    }
}
