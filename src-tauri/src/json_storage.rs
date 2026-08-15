use serde::de::DeserializeOwned;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub fn parse<T: DeserializeOwned>(text: &str) -> Result<T, serde_json::Error> {
    serde_json::from_str(text.trim_start_matches('\u{feff}'))
}

pub fn backup_invalid(path: &Path) -> Result<PathBuf, String> {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("data.json");
    let backup = path.with_file_name(format!("{file_name}.bad-{timestamp}"));
    std::fs::rename(path, &backup).map_err(|error| error.to_string())?;
    Ok(backup)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[derive(Debug, Deserialize, PartialEq)]
    struct Example {
        enabled: bool,
    }

    #[test]
    fn parses_utf8_bom_json() {
        let value: Example = parse("\u{feff}{\"enabled\":true}").expect("BOM JSON");
        assert_eq!(value, Example { enabled: true });
    }

    #[test]
    fn rejects_empty_json() {
        assert!(parse::<Example>("").is_err());
    }
}
