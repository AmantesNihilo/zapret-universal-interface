use crate::models::{AppState, LogLine, Settings, TestResult, ZapretEngine};
use crate::runtime::tg_ws::TgWsRuntimeHandle;
use std::process::Child;

pub struct RuntimeState {
    pub app_state: AppState,
    pub settings: Settings,
    pub logs: Vec<LogLine>,
    pub zapret_child: Option<Child>,
    pub zapret_winws_pids: Vec<u32>,
    pub active_zapret_engine: Option<ZapretEngine>,
    pub tg_ws_runtime: Option<TgWsRuntimeHandle>,
    pub test_running: bool,
    pub test_cancelled: bool,
    pub test_results: Vec<TestResult>,
    pub shutting_down: bool,
}

impl Default for RuntimeState {
    fn default() -> Self {
        Self {
            app_state: AppState::default(),
            settings: Settings::default(),
            logs: Vec::new(),
            zapret_child: None,
            zapret_winws_pids: Vec::new(),
            active_zapret_engine: None,
            tg_ws_runtime: None,
            test_running: false,
            test_cancelled: false,
            test_results: Vec::new(),
            shutting_down: false,
        }
    }
}
