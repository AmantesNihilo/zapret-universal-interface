use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(default)]
pub struct Settings {
    pub theme: ThemeMode,
    pub accent: String,
    pub language: String,
    pub layout_orientation: LayoutOrientation,
    pub launch_minimized: bool,
    pub close_to_tray: bool,
    pub start_with_windows: bool,
    pub auto_start_active_profile_on_launch: bool,
    pub custom_preset_roots: Vec<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: ThemeMode::Dark,
            accent: "cyan".into(),
            language: "ru".into(),
            layout_orientation: LayoutOrientation::Portrait,
            launch_minimized: false,
            close_to_tray: false,
            start_with_windows: false,
            auto_start_active_profile_on_launch: false,
            custom_preset_roots: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ThemeMode {
    Dark,
    Light,
    Oled,
    System,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LayoutOrientation {
    Portrait,
    Landscape,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(default)]
pub struct Profile {
    pub id: String,
    pub name: String,
    pub zapret_enabled: bool,
    pub zapret_preset_id: Option<String>,
    pub tg_ws_enabled: bool,
    pub tg_ws_host: String,
    pub tg_ws_port: u16,
    pub tg_ws_secret: String,
    pub tg_ws_default_domains: bool,
    pub tg_ws_cf_domains: Vec<String>,
    pub tg_ws_cf_worker_domain: Option<String>,
    pub tg_ws_cf_priority: bool,
    pub tg_ws_cf_balance: bool,
    pub autostart_on_app_launch: Option<bool>,
    pub notes: Option<String>,
}

impl Default for Profile {
    fn default() -> Self {
        Self {
            id: "default".into(),
            name: "Default".into(),
            zapret_enabled: true,
            zapret_preset_id: None,
            tg_ws_enabled: false,
            tg_ws_host: "127.0.0.1".into(),
            tg_ws_port: 1081,
            tg_ws_secret: "change-me".into(),
            tg_ws_default_domains: false,
            tg_ws_cf_domains: Vec::new(),
            tg_ws_cf_worker_domain: None,
            tg_ws_cf_priority: false,
            tg_ws_cf_balance: false,
            autostart_on_app_launch: Some(false),
            notes: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfilesFile {
    pub active_profile_id: String,
    pub profiles: Vec<Profile>,
}

impl Default for ProfilesFile {
    fn default() -> Self {
        Self {
            active_profile_id: "default".into(),
            profiles: vec![Profile::default()],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Preset {
    pub id: String,
    pub name: String,
    pub path: String,
    pub relative_path: String,
    pub kind: PresetKind,
    pub favorite: bool,
    pub hidden: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PresetPreferences {
    pub favorite: bool,
    pub hidden: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PresetKind {
    Bat,
    Cmd,
    Config,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServiceStatus {
    pub service: ServiceName,
    pub state: ServiceState,
    pub pid: Option<u32>,
    pub message: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConflictProcess {
    pub image: String,
    pub pid: u32,
    pub title: Option<String>,
}

impl ServiceStatus {
    pub fn stopped(service: ServiceName) -> Self {
        Self {
            service,
            state: ServiceState::Stopped,
            pid: None,
            message: None,
            error: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ServiceName {
    Zapret,
    TgWs,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ServiceState {
    Stopped,
    Starting,
    Running,
    Stopping,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppState {
    pub status: AppStatus,
    pub active_profile_id: String,
    pub zapret: ServiceStatus,
    pub tg_ws: ServiceStatus,
    pub last_error: Option<String>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            status: AppStatus::Off,
            active_profile_id: "default".into(),
            zapret: ServiceStatus::stopped(ServiceName::Zapret),
            tg_ws: ServiceStatus::stopped(ServiceName::TgWs),
            last_error: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AppStatus {
    Off,
    Starting,
    On,
    Stopping,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogLine {
    pub source: LogSource,
    pub timestamp: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LogSource {
    App,
    Zapret,
    TgWs,
    Tests,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TestResult {
    pub id: String,
    pub preset_id: String,
    pub preset_name: String,
    pub mode: TestMode,
    pub started_at: String,
    pub finished_at: String,
    pub score: u8,
    pub ok: u32,
    pub total: u32,
    pub services: Vec<ServiceTestResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TestMode {
    Quick,
    Full,
    Advanced,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServiceTestResult {
    pub name: String,
    pub status: TestServiceStatus,
    pub ok: u32,
    pub total: u32,
    pub errors: Vec<String>,
    pub targets: Vec<TestTargetResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TestServiceStatus {
    Passed,
    Partial,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TestTargetResult {
    pub service: String,
    #[serde(default)]
    pub label: String,
    pub url: String,
    pub ok: bool,
    pub status: Option<u16>,
    pub latency_ms: Option<u128>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Diagnostics {
    pub resources_path: String,
    pub data_path: String,
    pub logs_path: String,
    pub preset_count: usize,
    pub selected_preset_exists: bool,
    pub winws_found: bool,
    pub tg_ws_found: bool,
    pub tg_ws_engine: String,
    pub tg_ws_engine_version: String,
    pub winws_running: bool,
    pub tg_ws_running: bool,
    pub is_admin: bool,
    pub tg_ws_port_available: bool,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCheck {
    pub update_available: bool,
    pub current_version: String,
    pub latest_version: Option<String>,
    pub release_name: Option<String>,
    pub release_notes: Option<String>,
    pub release_url: Option<String>,
    pub published_at: Option<String>,
    pub distribution: String,
    pub can_install: bool,
    pub installer_asset: Option<UpdateAsset>,
    pub portable_asset: Option<UpdateAsset>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateAsset {
    pub name: String,
    pub download_url: String,
    pub size: u64,
    pub kind: String,
}
