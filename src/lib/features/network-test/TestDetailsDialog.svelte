<script lang="ts">
  import { X } from "@lucide/svelte";
  import type { ProbeCapability, ProbeStatus, TestResult, TestTargetResult } from "$lib/api/types";
  import { t } from "$lib/stores/i18n";
  import { settings } from "$lib/stores/settings";
  import { formatUnixTime } from "$lib/shared/lib/date";

  let {
    result = null,
    onClose
  }: {
    result?: TestResult | null;
    onClose: () => void;
  } = $props();

  function capabilityLabel(capability: ProbeCapability) {
    const labels: Record<ProbeCapability, string> = {
      webApi: $t("test.capabilityWebApi"),
      cdnMedia: $t("test.capabilityCdnMedia"),
      websocket: "WebSocket",
      udpVoice: $t("test.capabilityUdpVoice"),
      http3: "HTTP/3",
      dns: "DNS",
      transport: $t("test.capabilityTransport"),
      generic: $t("common.check")
    };
    return labels[capability] ?? $t("common.check");
  }

  function targetMeta(target: TestTargetResult) {
    const parts = [capabilityLabel(target.capability)];
    if (target.status) parts.push(`HTTP ${target.status}`);
    if (target.negotiatedProtocol) parts.push(target.negotiatedProtocol);
    if (target.tlsVersion) parts.push(target.tlsVersion);
    if (target.alpn) parts.push(`ALPN ${target.alpn}`);
    if (target.latencyMs) parts.push(`${target.latencyMs} ms`);
    if (target.failureStage) parts.push(target.failureStage.toUpperCase());
    return parts.join(" / ");
  }

  function capabilityStatus(targets: TestTargetResult[], capability: ProbeCapability): ProbeStatus | null {
    const matching = targets.filter((target) => target.capability === capability);
    if (matching.length === 0) return null;
    if (matching.some((target) => target.probeStatus === "failed")) return "failed";
    if (matching.some((target) => target.probeStatus === "inconclusive" || target.probeStatus === "cancelled")) return "inconclusive";
    return "passed";
  }

  const matrixCapabilities: ProbeCapability[] = ["webApi", "cdnMedia", "websocket", "udpVoice", "http3", "transport"];

  function probeStatusLabel(status: string) {
    if (status === "passed") return $t("common.ok");
    if (status === "inconclusive") return $t("test.inconclusive");
    if (status === "cancelled") return $t("test.cancelled");
    return $t("common.fail");
  }

  function changeLabel(change: string) {
    if (change === "unblocked") return $t("test.changeUnblocked");
    if (change === "unchangedAvailable") return $t("test.changeUnchangedAvailable");
    if (change === "unchangedBlocked") return $t("test.changeUnchangedBlocked");
    if (change === "regressed") return $t("test.changeRegressed");
    if (change === "inconclusive") return $t("test.inconclusive");
    return "";
  }

  function recommendationLabel(value: TestResult["recommendation"]) {
    if (value === "recommended") return $t("test.recommended");
    if (value === "partial") return $t("test.partial");
    return $t("test.notRecommended");
  }

  function modeLabel(value: TestResult["mode"]) {
    return value === "all" ? $t("test.modeAll") : $t("test.modeSelected");
  }

</script>

