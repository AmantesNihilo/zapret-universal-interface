<script lang="ts">
  import { onMount } from "svelte";
  import { Check, ChevronDown, Search } from "@lucide/svelte";
  import type { Preset } from "$lib/api/types";
  import { t } from "$lib/stores/i18n";

  let {
    presets = [],
    selectedId = null,
    placeholder,
    emptyText,
    disabled = false,
    onSelect
  }: {
    presets: Preset[];
    selectedId?: string | null;
    placeholder?: string;
    emptyText?: string;
    disabled?: boolean;
    onSelect: (presetId: string | null) => void;
  } = $props();

  let open = $state(false);
  let query = $state("");
  let root: HTMLDivElement;

  const selected = $derived(presets.find((preset) => preset.id === selectedId) ?? null);
  const filteredPresets = $derived.by(() => {
    const needle = query.trim().toLowerCase();
    if (!needle) return presets.slice(0, 220);
    return presets
      .filter((preset) => {
        return (
          preset.name.toLowerCase().includes(needle) ||
          preset.relativePath.toLowerCase().includes(needle)
        );
      })
      .slice(0, 220);
  });

  onMount(() => {
    const closeOutside = (event: PointerEvent) => {
      if (!root?.contains(event.target as Node)) {
        open = false;
      }
    };
    const closeOnEscape = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        open = false;
      }
    };

    document.addEventListener("pointerdown", closeOutside);
    document.addEventListener("keydown", closeOnEscape);

    return () => {
      document.removeEventListener("pointerdown", closeOutside);
      document.removeEventListener("keydown", closeOnEscape);
    };
  });

  function choose(presetId: string | null) {
    onSelect(presetId);
    open = false;
  }
</script>

<div class="preset-dropdown" bind:this={root}>
  <button
    class:open
    class="preset-dropdown-button"
    type="button"
    disabled={disabled}
    aria-expanded={open}
    onclick={() => (open = !open)}
  >
    <span>
      <strong>{selected?.name ?? placeholder ?? $t("presets.notSelected")}</strong>
      {#if selected}
        <small>{selected.relativePath}</small>
      {/if}
    </span>
    <ChevronDown class="preset-dropdown-chevron" size={17} />
  </button>

  {#if open}
    <div class="preset-dropdown-menu">
      <label class="preset-dropdown-search">
        <Search size={15} />
        <input
          placeholder={$t("presets.search")}
          value={query}
          oninput={(event) => (query = event.currentTarget.value)}
        />
      </label>

      <button class:active={selectedId === null} type="button" onclick={() => choose(null)}>
        <span>
          <strong>{$t("presets.notSelected")}</strong>
          <small>{$t("presets.required")}</small>
        </span>
        {#if selectedId === null}
          <Check size={15} />
        {/if}
      </button>

      {#if filteredPresets.length === 0}
        <p>{emptyText ?? $t("zapret.noPresetsFolder")}</p>
      {:else}
        {#each filteredPresets as preset}
          <button class:active={preset.id === selectedId} type="button" onclick={() => choose(preset.id)}>
            <span>
              <strong>{preset.name}</strong>
              <small>{preset.relativePath}</small>
            </span>
            {#if preset.id === selectedId}
              <Check size={15} />
            {/if}
          </button>
        {/each}
      {/if}
    </div>
  {/if}
</div>
