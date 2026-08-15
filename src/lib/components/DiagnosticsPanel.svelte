<script lang="ts">
  import { AlertTriangle, CheckCircle2, ChevronDown, Cpu, Database, FolderOpen, Radio, ShieldCheck } from "@lucide/svelte";
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
    "winws2.exe was not found in Zapret 2 resources.": $t("diagnostics.warningWinws2Missing"),
    "tg-ws port is busy or unavailable.": $t("diagnostics.warningTgPort")
  }));

  const systemItems = $derived.by(() => {
    if (!diagnostics) return [];
    return [
      { label: $t("diagnostics.admin"), hint: $t("diagnostics.adminHint"), value: statusText(diagnostics.isAdmin), ok: diagnostics.isAdmin },
      { label: "Zapret Classic", hint: $t("diagnostics.classicHint"), value: statusText(diagnostics.winwsFound), ok: diagnostics.winwsFound },
      { label: "Zapret 2", hint: $t("diagnostics.zapret2Hint"), value: statusText(diagnostics.winws2Found), ok: diagnostics.winws2Found },
      { label: $t("diagnostics.presets"), hint: $t("diagnostics.presetsHint"), value: String(diagnostics.presetCount), ok: diagnostics.presetCount > 0 }
    ];
  });

  const serviceItems = $derived.by(() => {
    if (!diagnostics) return [];
    return [
      {
        label: $t("diagnostics.tgEngine"),
        hint: $t("diagnostics.tgEngineHint"),
        value: `${diagnostics.tgWsEngine} ${diagnostics.tgWsEngineVersion}`,
        ok: diagnostics.tgWsFound
      },
      { label: $t("diagnostics.winwsRunning"), hint: $t("diagnostics.processHint"), value: yesNo(diagnostics.winwsRunning), neutral: true },
      { label: $t("diagnostics.winws2Running"), hint: $t("diagnostics.processHint"), value: yesNo(diagnostics.winws2Running), neutral: true },
      { label: $t("diagnostics.tgRunning"), hint: $t("diagnostics.processHint"), value: yesNo(diagnostics.tgWsRunning), neutral: true },
      { label: $t("diagnostics.tgPort"), hint: $t("diagnostics.tgPortHint"), value: statusText(diagnostics.tgWsPortAvailable), ok: diagnostics.tgWsPortAvailable }
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
  const passedChecks = $derived(
    systemItems.filter((item) => item.ok).length +
      serviceItems.filter((item) => item.ok !== false).length
  );
  const totalChecks = $derived(systemItems.length + serviceItems.length);

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
    <section class:warning={diagnostics.warnings.length > 0} class="diagnostics-overview">
      <div class="diagnostics-health-icon">
        {#if diagnostics.warnings.length > 0}
          <AlertTriangle size={22} />
        {:else}
          <ShieldCheck size={22} />
        {/if}
      </div>
      <div>
        <strong>{diagnostics.warnings.length > 0 ? $t("diagnostics.attention") : $t("diagnostics.ready")}</strong>
        <span>{$t("diagnostics.checksPassed", { passed: passedChecks, total: totalChecks })}</span>
        <div class="diagnostics-progress" aria-hidden="true">
          <span style={`width: ${totalChecks > 0 ? Math.round((passedChecks / totalChecks) * 100) : 0}%`}></span>
        </div>
      </div>
      <span class="diagnostics-score">
        <strong>{diagnostics.warnings.length}</strong>
        <small>{$t("diagnostics.issues")}</small>
      </span>
    </section>

    <details class="diagnostics-block diagnostics-check-group" open>
      <summary class="diagnostics-section-title">
        <span class="diagnostics-section-icon"><Cpu size={15} /></span>
        <div>
          <h4>{$t("diagnostics.system")}</h4>
          <small>{$t("diagnostics.systemHint")}</small>
        </div>
        <ChevronDown class="diagnostics-chevron" size={16} />
      </summary>
      <div class="diagnostic-pill-grid">
        {#each systemItems as item}
          <div class:bad={!item.ok} class="diagnostic-pill">
            <span class="diagnostic-state-icon">
              {#if item.ok}<CheckCircle2 size={15} />{:else}<AlertTriangle size={15} />{/if}
            </span>
            <span class="diagnostic-item-copy">
              <strong>{item.label}</strong>
              <small>{item.hint}</small>
            </span>
            <strong>{item.value}</strong>
          </div>
        {/each}
      </div>
    </details>

    <details class="diagnostics-block diagnostics-check-group" open>
      <summary class="diagnostics-section-title">
        <span class="diagnostics-section-icon"><Radio size={15} /></span>
        <div>
          <h4>{$t("diagnostics.services")}</h4>
          <small>{$t("diagnostics.servicesHint")}</small>
        </div>
        <ChevronDown class="diagnostics-chevron" size={16} />
      </summary>
      <div class="diagnostic-pill-grid">
        {#each serviceItems as item}
          <div class:bad={item.ok === false} class="diagnostic-pill">
            <span class="diagnostic-state-icon">
              {#if item.ok === false}<AlertTriangle size={15} />{:else}<CheckCircle2 size={15} />{/if}
            </span>
            <span class="diagnostic-item-copy">
              <strong>{item.label}</strong>
              <small>{item.hint}</small>
            </span>
            <strong class:neutral={item.neutral}>{item.value}</strong>
          </div>
        {/each}
      </div>
    </details>

    <details class="diagnostics-block diagnostics-path-block">
      <summary class="diagnostics-section-title">
        <span class="diagnostics-section-icon"><Database size={15} /></span>
        <div>
          <h4>{$t("diagnostics.paths")}</h4>
          <small>{$t("diagnostics.pathsHint")}</small>
        </div>
        <ChevronDown class="diagnostics-chevron" size={16} />
      </summary>
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
    </details>

    <section class:empty={diagnostics.warnings.length === 0} class="diagnostics-block diagnostics-warnings">
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
