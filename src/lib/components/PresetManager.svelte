<script lang="ts">
  import { Eye, EyeOff, FolderOpen, Star } from "@lucide/svelte";
  import type { Preset } from "$lib/api/types";
  import { t } from "$lib/stores/i18n";

  let {
    presets = [],
    selectedId = null,
    showHidden = false,
    onSelect,
    onFavorite,
    onHidden,
    onReveal
  }: {
    presets: Preset[];
    selectedId?: string | null;
    showHidden?: boolean;
    onSelect: (preset: Preset) => void;
    onFavorite: (preset: Preset) => void;
    onHidden: (preset: Preset) => void;
    onReveal: (preset: Preset) => void;
  } = $props();

  const visible = $derived(presets.filter((preset) => showHidden || !preset.hidden).slice(0, 40));
</script>

<div class="preset-manager">
  {#if visible.length === 0}
    <p class="muted-text">{$t("presets.noMatch")}</p>
  {:else}
    {#each visible as preset}
      <article class:selected={preset.id === selectedId} class:hidden={preset.hidden} class="preset-row">
        <button class="preset-name" type="button" onclick={() => onSelect(preset)}>
          <strong>{preset.name}</strong>
          <span>{preset.relativePath}</span>
        </button>
        <button class:active={preset.favorite} class="icon-button" type="button" title={$t("common.favorite")} onclick={() => onFavorite(preset)}>
          <Star size={15} />
        </button>
        <button class="icon-button" type="button" title={preset.hidden ? $t("common.unhide") : $t("common.hide")} onclick={() => onHidden(preset)}>
          {#if preset.hidden}
            <Eye size={15} />
          {:else}
            <EyeOff size={15} />
          {/if}
        </button>
        <button class="icon-button" type="button" title={$t("common.reveal")} onclick={() => onReveal(preset)}>
          <FolderOpen size={15} />
        </button>
      </article>
    {/each}
  {/if}
</div>
