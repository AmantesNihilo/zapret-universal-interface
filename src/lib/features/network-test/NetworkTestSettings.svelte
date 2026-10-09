<script lang="ts">
  import { Check, ChevronDown, Cpu, Download, Pencil, RotateCw, Shield, Upload, X } from "@lucide/svelte";
  import { slide } from "svelte/transition";
  import type { Preset, TestResult, TestTargetConfig, ZapretEngine } from "$lib/api/types";
  import TestRecommendations from "$lib/components/TestRecommendations.svelte";
  import TestSummary from "$lib/components/TestSummary.svelte";
  import { settings } from "$lib/stores/settings";
  import { currentProgress, currentTargets, testResults, testRunning, testStopping } from "$lib/stores/tests";
  import { t } from "$lib/stores/i18n";
  import { formatUnixTime } from "$lib/shared/lib/date";

  type TargetGroup = { service: string; targets: TestTargetConfig[] };

  let {
    activeEngine,
    changingEngine,
    engineTransition,
    serviceConfigLocked,
    enginePresets,
    latestResult,
    displayedResults,
    effectiveTargets,
    targetGroups,
    realityExpanded,
    targetsExpanded,
    resultsExpanded,
    onChooseEngine,
    onBeginEngineChange,
    onExport,
    onImport,
    onDetails,
    onTestAll,
    onChoosePresets,
    onStop,
    onRealityExpandedChange,
    onResultsExpandedChange,
    onTargetsExpandedChange,
    onUseResult,
    onEditTargets
  }: {
    activeEngine: ZapretEngine | null;
    changingEngine: boolean;
    engineTransition: ZapretEngine | null;
    serviceConfigLocked: boolean;
    enginePresets: Preset[];
    latestResult: TestResult | null;
    displayedResults: TestResult[];
    effectiveTargets: TestTargetConfig[];
    targetGroups: TargetGroup[];
    realityExpanded: boolean;
    targetsExpanded: boolean;
    resultsExpanded: boolean;
    onChooseEngine: (engine: ZapretEngine) => void | Promise<void>;
    onBeginEngineChange: () => void;
    onExport: () => void | Promise<void>;
    onImport: () => void | Promise<void>;
    onDetails: (result: TestResult) => void;
    onTestAll: () => void | Promise<void>;
    onChoosePresets: () => void;
    onStop: () => void | Promise<void>;
    onRealityExpandedChange: (value: boolean) => void;
    onResultsExpandedChange: (value: boolean) => void;
    onTargetsExpandedChange: (value: boolean) => void;
    onUseResult: (result: TestResult) => void | Promise<void>;
    onEditTargets: () => void;
  } = $props();
</script>

