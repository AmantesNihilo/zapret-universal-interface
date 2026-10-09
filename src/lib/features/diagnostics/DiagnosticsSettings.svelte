<script lang="ts">
  import { RotateCw } from "@lucide/svelte";
  import type { Diagnostics } from "$lib/api/types";
  import DiagnosticsPanel from "$lib/components/DiagnosticsPanel.svelte";
  import { t } from "$lib/stores/i18n";

  let {
    diagnostics,
    onRefresh,
    onOpenReport,
    onOpenPath
  }: {
    diagnostics: Diagnostics | null;
    onRefresh: () => void | Promise<void>;
    onOpenReport: () => void | Promise<void>;
    onOpenPath: (path: string) => void | Promise<void>;
  } = $props();
</script>

<section class="diagnostics-workspace">
  <div class="section-title-row diagnostics-title-row">
    <div>
      <h3>{$t("diagnostics.title")}</h3>
      <p class="muted-text">{$t("diagnostics.subtitle")}</p>
    </div>
    <div class="diagnostics-title-actions">
      <button class="icon-button" type="button" title={$t("common.refresh")} onclick={onRefresh}>
        <RotateCw size={16} />
      </button>
      <button class="secondary-button compact-button" type="button" onclick={onOpenReport}>
        {$t("report.title")}
      </button>
    </div>
  </div>
  <DiagnosticsPanel {diagnostics} {onOpenPath} />
</section>
