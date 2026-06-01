<script lang="ts">
  import { FolderOpen } from "@lucide/svelte";
  import type { Diagnostics } from "$lib/api/types";
  import { t } from "$lib/stores/i18n";

  let {
    diagnostics = null,
    onOpenPath
  }: {
    diagnostics?: Diagnostics | null;
    onOpenPath?: (path: string) => void;
  } = $props();

  const warningTranslations = $derived.by((): Record<string, string> => ({
    "Administrator rights are not detected. zapret may fail to start.": $t("diagnostics.warningAdmin"),
    "No zapret presets were discovered.": $t("diagnostics.warningNoPresets"),
    "Selected zapret preset is missing.": $t("diagnostics.warningPresetMissing"),
    "winws.exe was not found in zapret resources.": $t("diagnostics.warningWinwsMissing"),
    "tg-ws port is busy or unavailable.": $t("diagnostics.warningTgPort")
  }));

  const systemItems = $derived.by(() => {
    if (!diagnostics) return [];
    return [
      { label: $t("diagnostics.admin"), value: statusText(diagnostics.isAdmin), ok: diagnostics.isAdmin },
      { label: "winws.exe", value: statusText(diagnostics.winwsFound), ok: diagnostics.winwsFound },
      { label: $t("diagnostics.presets"), value: String(diagnostics.presetCount), ok: diagnostics.presetCount > 0 }
    ];
  });

  const serviceItems = $derived.by(() => {
    if (!diagnostics) return [];
    return [
      {
        label: $t("diagnostics.tgEngine"),
        value: `${diagnostics.tgWsEngine} ${diagnostics.tgWsEngineVersion}`,
        ok: diagnostics.tgWsFound
      },
      { label: $t("diagnostics.winwsRunning"), value: yesNo(diagnostics.winwsRunning), neutral: true },
      { label: $t("diagnostics.tgRunning"), value: yesNo(diagnostics.tgWsRunning), neutral: true },
      { label: $t("diagnostics.tgPort"), value: statusText(diagnostics.tgWsPortAvailable), ok: diagnostics.tgWsPortAvailable }
    ];
  });

  const pathItems = $derived.by(() => {
    if (!diagnostics) return [];
    return [
      { label: $t("common.resources"), value: diagnostics.resourcesPath },
      { label: $t("common.data"), value: diagnostics.dataPath },
      { label: $t("common.logs"), value: diagnostics.logsPath }
    ];
  });

  function statusText(value: boolean) {
    return value ? $t("common.ok") : $t("common.fail");
  }

  function yesNo(value: boolean) {
    return value ? $t("common.yes") : $t("common.no");
  }

  function warningText(value: string) {
    return warningTranslations[value] ?? value;
  }
</script>

<div class="diagnostics-panel">
  {#if diagnostics}
    <section class="diagnostics-block">
      <header>
        <h4>{$t("diagnostics.system")}</h4>
      </header>
      <div class="diagnostic-pill-grid">
        {#each systemItems as item}
          <div class="diagnostic-pill">
            <span>{item.label}</span>
            <strong class:bad={!item.ok}>{item.value}</strong>
          </div>
        {/each}
      </div>
    </section>

    <section class="diagnostics-block">
      <header>
        <h4>{$t("diagnostics.services")}</h4>
      </header>
      <div class="diagnostic-pill-grid">
        {#each serviceItems as item}
          <div class="diagnostic-pill">
            <span>{item.label}</span>
            <strong class:bad={item.ok === false} class:neutral={item.neutral}>{item.value}</strong>
          </div>
        {/each}
      </div>
    </section>

    <section class="diagnostics-block">
      <header>
        <h4>{$t("diagnostics.paths")}</h4>
      </header>
      <div class="diagnostic-paths">
        {#each pathItems as item}
          <article>
            <div>
              <span>{item.label}</span>
              <strong>{item.value}</strong>
            </div>
            <button class="icon-button" type="button" title={$t("common.reveal")} onclick={() => onOpenPath?.(item.value)}>
              <FolderOpen size={15} />
            </button>
          </article>
        {/each}
      </div>
    </section>

    <section class="diagnostics-block">
      <header>
        <h4>{$t("diagnostics.warnings")}</h4>
        <span>{diagnostics.warnings.length}</span>
      </header>
      {#if diagnostics.warnings.length > 0}
        <div class="warning-list">
          {#each diagnostics.warnings as warning}
            <p>{warningText(warning)}</p>
          {/each}
        </div>
      {:else}
        <div class="diagnostics-ok">
          <strong>{$t("diagnostics.allGood")}</strong>
          <span>{$t("diagnostics.allGoodHint")}</span>
        </div>
      {/if}
    </section>
  {:else}
    <p class="muted-text">{$t("diagnostics.notLoaded")}</p>
  {/if}
</div>
