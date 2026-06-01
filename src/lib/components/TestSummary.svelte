<script lang="ts">
  import type { TestResult, TestTargetResult } from "$lib/api/types";
  import { t } from "$lib/stores/i18n";

  let {
    result = null,
    running = false,
    stopping = false,
    presetName = null,
    targets = [],
    onDetails
  }: {
    result?: TestResult | null;
    running?: boolean;
    stopping?: boolean;
    presetName?: string | null;
    targets?: TestTargetResult[];
    onDetails?: (result: TestResult) => void;
  } = $props();

  const passedTargets = $derived(targets.filter((target) => target.ok).length);
  const progress = $derived(targets.length > 0 ? Math.round((passedTargets / targets.length) * 100) : 0);
  const latestTargets = $derived(targets.slice(-4).reverse());

  function chipLabel(label: string) {
    if (label.includes("TLS1.3")) return "TLS 1.3";
    if (label.includes("TLS1.2")) return "TLS 1.2";
    if (label.includes("HTTP1.1")) return "HTTP";
    if (label.includes("Ping")) return "Ping";
    return $t("common.check");
  }
</script>

<div class="test-summary">
  {#if running}
    <div class="score-ring pending" class:stopping>{stopping ? "..." : progress}</div>
    <div>
      <strong>{stopping ? $t("test.stoppingTitle") : (presetName ?? $t("test.running"))}</strong>
      <p>
        {#if stopping}
          {$t("test.stoppingText")}
        {:else}
          {$t("test.finishedPassed", { ok: passedTargets, total: targets.length })}
        {/if}
      </p>
      <div class="test-progress" aria-hidden="true">
        <span style={`width: ${Math.max(8, progress)}%`}></span>
      </div>
      {#if latestTargets.length > 0}
        <div class="live-targets">
          {#each latestTargets as target}
            <span class:failed={!target.ok}>{chipLabel(target.label)}</span>
          {/each}
        </div>
      {/if}
    </div>
  {:else if result}
    <div class="score-ring">{result.score}</div>
    <div>
      <strong>{result.presetName}</strong>
      <p>{$t("test.passed", { ok: result.ok, total: result.total })}</p>
    </div>
    <button class="secondary-button compact-button" type="button" onclick={() => onDetails?.(result)}>{$t("common.details")}</button>
  {:else}
    <div class="score-ring muted">--</div>
    <div>
      <strong>{$t("test.noResults")}</strong>
      <p>{$t("test.runQuickHint")}</p>
    </div>
  {/if}
</div>
