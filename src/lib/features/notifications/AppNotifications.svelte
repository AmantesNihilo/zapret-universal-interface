<script lang="ts">
  import { AlertTriangle, X } from "@lucide/svelte";
  import { t } from "$lib/stores/i18n";

  let {
    error = null,
    abortReason = null,
    stopping = false,
    onCloseError,
    onCloseAbort
  }: {
    error?: string | null;
    abortReason?: string | null;
    stopping?: boolean;
    onCloseError: () => void;
    onCloseAbort: () => void;
  } = $props();
</script>

{#if error}
  <div class="notification-region" aria-live="assertive" aria-atomic="true">
    <article class="app-notification notification-error">
      <span class="notification-icon" aria-hidden="true"><AlertTriangle size={18} /></span>
      <div class="notification-content">
        <strong>{$t("common.error")}</strong>
        <p>{error}</p>
      </div>
      <button class="notification-close" type="button" onclick={onCloseError} title={$t("common.close")}>
        <X size={15} />
      </button>
      <span class="notification-timer" aria-hidden="true"></span>
    </article>
  </div>
{/if}

{#if abortReason}
  <div class="notification-region" aria-live="assertive" aria-atomic="true">
    <article class="app-notification notification-warning">
      <span class="notification-icon" aria-hidden="true"><AlertTriangle size={18} /></span>
      <div class="notification-content">
        <strong>{$t("test.abortedTitle")}</strong>
        <p>{abortReason === "processExited" ? $t("test.processExitedExternal") : $t("test.abortedGeneric")}</p>
      </div>
      <button class="notification-close" type="button" onclick={onCloseAbort} title={$t("common.close")}>
        <X size={15} />
      </button>
    </article>
  </div>
{/if}

{#if stopping}
  <div class="wait-overlay" role="presentation" aria-live="polite">
    <div class="wait-panel">
      <div class="wait-orbit" aria-hidden="true">
        <span></span>
        <span></span>
      </div>
      <strong>{$t("test.stoppingTitle")}</strong>
      <p>{$t("test.stoppingText")}</p>
    </div>
  </div>
{/if}
