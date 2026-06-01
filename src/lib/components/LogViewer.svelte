<script lang="ts">
  import type { LogLine } from "$lib/api/types";
  import { t } from "$lib/stores/i18n";

  let { lines = [] }: { lines: LogLine[] } = $props();

  function formatTimestamp(value: string) {
    const seconds = Number(value);
    if (!Number.isFinite(seconds) || seconds <= 0) return value;
    return new Date(seconds * 1000).toLocaleTimeString([], {
      hour: "2-digit",
      minute: "2-digit",
      second: "2-digit"
    });
  }
</script>

<div class="log-viewer" role="log" aria-live="polite">
  {#if lines.length === 0}
    <div class="log-empty">{$t("activity.empty")}</div>
  {:else}
    {#each lines.slice(-80) as line}
      <div class="log-line">
        <span>{formatTimestamp(line.timestamp)}</span>
        <strong>{line.source}</strong>
        <p>{line.message}</p>
      </div>
    {/each}
  {/if}
</div>
