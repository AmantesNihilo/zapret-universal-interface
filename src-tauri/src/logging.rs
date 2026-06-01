use crate::models::{LogLine, LogSource};
use crate::paths;
use crate::state::RuntimeState;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter};

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
        runtime.logs.push(line.clone());
        if runtime.logs.len() > 500 {
            runtime.logs.remove(0);
        }
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
    paths::ensure_data_layout().map_err(|error| error.to_string())?;
    let file_name = match line.source {
        LogSource::App => "app.log",
        LogSource::Zapret => "zapret.log",
        LogSource::TgWs => "tg-ws.log",
        LogSource::Tests => "tests.log",
    };
    let text = format!("[{}] {}\n", line.timestamp, line.message);
    std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(paths::logs_dir().join(file_name))
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
