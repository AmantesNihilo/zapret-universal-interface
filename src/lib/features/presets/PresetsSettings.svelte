<script lang="ts">
  import { FolderOpen, X } from "@lucide/svelte";
  import type { Preset, Profile } from "$lib/api/types";
  import PresetManager from "$lib/components/PresetManager.svelte";
  import { settings } from "$lib/stores/settings";
  import { t } from "$lib/stores/i18n";

  let {
    presets,
    selectedPreset,
    activeProfile,
    showHidden,
    favoritesOnly,
    search,
    onShowHiddenChange,
    onFavoritesOnlyChange,
    onSearchChange,
    onOpenPath,
    onRemoveCustomRoot,
    onRevealPreset,
    onAddCustomRoot,
    onSelect,
    onFavorite,
    onHidden
  }: {
    presets: Preset[];
    selectedPreset: Preset | null;
    activeProfile: Profile | null;
    showHidden: boolean;
    favoritesOnly: boolean;
    search: string;
    onShowHiddenChange: (value: boolean) => void;
    onFavoritesOnlyChange: (value: boolean) => void;
    onSearchChange: (value: string) => void;
    onOpenPath: (path: string) => void | Promise<void>;
    onRemoveCustomRoot: (root: string) => void | Promise<void>;
    onRevealPreset: (preset: Preset) => void | Promise<void>;
    onAddCustomRoot: () => void | Promise<void>;
    onSelect: (preset: Preset) => void | Promise<void>;
    onFavorite: (preset: Preset) => void | Promise<void>;
    onHidden: (preset: Preset) => void | Promise<void>;
  } = $props();
</script>

<section class="presets-settings-section">
  <div class="presets-layout">
    <div class="presets-meta-column">
      <h3>{$t("settings.presets")}</h3>
      <div class="preset-summary">
        <strong>{presets.length}</strong>
        <span>{$t("presets.found")}</span>
      </div>
      <label class="check-row">
        <input type="checkbox" checked={showHidden} onchange={(event) => onShowHiddenChange(event.currentTarget.checked)} />
        {$t("presets.showHidden")}
      </label>
      <label class="check-row">
        <input type="checkbox" checked={favoritesOnly} onchange={(event) => onFavoritesOnlyChange(event.currentTarget.checked)} />
        {$t("presets.favoritesOnly")}
      </label>
      {#if ($settings.customPresetRoots ?? []).length > 0}
        <div class="custom-roots-card">
          <div class="custom-roots-head">
            <strong>{$t("presets.customRoots")}</strong>
            <span>{$settings.customPresetRoots.length}</span>
          </div>
          <div class="custom-roots-list">
            {#each $settings.customPresetRoots as root}
              <article class="custom-root-row">
                <button class="custom-root-path" type="button" onclick={() => onOpenPath(root)}>
                  <FolderOpen size={15} />
                  <span>{root}</span>
                </button>
                <button class="icon-button" type="button" title={$t("presets.removeCustomRoot")} onclick={() => onRemoveCustomRoot(root)}>
                  <X size={15} />
                </button>
              </article>
            {/each}
          </div>
        </div>
      {/if}
      {#if selectedPreset}
        <button class="secondary-button" type="button" onclick={() => onRevealPreset(selectedPreset)}>
          <FolderOpen size={16} /> {$t("presets.revealSelected")}
        </button>
      {/if}
      <button class="secondary-button" type="button" onclick={onAddCustomRoot}>
        <FolderOpen size={16} /> {$t("zapret.addCustom")}
      </button>
    </div>
    <div class="presets-list-column">
      <label>
        {$t("common.search")}
        <input placeholder={$t("presets.search")} value={search} oninput={(event) => onSearchChange(event.currentTarget.value)} />
      </label>
      <PresetManager
        {presets}
        selectedId={activeProfile?.zapretPresetId}
        {showHidden}
        onSelect={onSelect}
        onFavorite={onFavorite}
        onHidden={onHidden}
        onReveal={onRevealPreset}
      />
    </div>
  </div>
</section>
