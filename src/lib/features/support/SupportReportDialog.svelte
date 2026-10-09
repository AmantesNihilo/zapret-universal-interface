<script lang="ts">
  import { X } from "@lucide/svelte";
  import { t } from "$lib/stores/i18n";

  let {
    open = false,
    appVersion,
    reportText = "",
    copied = false,
    getReport,
    onCopy,
    onClose
  }: {
    open?: boolean;
    appVersion: string;
    reportText?: string;
    copied?: boolean;
    getReport: () => string;
    onCopy: () => void | Promise<void>;
    onClose: () => void;
  } = $props();
</script>

{#if open}
  <div class="about-overlay" role="presentation" onclick={(event) => event.currentTarget === event.target && onClose()}>
    <div class="about-panel report-panel" role="dialog" aria-modal="true" aria-label={$t("report.title")}>
      <header>
        <div>
          <h3>{$t("report.title")}</h3>
          <p>ZUI {appVersion}</p>
        </div>
        <button class="icon-button" type="button" onclick={onClose} title={$t("common.close")}>
          <X size={18} />
        </button>
      </header>
      <textarea readonly rows="18" value={reportText || getReport()}></textarea>
      <button class="secondary-button" type="button" onclick={onCopy}>
        {copied ? $t("report.copied") : $t("report.copy")}
      </button>
    </div>
  </div>
{/if}
