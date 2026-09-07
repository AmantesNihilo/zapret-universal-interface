use crate::models::{LogLine, LogSource};
use crate::paths;
use crate::state::RuntimeState;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use tauri::{AppHandle, Emitter};

static TG_WS_LOG_MAX_BYTES: AtomicU64 = AtomicU64::new(5 * 1024 * 1024);

pub fn set_tg_ws_log_max_mb(megabytes: f64) {
    let bytes = (megabytes.clamp(1.0, 1024.0) * 1024.0 * 1024.0) as u64;
    TG_WS_LOG_MAX_BYTES.store(bytes, Ordering::Relaxed);
}

pub fn push(
    app: &AppHandle,
    state: &Mutex<RuntimeState>,
    source: LogSource,
    message: impl Into<String>,
) {
    let line = LogLine {
        source,
        timestamp: timestamp(),
        message: message.into(),
    };

    let _ = append_file(&line);
    {
        let mut runtime = state.lock().unwrap();
        if runtime.logs.len() >= 500 {
            runtime.logs.drain(..100);
        }
        runtime.logs.push(line.clone());
    }
    let _ = app.emit("log_line", line);
}

pub fn clear(state: &Mutex<RuntimeState>) -> Result<(), String> {
    state.lock().unwrap().logs.clear();
    let dir = paths::logs_dir();
    for name in ["app.log", "zapret.log", "tg-ws.log", "tests.log"] {
        let path = dir.join(name);
        if path.exists() {
            std::fs::write(path, "").map_err(|error| error.to_string())?;
        }
    }
    Ok(())
}

pub fn load_recent(limit: usize) -> Vec<LogLine> {
    let mut lines = Vec::new();
    let files = [
        (LogSource::App, "app.log"),
        (LogSource::Zapret, "zapret.log"),
        (LogSource::TgWs, "tg-ws.log"),
        (LogSource::Tests, "tests.log"),
    ];

    for (source, file_name) in files {
        let path = paths::logs_dir().join(file_name);
        let Ok(text) = std::fs::read_to_string(path) else {
            continue;
        };
        for raw in text.lines().rev().take(limit) {
            lines.push(parse_line(source.clone(), raw));
        }
    }

    lines.sort_by(|left, right| left.timestamp.cmp(&right.timestamp));
    if lines.len() > limit {
        lines.split_off(lines.len() - limit)
    } else {
        lines
    }
}

fn append_file(line: &LogLine) -> Result<(), String> {
    let file_name = match line.source {
        LogSource::App => "app.log",
        LogSource::Zapret => "zapret.log",
        LogSource::TgWs => "tg-ws.log",
        LogSource::Tests => "tests.log",
    };
    let logs_dir = paths::logs_dir();
    if !logs_dir.is_dir() {
        std::fs::create_dir_all(&logs_dir).map_err(|error| error.to_string())?;
    }
    let path = logs_dir.join(file_name);
    if matches!(line.source, LogSource::TgWs)
        && std::fs::metadata(&path)
            .map(|metadata| metadata.len() >= TG_WS_LOG_MAX_BYTES.load(Ordering::Relaxed))
            .unwrap_or(false)
    {
        let backup = logs_dir.join("tg-ws.log.1");
        if backup.exists() {
            let _ = std::fs::remove_file(&backup);
        }
        std::fs::rename(&path, backup).map_err(|error| error.to_string())?;
    }
    let text = format!("[{}] {}\n", line.timestamp, line.message);
    std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .and_then(|mut file| {
            use std::io::Write;
            file.write_all(text.as_bytes())
        })
        .map_err(|error| error.to_string())
}

fn parse_line(source: LogSource, raw: &str) -> LogLine {
    if let Some(rest) = raw.strip_prefix('[') {
        if let Some((timestamp, message)) = rest.split_once("] ") {
            return LogLine {
                source,
                timestamp: timestamp.to_string(),
                message: message.to_string(),
            };
        }
    }

    LogLine {
        source,
        timestamp: "0".into(),
        message: raw.into(),
    }
}

fn timestamp() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    now.as_secs().to_string()
}
