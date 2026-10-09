<script lang="ts">
  import { Cpu, FolderOpen, RotateCw } from "@lucide/svelte";
  import type { Diagnostics, Preset, Profile, ZapretEngine } from "$lib/api/types";
  import PresetDropdown from "$lib/components/PresetDropdown.svelte";
  import TgWsSettings from "$lib/components/TgWsSettings.svelte";
  import { t } from "$lib/stores/i18n";

  let {
    activeProfile,
    activeEngine,
    enginePresets,
    showHiddenPresets,
    serviceConfigLocked,
    diagnostics,
    tgProxySecret,
    onUpdateProfile,
    onBeginEngineChange,
    onConfigureZapret,
    onOpenResources,
    onRescanPresets,
    onAddCustomRoot
  }: {
    activeProfile: Profile | null;
    activeEngine: ZapretEngine | null;
    enginePresets: Preset[];
    showHiddenPresets: boolean;
    serviceConfigLocked: boolean;
    diagnostics: Diagnostics | null;
    tgProxySecret: string;
    onUpdateProfile: (patch: Partial<Profile>) => void | Promise<void>;
    onBeginEngineChange: () => void;
    onConfigureZapret: () => void;
    onOpenResources: () => void | Promise<void>;
    onRescanPresets: () => void | Promise<void>;
    onAddCustomRoot: () => void | Promise<void>;
  } = $props();
</script>

{#if activeProfile}
  <section>
    <h3>{$t("settings.services")}</h3>
    <label class="check-row">
      <input
        type="checkbox"
        checked={activeProfile.zapretEnabled}
        disabled={serviceConfigLocked}
        onchange={(event) => onUpdateProfile({ zapretEnabled: event.currentTarget.checked })}
      />
      {$t("zapret.enablePower")}
    </label>
    <div class="engine-inline-card">
      <span>
        <small>{$t("engine.title")}</small>
        <strong>{activeEngine === "zapret2" ? "Zapret 2" : activeEngine === "classic" ? "Zapret Classic" : $t("engine.notConfigured")}</strong>
      </span>
      <button class="secondary-button compact-button" type="button" disabled={serviceConfigLocked} onclick={onBeginEngineChange}>
        {$t("engine.change")}
      </button>
    </div>
    {#if activeEngine}
      <div class="field-group">
        <span>{$t("zapret.preset")}</span>
        <PresetDropdown
          presets={enginePresets.filter((preset) => showHiddenPresets || !preset.hidden)}
          selectedId={activeProfile.zapretPresetId}
          disabled={serviceConfigLocked}
          emptyText={$t("zapret.noPresetsFolder")}
          onSelect={(presetId) => onUpdateProfile({ zapretPresetId: presetId })}
        />
      </div>
    {:else}
      <button class="primary-button" type="button" onclick={onConfigureZapret}>
        <Cpu size={16} /> {$t("engine.configure")}
      </button>
    {/if}
    {#if activeEngine && enginePresets.length === 0}
      <div class="empty-callout">
        <span>{$t("zapret.noPresets")}</span>
        <button class="secondary-button" type="button" disabled={!diagnostics} onclick={onOpenResources}>
          <FolderOpen size={16} /> {$t("zapret.openFolder")}
        </button>
      </div>
    {/if}
    <button class="secondary-button" type="button" onclick={onRescanPresets}>
      <RotateCw size={16} /> {$t("zapret.rescan")}
    </button>
    <button class="secondary-button" type="button" onclick={onAddCustomRoot}>
      <FolderOpen size={16} /> {$t("zapret.addCustom")}
    </button>
  </section>

  <TgWsSettings
    profile={activeProfile}
    locked={serviceConfigLocked}
    secretPreview={tgProxySecret}
    onUpdate={onUpdateProfile}
  />
{:else}
  <section>
    <h3>{$t("settings.services")}</h3>
    <p class="muted-text">{$t("settings.noProfile")}</p>
  </section>
{/if}
