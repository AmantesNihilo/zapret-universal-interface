use super::manifest;
use crate::models::{
    BaselineSnapshot, ProbeChange, ProbeStatus, ServiceTestResult, TestRecommendation,
    TestServiceStatus, TestTargetResult,
};

pub struct ScoreSummary {
    pub score: u8,
    pub recommendation: TestRecommendation,
    pub ok: u32,
    pub total: u32,
    pub passed_weight: u32,
    pub total_weight: u32,
    pub inconclusive: u32,
    pub regressions: u32,
}

pub fn apply_baseline_comparison(
    targets: &mut [TestTargetResult],
    baseline: Option<&BaselineSnapshot>,
) {
    for target in targets {
        let Some(baseline_target) = baseline.and_then(|snapshot| {
            snapshot.targets.iter().find(|candidate| {
                (!target.target_id.is_empty() && candidate.target_id == target.target_id)
                    || (candidate.service == target.service
                        && candidate.label == target.label
                        && candidate.url == target.url)
            })
        }) else {
            target.change = ProbeChange::NotCompared;
            continue;
        };
        target.change = compare(baseline_target.probe_status, target.probe_status);
    }
}

fn compare(before: ProbeStatus, after: ProbeStatus) -> ProbeChange {
    match (before, after) {
        (ProbeStatus::Failed, ProbeStatus::Passed) => ProbeChange::Unblocked,
        (ProbeStatus::Passed, ProbeStatus::Passed) => ProbeChange::UnchangedAvailable,
        (ProbeStatus::Failed, ProbeStatus::Failed) => ProbeChange::UnchangedBlocked,
        (ProbeStatus::Passed, ProbeStatus::Failed) => ProbeChange::Regressed,
        _ => ProbeChange::Inconclusive,
    }
}

pub fn build_services(targets: &[TestTargetResult]) -> Vec<ServiceTestResult> {
    let mut names: Vec<String> = [
        "Zapret",
        "Discord",
        "YouTube",
        "Google",
        "Cloudflare",
        "DNS",
    ]
    .into_iter()
    .map(str::to_string)
    .collect();
    for target in targets {
        if !names.iter().any(|name| name == &target.service) {
            names.push(target.service.clone());
        }
    }

    names
        .into_iter()
        .filter_map(|name| {
            let service_targets: Vec<TestTargetResult> = targets
                .iter()
                .filter(|target| target.service == name)
                .cloned()
                .collect();
            if service_targets.is_empty() {
                return None;
            }
            let scored: Vec<&TestTargetResult> = service_targets
                .iter()
                .filter(|target| !target.diagnostic && target.weight > 0)
                .collect();
            let ok = scored
                .iter()
                .filter(|target| target.probe_status == ProbeStatus::Passed)
                .count() as u32;
            let failed = scored
                .iter()
                .filter(|target| target.probe_status == ProbeStatus::Failed)
                .count() as u32;
            let total = ok + failed;
            let passed_weight = scored
                .iter()
                .filter(|target| target.probe_status == ProbeStatus::Passed)
                .map(|target| target.weight as u32)
                .sum::<u32>();
            let failed_weight = scored
                .iter()
                .filter(|target| target.probe_status == ProbeStatus::Failed)
                .map(|target| target.weight as u32)
                .sum::<u32>();
            let total_weight = passed_weight + failed_weight;
            let inconclusive = scored
                .iter()
                .filter(|target| {
                    matches!(
                        target.probe_status,
                        ProbeStatus::Inconclusive | ProbeStatus::Cancelled
                    )
                })
                .count() as u32;
            let regressions = scored
                .iter()
                .filter(|target| target.change == ProbeChange::Regressed)
                .count() as u32;
            let required_failed = scored
                .iter()
                .any(|target| target.required && target.probe_status == ProbeStatus::Failed);
            let required_incomplete = scored
                .iter()
                .any(|target| target.required && target.probe_status != ProbeStatus::Passed);
            let mut score = passed_weight
                .saturating_mul(100)
                .checked_div(total_weight)
                .unwrap_or(0) as u8;
            if required_failed {
                score = score.min(69);
            }
            if regressions > 0 {
                score = score.saturating_sub((regressions.min(3) * 10) as u8);
            }
            let status = if total == 0 || (required_incomplete && !required_failed) {
                TestServiceStatus::Partial
            } else if score >= 90 && !required_incomplete {
                TestServiceStatus::Passed
            } else if ok > 0 {
                TestServiceStatus::Partial
            } else {
                TestServiceStatus::Failed
            };
            let errors = service_targets
                .iter()
                .filter_map(|target| target.error.clone())
                .collect();
            Some(ServiceTestResult {
                name,
                status,
                ok,
                total,
                score,
                passed_weight,
                total_weight,
                inconclusive,
                regressions,
                errors,
                targets: service_targets,
            })
        })
        .collect()
}

