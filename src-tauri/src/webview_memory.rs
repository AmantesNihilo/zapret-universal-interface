use tauri::{AppHandle, Manager};

#[derive(Clone, Copy)]
pub enum MemoryMode {
    Normal,
    Low,
}

pub fn set_main_webview_memory_mode(app: &AppHandle, mode: MemoryMode) {
    let Some(window) = app.get_webview_window("main") else {
        return;
    };

    #[cfg(windows)]
    let _ = window.with_webview(move |webview| {
        use webview2_com::Microsoft::Web::WebView2::Win32::{
            ICoreWebView2_19, COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_LOW,
            COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_NORMAL,
        };
        use windows::core::Interface;

        let Ok(core_webview) = (unsafe { webview.controller().CoreWebView2() }) else {
            return;
        };
        let Ok(memory_webview) = core_webview.cast::<ICoreWebView2_19>() else {
            return;
        };
        let target = match mode {
            MemoryMode::Normal => COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_NORMAL,
            MemoryMode::Low => COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_LOW,
        };
        let _ = unsafe { memory_webview.SetMemoryUsageTargetLevel(target) };
    });

    #[cfg(not(windows))]
    let _ = (window, mode);
}
