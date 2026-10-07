use crate::models::{BaselineSnapshot, ProbeStatus, TestTargetResult};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::process::{Command, Stdio};

pub fn network_fingerprint() -> String {
    let mut hasher = DefaultHasher::new();
    for (program, arguments) in [("ipconfig", &["/all"][..]), ("route", &["print", "-4"][..])] {
        let mut command = Command::new(program);
        command
            .args(arguments)
            .stdin(Stdio::null())
            .stderr(Stdio::null());
        hide(&mut command);
        match command.output() {
            Ok(output) => {
                output.status.code().hash(&mut hasher);
                output.stdout.hash(&mut hasher);
            }
            Err(error) => error.kind().hash(&mut hasher),
        }
    }
    format!("{:016x}", hasher.finish())
}

pub fn snapshot(captured_at: String, targets: Vec<TestTargetResult>) -> BaselineSnapshot {
    let passed = targets
        .iter()
        .filter(|target| target.probe_status == ProbeStatus::Passed)
        .count() as u32;
    let failed = targets
        .iter()
        .filter(|target| target.probe_status == ProbeStatus::Failed)
        .count() as u32;
    let inconclusive = targets
        .iter()
        .filter(|target| {
            matches!(
                target.probe_status,
                ProbeStatus::Inconclusive | ProbeStatus::Cancelled
            )
        })
        .count() as u32;
    BaselineSnapshot {
        captured_at,
        network_fingerprint: network_fingerprint(),
        passed,
        failed,
        inconclusive,
        targets,
    }
}

#[cfg(windows)]
fn hide(command: &mut Command) {
    use std::os::windows::process::CommandExt;
    command.creation_flags(0x08000000);
}

#[cfg(not(windows))]
fn hide(_command: &mut Command) {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{ProbeChange, ProbeContext};

    fn target(status: ProbeStatus) -> TestTargetResult {
        TestTargetResult {
            service: "Test".into(),
            label: "Test".into(),
            url: "test".into(),
            ok: status == ProbeStatus::Passed,
            probe_status: status,
            context: ProbeContext::Baseline,
            change: ProbeChange::NotCompared,
            failure_stage: None,
            reason_code: None,
            weight: 1,
            required: false,
            diagnostic: false,
            status: None,
            latency_ms: None,
            error: None,
        }
    }

    #[test]
    fn snapshot_counts_statuses() {
        let snapshot = snapshot(
            "1".into(),
            vec![
                target(ProbeStatus::Passed),
                target(ProbeStatus::Failed),
                target(ProbeStatus::Inconclusive),
            ],
        );
        assert_eq!(
            (snapshot.passed, snapshot.failed, snapshot.inconclusive),
            (1, 1, 1)
        );
    }
}
