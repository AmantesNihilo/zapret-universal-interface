<script lang="ts">
  import type { ConflictProcess } from "$lib/api/types";
  import { t } from "$lib/stores/i18n";

  let {
    conflicts = [],
    onCancel,
    onIgnore,
    onKillAndStart
  }: {
    conflicts?: ConflictProcess[];
    onCancel: () => void;
    onIgnore: () => void | Promise<void>;
    onKillAndStart: () => void | Promise<void>;
  } = $props();
</script>

{#if conflicts.length > 0}
  <div class="conflict-overlay" role="presentation">
    <div class="conflict-panel" role="dialog" aria-modal="true" aria-label={$t("conflicts.title")}>
      <div>
        <h3>{$t("conflicts.title")}</h3>
        <p>{$t("conflicts.text")}</p>
      </div>
      <div class="conflict-list">
        {#each conflicts as process}
          <div class="conflict-row">
            <strong>{process.image}</strong>
            <span>PID {process.pid}{process.title ? ` - ${process.title}` : ""}</span>
          </div>
        {/each}
      </div>
      <div class="conflict-actions">
        <button class="secondary-button" type="button" onclick={onCancel}>
          {$t("common.cancel")}
        </button>
        <button class="secondary-button" type="button" onclick={onIgnore}>
          {$t("common.ignore")}
        </button>
        <button class="primary-button" type="button" onclick={onKillAndStart}>
          {$t("conflicts.killStart")}
        </button>
      </div>
    </div>
  </div>
{/if}
