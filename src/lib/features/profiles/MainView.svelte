<script lang="ts">
  import { Activity, Cpu, Settings } from "@lucide/svelte";
  import type { AppState, Preset, Profile, ZapretEngine } from "$lib/api/types";
  import PowerButton from "$lib/components/PowerButton.svelte";
  import ServiceRow from "$lib/components/ServiceRow.svelte";
  import { t } from "$lib/stores/i18n";

  let {
    appState,
    activeProfile,
    activeEngine,
    selectedPreset,
    zapretConfigured,
    busy,
    powerLabel,
    powerHint,
    powerDisabled,
    serviceConfigLocked,
    tgProxyLink,
    onTogglePower,
    onUpdateProfile,
    onConfigureZapret,
    onOpenSettings,
    onOpenActivity,
    onOpenTelegramProxy
  }: {
    appState: AppState;
    activeProfile: Profile | null;
    activeEngine: ZapretEngine | null;
    selectedPreset: Preset | null;
    zapretConfigured: boolean;
    busy: boolean;
    powerLabel: string;
    powerHint: string;
    powerDisabled: boolean;
    serviceConfigLocked: boolean;
    tgProxyLink: string | null;
    onTogglePower: () => void | Promise<void>;
    onUpdateProfile: (patch: Partial<Profile>) => void | Promise<void>;
    onConfigureZapret: () => void;
    onOpenSettings: (tab: "general" | "services") => void;
    onOpenActivity: () => void;
    onOpenTelegramProxy: () => void | Promise<void>;
  } = $props();

  function presetLabel(preset: Preset | null) {
    return preset ? preset.relativePath : $t("zapret.noPreset");
  }
</script>

<section class="main-view">
  <div class="power-zone">
    <PowerButton status={appState.status} disabled={powerDisabled} onclick={onTogglePower} />
    <div>
      <strong>{busy ? $t("power.working") : powerLabel}</strong>
      <p>{powerHint}</p>
    </div>
  </div>

  {#if activeProfile}
    <div class="service-list">
      <div class:setup-required={!zapretConfigured} class="service-setup-shell">
        <ServiceRow
          kind="zapret"
          title={activeEngine === "zapret2" ? "zapret 2" : "zapret"}
          subtitle={zapretConfigured
            ? `${activeEngine === "zapret2" ? "Zapret 2" : "Classic"} · ${presetLabel(selectedPreset)}`
            : $t("engine.notConfigured")}
          enabled={activeProfile.zapretEnabled}
          status={appState.zapret}
          actionLabel={$t("service.changeProfile")}
          extraActionLabel={$t("common.logs")}
          disabled={serviceConfigLocked || !zapretConfigured}
          onToggle={(checked) => onUpdateProfile({ zapretEnabled: checked })}
          onAction={() => onOpenSettings("services")}
          onExtraAction={onOpenActivity}
        />
        {#if !zapretConfigured}
          <button class="configure-zapret-overlay" type="button" onclick={onConfigureZapret}>
            <Cpu size={19} />
            <span>
              <strong>{$t("engine.configure")}</strong>
              <small>{$t("engine.configureHint")}</small>
            </span>
          </button>
        {/if}
      </div>
      <ServiceRow
        kind="tg-ws"
        title="tg-ws"
        subtitle={`${activeProfile.tgWsHost}:${activeProfile.tgWsPort}`}
        enabled={activeProfile.tgWsEnabled}
        status={appState.tgWs}
        actionLabel={$t("service.configure")}
        extraActionLabel={$t("service.telegram")}
        extraDisabled={!tgProxyLink}
        disabled={serviceConfigLocked}
        onToggle={(checked) => onUpdateProfile({ tgWsEnabled: checked })}
        onAction={() => onOpenSettings("services")}
        onExtraAction={tgProxyLink ? onOpenTelegramProxy : null}
      />
    </div>
  {/if}

  <footer class="footer-actions">
    <nav class="footer-dock two-actions" aria-label={$t("common.settings")}>
      <button type="button" onclick={() => onOpenSettings("general")}>
        <span class="footer-action-icon"><Settings size={17} /></span>
        <span>{$t("common.settings")}</span>
      </button>
      <button type="button" onclick={onOpenActivity}>
        <span class="footer-action-icon"><Activity size={17} /></span>
        <span>{$t("common.activity")}</span>
      </button>
    </nav>
  </footer>
</section>
