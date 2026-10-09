<script lang="ts">
  import { Check, RotateCw, X } from "@lucide/svelte";
  import type { Preset } from "$lib/api/types";
  import { t } from "$lib/stores/i18n";

  let {
    open = false,
    search = "",
    presets = [],
    selectedIds = [],
    onSearchChange,
    onToggle,
    onSelectAll,
    onStart,
    onClose
  }: {
    open?: boolean;
    search?: string;
    presets?: Preset[];
    selectedIds?: string[];
    onSearchChange: (value: string) => void;
    onToggle: (presetId: string) => void;
    onSelectAll: () => void;
    onStart: () => void | Promise<void>;
    onClose: () => void;
  } = $props();
</script>

{#if open}
  <div class="about-overlay workflow-dialog-overlay" role="presentation" onclick={(event) => event.currentTarget === event.target && onClose()}>
    <div class="workflow-dialog test-selection-panel" role="dialog" aria-modal="true" aria-label={$t("test.selected")}>
      <header class="dialog-header">
        <div>
          <h3>{$t("test.choosePresets")}</h3>
          <p>{$t("test.selectedCount", { count: selectedIds.length })}</p>
        </div>
        <button class="icon-button" type="button" onclick={onClose} title={$t("common.close")}>
          <X size={18} />
        </button>
      </header>
      <div class="dialog-body preset-dialog-body">
        <label class="modal-search">
          {$t("common.search")}
          <input
            value={search}
            placeholder={$t("presets.search")}
            oninput={(event) => onSearchChange(event.currentTarget.value)}
          />
        </label>
        <div class="test-preset-picker">
          {#each presets as preset}
            <button
              class:active={selectedIds.includes(preset.id)}
              type="button"
              onclick={() => onToggle(preset.id)}
            >
              <span class="picker-check">
                {#if selectedIds.includes(preset.id)}<Check size={14} />{/if}
              </span>
              <span>
                <strong>{preset.name}</strong>
                <small>{preset.relativePath}</small>
              </span>
            </button>
          {/each}
        </div>
      </div>
      <footer class="dialog-footer">
        <div class="modal-action-row">
          <button class="secondary-button" type="button" onclick={onSelectAll}>
            {$t("test.selectAll")}
          </button>
          <button class="primary-button" type="button" disabled={selectedIds.length === 0} onclick={onStart}>
            <RotateCw size={16} /> {$t("test.startSelected", { count: selectedIds.length })}
          </button>
        </div>
      </footer>
    </div>
  </div>
{/if}
