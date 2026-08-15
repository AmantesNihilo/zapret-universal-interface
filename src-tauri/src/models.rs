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
    pub minimize_behavior: MinimizeBehavior,
    pub minimize_dont_ask: bool,
    pub start_with_windows: bool,
    pub auto_start_active_profile_on_launch: bool,
    pub check_updates_on_launch: bool,
    pub custom_preset_roots: Vec<String>,
    pub test_targets: Vec<TestTargetConfig>,
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
            minimize_behavior: MinimizeBehavior::Taskbar,
            minimize_dont_ask: false,
            start_with_windows: false,
            auto_start_active_profile_on_launch: false,
            check_updates_on_launch: true,
            custom_preset_roots: Vec::new(),
            test_targets: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MinimizeBehavior {
    #[default]
    Taskbar,
    Tray,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(default)]
pub struct TestTargetConfig {
    pub service: String,
    pub name: String,
    pub value: String,
    pub enabled: bool,
}

impl Default for TestTargetConfig {
    fn default() -> Self {
        Self {
            service: String::new(),
            name: String::new(),
            value: String::new(),
            enabled: true,
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
    pub zapret_engine: Option<ZapretEngine>,
    pub zapret_preset_id: Option<String>,
    pub tg_ws_enabled: bool,
    pub tg_ws_host: String,
    pub tg_ws_port: u16,
    pub tg_ws_secret: String,
    pub tg_ws_dc_ips: Vec<String>,
    pub tg_ws_cf_proxy_enabled: bool,
    pub tg_ws_cf_custom_enabled: bool,
    pub tg_ws_default_domains: bool,
    pub tg_ws_cf_domains: Vec<String>,
    pub tg_ws_cf_worker_enabled: bool,
    pub tg_ws_cf_worker_domain: Option<String>,
    pub tg_ws_fronting_domain: Option<String>,
    pub tg_ws_cf_priority: bool,
    pub tg_ws_cf_balance: bool,
    pub tg_ws_buf_kb: usize,
    pub tg_ws_pool_size: usize,
    pub tg_ws_verbose: bool,
    pub tg_ws_log_max_mb: f64,
    pub tg_ws_force_test_dc: bool,
    pub autostart_on_app_launch: Option<bool>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TgWsConnectivityKind {
    CfProxy,
    CfWorker,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TgWsConnectivityProbe {
    pub domain: String,
    pub dc: u32,
    pub target: String,
    pub ok: bool,
    pub latency_ms: Option<u128>,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TgWsConnectivityReport {
    pub kind: TgWsConnectivityKind,
    pub all_ok: bool,
    pub probes: Vec<TgWsConnectivityProbe>,
}

impl Default for Profile {
    fn default() -> Self {
        Self {
            id: "default".into(),
            name: "Default".into(),
            zapret_enabled: false,
            zapret_engine: None,
            zapret_preset_id: None,
            tg_ws_enabled: false,
            tg_ws_host: "127.0.0.1".into(),
            tg_ws_port: 1443,
            tg_ws_secret: hex::encode(rand::random::<[u8; 16]>()),
            tg_ws_dc_ips: vec!["2:149.154.167.220".into(), "4:149.154.167.220".into()],
            tg_ws_cf_proxy_enabled: true,
            tg_ws_cf_custom_enabled: false,
            tg_ws_default_domains: true,
            tg_ws_cf_domains: Vec::new(),
            tg_ws_cf_worker_enabled: false,
            tg_ws_cf_worker_domain: None,
            tg_ws_fronting_domain: Some("sprinthost.ru".into()),
            tg_ws_cf_priority: false,
            tg_ws_cf_balance: false,
            tg_ws_buf_kb: 256,
            tg_ws_pool_size: 4,
            tg_ws_verbose: false,
            tg_ws_log_max_mb: 5.0,
            tg_ws_force_test_dc: false,
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
    pub engine: ZapretEngine,
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

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ZapretEngine {
    #[default]
    Classic,
    Zapret2,
}

impl ZapretEngine {
    pub fn process_name(self) -> &'static str {
        match self {
            Self::Classic => "winws.exe",
            Self::Zapret2 => "winws2.exe",
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            Self::Classic => "Zapret Classic",
            Self::Zapret2 => "Zapret 2",
        }
    }
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
    #[serde(default)]
    pub engine: ZapretEngine,
    #[serde(default)]
    pub preset_version: String,
    pub mode: TestMode,
    pub started_at: String,
    pub finished_at: String,
    #[serde(default)]
    pub cached_at: String,
    #[serde(default)]
    pub recommendation: TestRecommendation,
    pub score: u8,
    pub ok: u32,
    pub total: u32,
    pub services: Vec<ServiceTestResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TestMode {
    #[serde(alias = "quick")]
    Selected,
    #[serde(alias = "full", alias = "best", alias = "advanced")]
    All,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TestRecommendation {
    Recommended,
    Partial,
    NotRecommended,
}

impl Default for TestRecommendation {
    fn default() -> Self {
        Self::NotRecommended
    }
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
pub enum TestPhase {
    Starting,
    Warmup,
    Checking,
    Finishing,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TestProgress {
    pub test_id: String,
    pub preset_id: String,
    pub preset_name: String,
    pub engine: ZapretEngine,
    pub preset_index: usize,
    pub preset_count: usize,
    pub total_checks: u32,
    pub completed_checks: u32,
    pub passed_checks: u32,
    pub failed_checks: u32,
    pub phase: TestPhase,
    pub current_target: Option<String>,
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
    pub winws2_found: bool,
    pub tg_ws_found: bool,
    pub tg_ws_engine: String,
    pub tg_ws_engine_version: String,
    pub winws_running: bool,
    pub winws2_running: bool,
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
