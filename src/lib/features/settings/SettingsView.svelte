<script lang="ts">
  import type { Snippet } from "svelte";
  import { X } from "@lucide/svelte";
  import { t } from "$lib/stores/i18n";
  import type { SettingsTab } from "./types";

  const tabOrder: SettingsTab[] = ["general", "services", "test", "presets", "diagnostics"];

  let {
    tab,
    children,
    onTabChange,
    onClose,
    onWindowDrag
  }: {
    tab: SettingsTab;
    children: Snippet;
    onTabChange: (tab: SettingsTab) => void;
    onClose: () => void;
    onWindowDrag: (event: PointerEvent) => void;
  } = $props();

  const activeIndex = $derived(Math.max(0, tabOrder.indexOf(tab)));
</script>

<section class="settings-view">
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="view-heading" data-tauri-drag-region onpointerdown={onWindowDrag}>
    <h2>{$t("common.settings")}</h2>
    <button class="icon-button" type="button" onclick={onClose} title={$t("common.close")}>
      <X size={18} />
    </button>
  </div>

  <nav class="settings-tabs" style={`--active-index: ${activeIndex}`} aria-label={$t("common.settings")}>
    <span class="settings-tab-indicator" aria-hidden="true"></span>
    <button
      type="button"
      data-tab="general"
      class:active={tab === "general"}
      onclick={() => onTabChange("general")}
    >
      {$t("settings.general")}
    </button>
    <button
      type="button"
      data-tab="services"
      class:active={tab === "services"}
      onclick={() => onTabChange("services")}
    >
      {$t("settings.services")}
    </button>
    <button
      type="button"
      data-tab="test"
      class:active={tab === "test"}
      onclick={() => onTabChange("test")}
    >
      {$t("settings.test")}
    </button>
    <button
      type="button"
      data-tab="presets"
      class:active={tab === "presets"}
      onclick={() => onTabChange("presets")}
    >
      {$t("settings.presets")}
    </button>
    <button
      type="button"
      data-tab="diagnostics"
      class:active={tab === "diagnostics"}
      onclick={() => onTabChange("diagnostics")}
    >
      {$t("settings.checks")}
    </button>
  </nav>

  <div class="settings-grid">
    {@render children()}
  </div>
</section>
