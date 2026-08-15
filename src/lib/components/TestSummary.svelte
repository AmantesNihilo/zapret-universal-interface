<script lang="ts">
  import type { TestProgress, TestResult, TestTargetResult } from "$lib/api/types";
  import { t } from "$lib/stores/i18n";

  let {
    result = null,
    running = false,
    stopping = false,
    progress = null,
    targets = [],
    onDetails
  }: {
    result?: TestResult | null;
    running?: boolean;
    stopping?: boolean;
    progress?: TestProgress | null;
    targets?: TestTargetResult[];
    onDetails?: (result: TestResult) => void;
  } = $props();

  const completed = $derived(progress?.completedChecks ?? 0);
  const total = $derived(progress?.totalChecks ?? 0);
  const percent = $derived(total > 0 ? Math.min(100, Math.round((completed / total) * 100)) : 0);
  const batchPercent = $derived(
    progress && progress.presetCount > 0
      ? Math.min(
          100,
          Math.round(
            (((progress.presetIndex - 1) + (total > 0 ? completed / total : 0)) /
              progress.presetCount) *
              100
          )
        )
      : 0
  );
  const latestTargets = $derived(targets.slice(-3).reverse());

  function phaseLabel(value?: TestProgress["phase"]) {
    if (stopping) return $t("test.stoppingTitle");
    if (value === "starting") return $t("test.phaseStarting");
    if (value === "warmup") return $t("test.phaseWarmup");
    if (value === "finishing") return $t("test.phaseFinishing");
    return $t("test.phaseChecking");
  }

  function chipLabel(label: string) {
    if (label.includes("TLS1.3")) return "TLS 1.3";
    if (label.includes("TLS1.2")) return "TLS 1.2";
    if (label.includes("HTTP1.1")) return "HTTP";
    if (label.includes("Ping")) return "Ping";
    return $t("common.check");
  }

  function engineLabel(value: TestResult["engine"]) {
    return value === "zapret2" ? "Zapret 2" : "Classic";
  }

  function resultTime(value: string) {
    const timestamp = Number(value);
    if (!Number.isFinite(timestamp) || timestamp <= 0) return value;
    return new Date(timestamp * 1000).toLocaleString([], {
      day: "2-digit",
      month: "2-digit",
      hour: "2-digit",
      minute: "2-digit"
    });
  }
</script>

<div class:running class="test-summary">
  {#if running || stopping}
    <div
      class:stopping
      class="test-progress-orb"
      style={`--test-progress: ${percent * 3.6}deg`}
      aria-label={`${percent}%`}
    >
      <span>{stopping ? "..." : `${percent}%`}</span>
    </div>
    <div class="test-summary-live">
      <div class="test-summary-heading">
        <strong>{progress?.presetName ?? $t("test.preparing")}</strong>
        {#if progress}
          <span>{$t("test.presetPosition", { current: progress.presetIndex, total: progress.presetCount })}</span>
        {/if}
      </div>
      <p>
        <b>{phaseLabel(progress?.phase)}</b>
        {#if progress?.currentTarget}
          <span> · {progress.currentTarget}</span>
        {/if}
      </p>
      <div class="test-progress" aria-hidden="true">
        <span style={`width: ${percent}%`}></span>
      </div>
      <div class="test-live-counters">
        <span>{$t("test.completed", { completed, total })}</span>
        <span class="passed">{$t("test.successCount", { count: progress?.passedChecks ?? 0 })}</span>
        <span class="failed">Fail {progress?.failedChecks ?? 0}</span>
      </div>
      {#if progress && progress.presetCount > 1}
        <small class="batch-progress">
          {$t("test.batchProgress", { percent: batchPercent })}
        </small>
      {/if}
      {#if latestTargets.length > 0}
        <div class="live-targets">
          {#each latestTargets as target}
            <span class:failed={!target.ok}>{chipLabel(target.label)}</span>
          {/each}
        </div>
      {/if}
    </div>
  {:else if result}
    <div
      class:partial={result.recommendation === "partial"}
      class:failed={result.recommendation === "notRecommended"}
      class="score-ring"
    >
      {result.score}
    </div>
    <div class="test-summary-result">
      <strong>{result.presetName}</strong>
      <p>{engineLabel(result.engine)} · {$t("test.passed", { ok: result.ok, total: result.total })}</p>
      <small class="test-result-time">{$t("test.completedAt", { time: resultTime(result.finishedAt) })}</small>
    </div>
    <button class="secondary-button compact-button" type="button" onclick={() => onDetails?.(result)}>
      {$t("common.details")}
    </button>
  {:else}
    <div class="score-ring muted">--</div>
    <div>
      <strong>{$t("test.noResults")}</strong>
      <p>{$t("test.runTestsHint")}</p>
    </div>
  {/if}
</div>
