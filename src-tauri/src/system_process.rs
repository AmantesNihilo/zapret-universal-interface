use std::process::{Command, Stdio};

#[derive(Debug, Clone)]
pub struct ProcessInfo {
    pub image: String,
    pub pid: u32,
    pub title: Option<String>,
}

pub fn is_running(image_name: &str) -> bool {
    #[cfg(windows)]
    {
        let output = Command::new("tasklist")
            .args([
                "/FI",
                &format!("IMAGENAME eq {image_name}"),
                "/FO",
                "CSV",
                "/NH",
            ])
            .stdin(Stdio::null())
            .stderr(Stdio::null())
            .creation_flags(0x08000000)
            .output();

        return output
            .ok()
            .map(|output| {
                String::from_utf8_lossy(&output.stdout)
                    .to_lowercase()
                    .contains(&format!("\"{}\"", image_name.to_lowercase()))
            })
            .unwrap_or(false);
    }

    #[cfg(not(windows))]
    {
        Command::new("pgrep")
            .arg(image_name)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|status| status.success())
            .unwrap_or(false)
    }
}

pub fn image_pids(image_name: &str) -> Vec<u32> {
    running_processes_by_names(&[image_name])
        .into_iter()
        .map(|process| process.pid)
        .collect()
}

pub fn is_pid_running(pid: u32) -> bool {
    #[cfg(windows)]
    {
        return Command::new("tasklist")
            .args(["/FI", &format!("PID eq {pid}"), "/FO", "CSV", "/NH"])
            .stdin(Stdio::null())
            .stderr(Stdio::null())
            .creation_flags(0x08000000)
            .output()
            .ok()
            .map(|output| {
                let text = String::from_utf8_lossy(&output.stdout);
                text.lines().any(|line| {
                    parse_csv_line(line)
                        .get(1)
                        .and_then(|value| value.parse::<u32>().ok())
                        .map(|value| value == pid)
                        .unwrap_or(false)
                })
            })
            .unwrap_or(false);
    }

    #[cfg(not(windows))]
    {
        Command::new("kill")
            .args(["-0", &pid.to_string()])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|status| status.success())
            .unwrap_or(false)
    }
}

pub fn running_processes_by_names(image_names: &[&str]) -> Vec<ProcessInfo> {
    #[cfg(windows)]
    {
        let wanted: Vec<String> = image_names
            .iter()
            .map(|name| name.to_ascii_lowercase())
            .collect();
        let output = Command::new("tasklist")
            .args(["/FO", "CSV", "/NH", "/V"])
            .stdin(Stdio::null())
            .stderr(Stdio::null())
            .creation_flags(0x08000000)
            .output();

        let Ok(output) = output else {
            return Vec::new();
        };

        String::from_utf8_lossy(&output.stdout)
            .lines()
            .filter_map(|line| {
                let columns = parse_csv_line(line);
                let image = columns.first()?.to_string();
                if !wanted.iter().any(|name| name.eq_ignore_ascii_case(&image)) {
                    return None;
                }
                let pid = columns.get(1)?.parse().ok()?;
                let title = columns
                    .last()
                    .map(|value| value.trim().to_string())
                    .filter(|value| !value.is_empty() && value != "N/A");
                Some(ProcessInfo { image, pid, title })
            })
            .collect()
    }

    #[cfg(not(windows))]
    {
        let mut processes = Vec::new();
        for image_name in image_names {
            let output = Command::new("pgrep")
                .arg(image_name)
                .stdin(Stdio::null())
                .stderr(Stdio::null())
                .output();
            if let Ok(output) = output {
                for line in String::from_utf8_lossy(&output.stdout).lines() {
                    if let Ok(pid) = line.trim().parse() {
                        processes.push(ProcessInfo {
                            image: (*image_name).into(),
                            pid,
                            title: None,
                        });
                    }
                }
            }
        }
        processes
    }
}

pub fn kill_pid(pid: u32) -> bool {
    #[cfg(windows)]
    {
        return Command::new("taskkill")
            .args(["/PID", &pid.to_string(), "/T", "/F"])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .creation_flags(0x08000000)
            .status()
            .map(|status| status.success())
            .unwrap_or(false);
    }

    #[cfg(not(windows))]
    {
        Command::new("kill")
            .args(["-TERM", &pid.to_string()])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|status| status.success())
            .unwrap_or(false)
    }
}

fn parse_csv_line(line: &str) -> Vec<String> {
    let mut values = Vec::new();
    let mut current = String::new();
    let mut quoted = false;
    let mut chars = line.chars().peekable();

    while let Some(char) = chars.next() {
        match char {
            '"' if quoted && chars.peek() == Some(&'"') => {
                current.push('"');
                let _ = chars.next();
            }
            '"' => quoted = !quoted,
            ',' if !quoted => {
                values.push(current.trim().to_string());
                current.clear();
            }
            _ => current.push(char),
        }
    }
    values.push(current.trim().to_string());
    values
}

#[cfg(windows)]
trait CommandExtHidden {
    fn creation_flags(&mut self, flags: u32) -> &mut Self;
}

#[cfg(windows)]
impl CommandExtHidden for Command {
    fn creation_flags(&mut self, flags: u32) -> &mut Self {
        use std::os::windows::process::CommandExt;
        CommandExt::creation_flags(self, flags);
        self
    }
}
