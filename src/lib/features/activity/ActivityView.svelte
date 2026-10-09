<script lang="ts">
  import { X } from "@lucide/svelte";
  import { fly } from "svelte/transition";
  import type { AppState, LogLine, ZapretEngine } from "$lib/api/types";
  import LogViewer from "$lib/components/LogViewer.svelte";
  import { t } from "$lib/stores/i18n";

  let {
    source,
    sourceIndex,
    direction,
    lines,
    allLogs,
    appState,
    activeEngine,
    onSelectSource,
    onClear,
    onClose,
    onWindowDrag
  }: {
    source: "app" | "engine" | "tgWs";
    sourceIndex: number;
    direction: number;
    lines: LogLine[];
    allLogs: LogLine[];
    appState: AppState;
    activeEngine: ZapretEngine | null;
    onSelectSource: (source: "app" | "engine" | "tgWs") => void;
    onClear: () => void;
    onClose: () => void;
    onWindowDrag: (event: PointerEvent) => void | Promise<void>;
  } = $props();
</script>

<section class="activity-view">
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="view-heading" data-tauri-drag-region onpointerdown={onWindowDrag}>
    <h2>{$t("activity.title")}</h2>
    <button class="icon-button" type="button" onclick={onClose} title={$t("common.close")}>
      <X size={18} />
    </button>
  </div>
  <div class="activity-source-grid" style={`--activity-index: ${sourceIndex}`}>
    <span class="activity-tab-slider" aria-hidden="true"></span>
    <button class:active={source === "app"} type="button" onclick={() => onSelectSource("app")}>
      <span class="activity-source-indicator app"></span>
      <span>
        <strong>App</strong>
        <small>{allLogs.filter((line) => line.source === "app" || line.source === "tests").length} {$t("activity.events")}</small>
      </span>
    </button>
    <button class:active={source === "engine"} type="button" onclick={() => onSelectSource("engine")}>
      <span class="activity-source-indicator engine"></span>
      <span>
        <strong>{activeEngine === "zapret2" ? "Zapret 2" : "Zapret"}</strong>
        <small>{appState.zapret.state} · {allLogs.filter((line) => line.source === "zapret").length} {$t("activity.events")}</small>
      </span>
    </button>
    <button class:active={source === "tgWs"} type="button" onclick={() => onSelectSource("tgWs")}>
      <span class="activity-source-indicator tg"></span>
      <span>
        <strong>tg-ws</strong>
        <small>{appState.tgWs.state} · {allLogs.filter((line) => line.source === "tgWs").length} {$t("activity.events")}</small>
      </span>
    </button>
  </div>
  <div class="activity-log-viewport">
    {#key source}
      <div
        class="activity-log-slide"
        in:fly={{ x: direction * 42, duration: 220, opacity: 0 }}
        out:fly={{ x: direction * -28, duration: 150, opacity: 0 }}
      >
        <div class="activity-log-head">
          <div>
            <strong>{source === "app" ? "ZUI" : source === "engine" ? (activeEngine === "zapret2" ? "Zapret 2" : "Zapret") : "tg-ws"}</strong>
            <span>{$t("activity.latestEvents")}</span>
          </div>
          <button class="secondary-button compact-button" type="button" onclick={onClear}>{$t("activity.clear")}</button>
        </div>
        <LogViewer {lines} emptyText={$t("activity.channelEmpty")} />
      </div>
    {/key}
  </div>
</section>