{#if result}
  <div class="test-details-overlay" role="presentation" onclick={(event) => event.currentTarget === event.target && onClose()}>
    <div class="test-details-panel" role="dialog" aria-modal="true" aria-label={$t("test.details")}>
      <div class="test-details-head">
        <div>
          <h3>{result.presetName}</h3>
          <p>{recommendationLabel(result.recommendation)} · {modeLabel(result.mode)}</p>
        </div>
        <button class="icon-button" type="button" onclick={onClose} title={$t("common.close")}>
          <X size={18} />
        </button>
      </div>

      <div class="test-details-overview" class:with-baseline={Boolean(result.baseline)}>
        <div class="test-details-score">
          <div class="score-ring" class:partial={result.recommendation === "partial"} class:failed={result.recommendation === "notRecommended"}>{result.score}</div>
          <div>
            <strong>{$t("test.overall")}</strong>
            <span>{$t("test.passed", { ok: result.ok, total: result.total })}</span>
            {#if result.inconclusive > 0}<small>{$t("test.inconclusiveCount", { count: result.inconclusive })}</small>{/if}
            {#if result.regressions > 0}<small>{$t("test.regressionCount", { count: result.regressions })}</small>{/if}
          </div>
        </div>

        {#if result.baseline}
          <div class="test-details-score baseline-summary">
            <div class="score-ring muted">B</div>
            <div>
              <strong>{$t("test.baseline")}</strong>
              <span>{$t("test.baselineSummary", { passed: result.baseline.passed, failed: result.baseline.failed, inconclusive: result.baseline.inconclusive })}</span>
              <small>{$t("test.networkFingerprint", { value: result.baseline.networkFingerprint })}</small>
            </div>
          </div>
        {/if}
      </div>

      <div class="test-details-meta">
        <span><strong>{$t("test.version")}</strong>{result.presetVersion || "-"}</span>
        <span><strong>{$t("test.cachedAt")}</strong>{formatUnixTime(result.cachedAt || result.finishedAt, $settings.language)}</span>
        <span><strong>{$t("test.mode")}</strong>{modeLabel(result.mode)}</span>
      </div>

      <div class="test-details-list">
        {#each result.services as service}
          <article class="test-service-card">
            <header>
              <strong>{service.name}</strong>
              <span class:passed={service.status === "passed"} class:partial={service.status === "partial"} class:failed={service.status === "failed"}>
                {service.ok}/{service.total}
              </span>
            </header>
            <div class="capability-matrix" aria-label={$t("test.capabilityMatrix")}>
              {#each matrixCapabilities as capability}
                {@const status = capabilityStatus(service.targets, capability)}
                {#if status}
                  <div class:passed={status === "passed"} class:failed={status === "failed"} class:inconclusive={status === "inconclusive"}>
                    <span>{capabilityLabel(capability)}</span>
                    <strong>{probeStatusLabel(status)}</strong>
                  </div>
                {/if}
              {/each}
            </div>
            <div>
              {#each service.targets as target}
                <div class:failed={target.probeStatus === "failed"} class:inconclusive={target.probeStatus === "inconclusive"} class="target-row">
                  <span>{probeStatusLabel(target.probeStatus)}</span>
                  <div class="target-body">
                    <div class="target-title-line">
                      <strong>{target.label || target.url}</strong>
                      <small class="protocol-chip">{capabilityLabel(target.capability)}</small>
                    </div>
                    <small>{target.url}</small>
                    {#if target.finalUrl && target.finalUrl !== target.url}<small>{$t("test.finalUrl")}: {target.finalUrl}</small>{/if}
                    <small>{targetMeta(target)}</small>
                    {#if target.contentType}<small>{target.contentType}{target.bytesRead ? ` · ${target.bytesRead} B` : ""}</small>{/if}
                    {#if changeLabel(target.change)}<small class:failed={target.change === "regressed"}>{changeLabel(target.change)}</small>{/if}
                    {#if target.steps?.length}
                      <div class="probe-steps">
                        {#each target.steps as step}
                          <span class:passed={step.status === "passed"} class:failed={step.status === "failed"} class:inconclusive={step.status === "inconclusive"}>
                            <strong>{step.stage.toUpperCase()}</strong>
                            <small>{step.detail}{step.latencyMs != null ? ` · ${step.latencyMs} ms` : ""}</small>
                          </span>
                        {/each}
                      </div>
                    {/if}
                  </div>
                  {#if target.error}
                    <em>{target.error}</em>
                  {/if}
                </div>
              {/each}
            </div>
          </article>
        {/each}
      </div>
    </div>
  </div>
{/if}