<section class="test-section">
  {#if !activeEngine || changingEngine}
    <div class:transitioning={engineTransition !== null} class="engine-onboarding">
      <div class="engine-onboarding-head">
        <div>
          <h3>{$t("engine.chooseTitle")}</h3>
          <p>{$t("engine.chooseText")}</p>
        </div>
      </div>
      <div class="engine-choice-grid">
        <button
          class:active={activeEngine === "classic"}
          class:selected-transition={engineTransition === "classic"}
          class:transition-muted={engineTransition !== null && engineTransition !== "classic"}
          class="engine-choice-card"
          type="button"
          disabled={serviceConfigLocked || engineTransition !== null}
          onclick={() => onChooseEngine("classic")}
        >
          <span class="engine-choice-icon"><Shield size={23} /></span>
          <span>
            <strong>Zapret Classic</strong>
            <small>{$t("engine.classicDescription")}</small>
          </span>
          {#if activeEngine === "classic"}<Check size={18} />{/if}
        </button>
        <button
          class:active={activeEngine === "zapret2"}
          class:selected-transition={engineTransition === "zapret2"}
          class:transition-muted={engineTransition !== null && engineTransition !== "zapret2"}
          class="engine-choice-card"
          type="button"
          disabled={serviceConfigLocked || engineTransition !== null}
          onclick={() => onChooseEngine("zapret2")}
        >
          <span class="engine-choice-icon"><Cpu size={23} /></span>
          <span>
            <strong>Zapret 2</strong>
            <small>{$t("engine.zapret2Description")}</small>
          </span>
          {#if activeEngine === "zapret2"}<Check size={18} />{/if}
        </button>
      </div>
      <div class="engine-choice-note">
        <strong>{$t("engine.whyChoice")}</strong>
        <span>{$t("engine.whyChoiceText")}</span>
      </div>
    </div>
  {:else}
    <div class="test-workspace">
      <div class="section-title-row">
        <div>
          <h3>{$t("settings.test")}</h3>
          <p class="muted-text">
            {activeEngine === "zapret2" ? "Zapret 2" : "Zapret Classic"} · {enginePresets.length} {$t("engine.presetsAvailable")}
          </p>
        </div>
        <div class="test-head-actions">
          <button class="secondary-button compact-button" type="button" disabled={$testRunning || $testStopping} onclick={onBeginEngineChange}>
            <Cpu size={15} /> {$t("engine.change")}
          </button>
          <button class="icon-button" type="button" title={$t("test.export")} disabled={$testRunning || $testStopping || $testResults.length === 0} onclick={onExport}>
            <Download size={16} />
          </button>
          <button class="icon-button" type="button" title={$t("test.import")} disabled={$testRunning || $testStopping} onclick={onImport}>
            <Upload size={16} />
          </button>
        </div>
      </div>

      <TestSummary
        result={latestResult}
        running={$testRunning}
        stopping={$testStopping}
        progress={$currentProgress}
        targets={$currentTargets}
        onDetails={onDetails}
      />

      <div class="test-mode-grid two-modes">
        <article class="test-mode-card accent">
          <div>
            <strong>{$t("test.all")}</strong>
            <span>{$t("test.allHintNew")}</span>
          </div>
          <button class="primary-button" type="button" disabled={$testRunning || $testStopping || enginePresets.length === 0} onclick={onTestAll}>
            <RotateCw size={16} /> {$t("test.start")}
          </button>
        </article>
        <article class="test-mode-card">
          <div>
            <strong>{$t("test.selected")}</strong>
            <span>{$t("test.selectedHint")}</span>
          </div>
          <button class="secondary-button" type="button" disabled={$testRunning || $testStopping || enginePresets.length === 0} onclick={onChoosePresets}>
            <Check size={16} /> {$t("test.choose")}
          </button>
        </article>
      </div>

      {#if $testRunning || $testStopping}
        <button class="secondary-button stop-test-button" type="button" disabled={!$testRunning || $testStopping} onclick={onStop}>
          <X size={16} /> {$testStopping ? $t("test.stopping") : $t("test.stop")}
        </button>
      {/if}

      <details class="test-note" open={realityExpanded} ontoggle={(event) => onRealityExpandedChange(event.currentTarget.open)}>
        <summary>
          <strong>{$t("test.realityTitle")}</strong>
          <ChevronDown size={16} />
        </summary>
        <span>{$t("test.realityText")}</span>
      </details>

      {#if displayedResults.length > 0}
        <section class="test-results-section">
          <button class="test-results-toggle" type="button" aria-expanded={resultsExpanded} onclick={() => onResultsExpandedChange(!resultsExpanded)}>
            <span>
              <strong>{$t("test.savedResults")}</strong>
              <small>
                {$t("test.completedAt", { time: formatUnixTime(displayedResults[0].finishedAt, $settings.language) })}
                · {displayedResults.length} {$t("test.resultsCount")}
              </small>
            </span>
            <ChevronDown size={17} />
          </button>
          {#if resultsExpanded}
            <div class="collapsible-transition" transition:slide={{ duration: 220 }}>
              <TestRecommendations results={displayedResults} onUse={onUseResult} onDetails={onDetails} />
            </div>
          {/if}
        </section>
      {/if}

      <details class="test-targets-card" open={targetsExpanded} ontoggle={(event) => onTargetsExpandedChange(event.currentTarget.open)}>
        <summary class="test-targets-head">
          <div>
            <strong>{$t("test.targets")}</strong>
            <span>{effectiveTargets.filter((target) => target.enabled).length} {$t("test.targetsEnabled")}</span>
          </div>
          <span class="test-target-actions">
            <button
              class="icon-button"
              type="button"
              title={$t("test.editTargets")}
              disabled={$testRunning || $testStopping}
              onclick={(event) => {
                event.preventDefault();
                onEditTargets();
              }}
            >
              <Pencil size={16} />
            </button>
            <ChevronDown size={17} />
          </span>
        </summary>
        <div class="test-target-groups">
          {#each targetGroups as group}
            <article>
              <span class="target-service-dot"></span>
              <div>
                <strong>{group.service}</strong>
                <small>{group.targets.filter((target) => target.enabled).length}/{group.targets.length} {$t("test.active")}</small>
              </div>
            </article>
          {/each}
        </div>
      </details>
    </div>
  {/if}
</section>