pub fn summarize(services: &[ServiceTestResult], process_ok: bool) -> ScoreSummary {
    let ok = services.iter().map(|service| service.ok).sum();
    let total = services.iter().map(|service| service.total).sum();
    let passed_weight = services.iter().map(|service| service.passed_weight).sum();
    let total_weight = services.iter().map(|service| service.total_weight).sum();
    let inconclusive = services.iter().map(|service| service.inconclusive).sum();
    let regressions = services.iter().map(|service| service.regressions).sum();

    let (weighted_score, service_weight) = services
        .iter()
        .filter(|service| service.name != "Zapret" && service.total_weight > 0)
        .fold((0u32, 0u32), |(score, weight), service| {
            let service_weight = manifest::service_weight(&service.name);
            (
                score + service.score as u32 * service_weight,
                weight + service_weight,
            )
        });
    let mut score = weighted_score.checked_div(service_weight).unwrap_or(0) as u8;
    if !process_ok {
        score = 0;
    }
    let core_failed = services.iter().any(|service| {
        matches!(service.name.as_str(), "Discord" | "YouTube")
            && matches!(service.status, TestServiceStatus::Failed)
    });
    let core_required_incomplete = services.iter().any(|service| {
        matches!(service.name.as_str(), "Discord" | "YouTube")
            && service
                .targets
                .iter()
                .any(|target| target.required && target.probe_status != ProbeStatus::Passed)
    });
    let recommendation = if process_ok
        && score >= 70
        && !core_failed
        && !core_required_incomplete
        && regressions == 0
    {
        TestRecommendation::Recommended
    } else if process_ok && (score >= 35 || ok > 0) {
        TestRecommendation::Partial
    } else {
        TestRecommendation::NotRecommended
    };

    ScoreSummary {
        score,
        recommendation,
        ok,
        total,
        passed_weight,
        total_weight,
        inconclusive,
        regressions,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{FailureStage, ProbeContext};

    fn target(status: ProbeStatus, required: bool, weight: u16) -> TestTargetResult {
        TestTargetResult {
            service: "Discord".into(),
            label: "Gateway".into(),
            url: "https://gateway.discord.gg".into(),
            ok: status == ProbeStatus::Passed,
            probe_status: status,
            context: ProbeContext::Preset,
            change: ProbeChange::NotCompared,
            failure_stage: (status == ProbeStatus::Failed).then_some(FailureStage::Http),
            reason_code: None,
            weight,
            required,
            diagnostic: false,
            status: None,
            latency_ms: None,
            error: None,
            ..Default::default()
        }
    }

    #[test]
    fn inconclusive_does_not_reduce_service_score() {
        let services = build_services(&[
            target(ProbeStatus::Passed, false, 10),
            target(ProbeStatus::Inconclusive, false, 50),
        ]);
        assert_eq!(services[0].score, 100);
        assert_eq!(services[0].inconclusive, 1);
    }

    #[test]
    fn required_failure_caps_service_and_regression_is_visible() {
        let mut after = target(ProbeStatus::Failed, true, 10);
        let before = target(ProbeStatus::Passed, true, 10);
        let baseline = BaselineSnapshot {
            captured_at: "1".into(),
            network_fingerprint: "test".into(),
            passed: 1,
            failed: 0,
            inconclusive: 0,
            targets: vec![before],
        };
        apply_baseline_comparison(std::slice::from_mut(&mut after), Some(&baseline));
        let services = build_services(&[after]);
        assert_eq!(services[0].status, TestServiceStatus::Failed);
        assert_eq!(services[0].regressions, 1);
    }

    #[test]
    fn required_core_failure_cannot_be_recommended() {
        let services = build_services(&[
            target(ProbeStatus::Failed, true, 1),
            target(ProbeStatus::Passed, false, 20),
            target(ProbeStatus::Passed, false, 20),
        ]);
        let summary = summarize(&services, true);
        assert!(!matches!(
            summary.recommendation,
            TestRecommendation::Recommended
        ));
    }

    #[test]
    fn required_media_inconclusive_keeps_service_partial() {
        let mut media = target(ProbeStatus::Inconclusive, true, 20);
        media.service = "YouTube".into();
        media.label = "GoogleVideo media range".into();
        let mut web = target(ProbeStatus::Passed, true, 20);
        web.service = "YouTube".into();
        web.label = "YouTube Web".into();
        let services = build_services(&[web, media]);
        assert_eq!(services[0].status, TestServiceStatus::Partial);
        let summary = summarize(&services, true);
        assert!(!matches!(
            summary.recommendation,
            TestRecommendation::Recommended
        ));
    }
}
