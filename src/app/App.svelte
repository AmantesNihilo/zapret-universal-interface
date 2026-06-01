<script lang="ts">
  import { onMount } from "svelte";
  import { get } from "svelte/store";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { open } from "@tauri-apps/plugin-dialog";
  import {
    Activity,
    ChevronDown,
    CircleHelp,
    Download,
    ExternalLink,
    FolderOpen,
    Minus,
    RotateCw,
    Settings,
    X
  } from "@lucide/svelte";
  import { commands } from "$lib/api/commands";
  import { onOperationFailed, onTrayAction } from "$lib/api/events";
  import type {
    ConflictProcess,
    Preset,
    Profile,
    Settings as SettingsModel,
    TestResult,
    UpdateCheck
  } from "$lib/api/types";
  import PowerButton from "$lib/components/PowerButton.svelte";
  import ServiceRow from "$lib/components/ServiceRow.svelte";
  import StatusBadge from "$lib/components/StatusBadge.svelte";
  import LogViewer from "$lib/components/LogViewer.svelte";
  import TestSummary from "$lib/components/TestSummary.svelte";
  import TestRecommendations from "$lib/components/TestRecommendations.svelte";
  import DiagnosticsPanel from "$lib/components/DiagnosticsPanel.svelte";
  import PresetManager from "$lib/components/PresetManager.svelte";
  import PresetDropdown from "$lib/components/PresetDropdown.svelte";
  import OptionDropdown, { type DropdownOption } from "$lib/components/OptionDropdown.svelte";
  import { appState, bindAppStateEvents, loadAppState } from "$lib/stores/appState";
  import { bindLogEvents, clearLogView, loadLogs, logs } from "$lib/stores/logs";
  import { loadPresets, presets, setPresetFavorite, setPresetHidden } from "$lib/stores/presets";
  import {
    loadProfiles,
    profilesFile,
    saveProfile
  } from "$lib/stores/profiles";
  import { loadSettings, saveSettings, settings as settingsStore } from "$lib/stores/settings";
  import { t } from "$lib/stores/i18n";
  import { diagnostics, loadDiagnostics } from "$lib/stores/diagnostics";
  import {
    bindTestEvents,
    batchRecommendations,
    cancelPresetTest,
    currentPresetName,
    currentTargets,
    loadTestResults,
    runAllPresetTest,
    runBestPresetTest,
    runQuickPresetTest,
    testResults,
    testRunning,
    testStopping
  } from "$lib/stores/tests";

  type SettingsTab = "general" | "services" | "test" | "presets" | "diagnostics";

  let view = $state<"main" | "settings" | "activity">("main");
  let error = $state<string | null>(null);
  let busy = $state(false);
  let presetSearch = $state("");
  let showHiddenPresets = $state(false);
  let favoritePresetsOnly = $state(false);
  let settingsTab = $state<SettingsTab>("general");
  let testDetails = $state<TestResult | null>(null);
  let conflictApps = $state<ConflictProcess[]>([]);
  let aboutOpen = $state(false);
  let reportOpen = $state(false);
  let reportCopied = $state(false);
  let updateInfo = $state<UpdateCheck | null>(null);
  let updateOpen = $state(false);
  let updateChecking = $state(false);
  let updateInstalling = $state(false);
  let updateMessage = $state<string | null>(null);

  const themeOptions = $derived.by((): DropdownOption[] => [
    { value: "dark", label: $t("theme.dark"), hint: $t("theme.darkHint") },
    { value: "oled", label: $t("theme.oled"), hint: $t("theme.oledHint") },
    { value: "light", label: $t("theme.light"), hint: $t("theme.lightHint") },
    { value: "system", label: $t("theme.system"), hint: $t("theme.systemHint") }
  ]);

  const languageOptions = $derived.by((): DropdownOption[] => [
    { value: "ru", label: $t("language.ru"), hint: $t("language.ruHint") },
    { value: "en", label: $t("language.en"), hint: $t("language.enHint") }
  ]);

  const orientationOptions = $derived.by((): DropdownOption[] => [
    { value: "portrait", label: $t("orientation.portrait"), hint: $t("orientation.portraitHint") },
    { value: "landscape", label: $t("orientation.landscape"), hint: $t("orientation.landscapeHint") }
  ]);

  const accentOptions = $derived.by((): DropdownOption[] => [
    { value: "cyan", label: $t("accent.cyan"), color: "#25c7d9" },
    { value: "teal", label: $t("accent.teal"), color: "#19d3b5" },
    { value: "green", label: $t("accent.green"), color: "#48c774" },
    { value: "lime", label: $t("accent.lime"), color: "#a3e635" },
    { value: "blue", label: $t("accent.blue"), color: "#4c8df6" },
    { value: "violet", label: $t("accent.violet"), color: "#8b5cf6" },
    { value: "pink", label: $t("accent.pink"), color: "#ec5ead" },
    { value: "red", label: $t("accent.red"), color: "#ef5b5b" },
    { value: "orange", label: $t("accent.orange"), color: "#f97316" },
    { value: "amber", label: $t("accent.amber"), color: "#e5ae38" }
  ]);

  const activeProfile = $derived.by(() => {
    const file = $profilesFile;
    return file.profiles.find((profile) => profile.id === file.activeProfileId) ?? file.profiles[0];
  });

  const selectedPreset = $derived.by(() => {
    if (!activeProfile?.zapretPresetId) return null;
    return $presets.find((preset) => preset.id === activeProfile.zapretPresetId) ?? null;
  });
  const tgProxyLink = $derived.by(() => extractTgProxyLink($appState.tgWs.message));
  const tgProxySecret = $derived.by(() => {
    return extractTgProxySecret(tgProxyLink) ?? previewTgProxySecret(activeProfile?.tgWsSecret ?? "");
  });

  const powerLabel = $derived($appState.status === "on" ? $t("power.turnOff") : $t("power.turnOn"));
  const serviceConfigLocked = $derived(
    busy || $testRunning || $testStopping || ["starting", "on", "stopping"].includes($appState.status)
  );
  const canStartProfile = $derived.by(() => {
    if (!activeProfile) return false;
    const zapretReady = activeProfile.zapretEnabled && Boolean(activeProfile.zapretPresetId);
    const tgWsReady = activeProfile.tgWsEnabled;
    return zapretReady || tgWsReady;
  });
  const powerDisabled = $derived.by(
    () =>
      busy ||
      $testRunning ||
      $testStopping ||
      ["starting", "stopping"].includes($appState.status) ||
      ($appState.status !== "on" && !canStartProfile)
  );
  const powerHint = $derived.by(() => {
    if (selectedPreset) return selectedPreset.name;
    if (activeProfile?.tgWsEnabled) return $t("power.tgReady");
    return $t("power.selectService");
  });
  const visiblePresets = $derived.by(() => {
    const query = presetSearch.trim().toLowerCase();
    const source = $presets.filter((preset) => {
      if (!showHiddenPresets && preset.hidden) return false;
      if (favoritePresetsOnly && !preset.favorite) return false;
      return true;
    });
    if (!query) return source.slice(0, 200);
    return source
      .filter((preset) => preset.relativePath.toLowerCase().includes(query))
      .slice(0, 200);
  });

  $effect(() => {
    if (typeof document === "undefined") return;
    document.documentElement.dataset.theme = $settingsStore.theme;
    document.documentElement.dataset.accent = $settingsStore.accent;
    document.documentElement.dataset.layout = $settingsStore.layoutOrientation || "portrait";
    document.documentElement.lang = $settingsStore.language || "ru";
  });

  onMount(() => {
    let unlistenState: (() => void) | undefined;
    let unlistenLogs: (() => void) | undefined;
    let unlistenTray: (() => void) | undefined;
    let unlistenTests: (() => void) | undefined;
    let unlistenFailures: (() => void) | undefined;
    const blockContextMenu = (event: MouseEvent) => event.preventDefault();

    document.addEventListener("contextmenu", blockContextMenu);

    async function init() {
      try {
        await Promise.all([
          loadAppState(),
          loadSettings(),
          loadProfiles(),
          loadPresets(),
          loadLogs(),
          loadTestResults(),
          loadDiagnostics()
        ]);
        unlistenState = await bindAppStateEvents();
        unlistenLogs = await bindLogEvents();
        unlistenTests = await bindTestEvents();
        unlistenTray = await onTrayAction((action) => {
          if (action === "settings") {
            openSettings("general");
          }
        });
        unlistenFailures = await onOperationFailed((message) => {
          error = message;
        });
        void checkForUpdatesOnLaunch();
      } catch (caught) {
        error = String(caught);
      }
    }

    init();

    return () => {
      if (unlistenState) {
        unlistenState();
      }
      if (unlistenLogs) {
        unlistenLogs();
      }
      if (unlistenTray) {
        unlistenTray();
      }
      if (unlistenTests) {
        unlistenTests();
      }
      if (unlistenFailures) {
        unlistenFailures();
      }
      document.removeEventListener("contextmenu", blockContextMenu);
    };
  });

  async function runAction(action: () => Promise<unknown>) {
    busy = true;
    error = null;
    try {
      await action();
      await Promise.all([loadAppState(), loadProfiles(), loadLogs(), loadDiagnostics()]);
    } catch (caught) {
      error = String(caught);
    } finally {
      busy = false;
    }
  }

  async function checkForUpdatesOnLaunch() {
    try {
      const info = await commands.checkForUpdate();
      updateInfo = info;
      updateOpen = info.updateAvailable;
    } catch {
      // Update checks should never block app startup.
    }
  }

  async function checkForUpdatesManual() {
    updateChecking = true;
    error = null;
    try {
      const info = await commands.checkForUpdate();
      updateInfo = info;
      updateOpen = info.updateAvailable;
      updateMessage = null;
      if (!info.updateAvailable) {
        updateMessage = $t("update.none", { version: info.currentVersion });
      }
    } catch (caught) {
      updateMessage = null;
      error = `${$t("update.failed")}: ${String(caught)}`;
    } finally {
      updateChecking = false;
    }
  }

  async function installUpdate() {
    updateInstalling = true;
    error = null;
    try {
      await commands.installUpdate();
    } catch (caught) {
      updateInstalling = false;
      error = `${$t("update.installFailed")}: ${String(caught)}`;
    }
  }

  async function openUpdateRelease() {
    if (!updateInfo?.releaseUrl) return;
    await commands.openUrl(updateInfo.releaseUrl);
  }

  function formatBytes(bytes?: number | null) {
    if (!bytes || bytes <= 0) return "";
    const units = ["B", "KB", "MB", "GB"];
    let size = bytes;
    let unit = 0;
    while (size >= 1024 && unit < units.length - 1) {
      size /= 1024;
      unit += 1;
    }
    return `${size.toFixed(unit === 0 ? 0 : 1)} ${units[unit]}`;
  }

  async function togglePower() {
    await runAction(async () => {
      if ($appState.status === "on") {
        await commands.stopProfile();
      } else {
        await startProfileWithConflictPrompt();
      }
    });
  }

  async function startProfileWithConflictPrompt() {
    if (activeProfile?.zapretEnabled) {
      const conflicts = await commands.getConflictingApps();
      if (conflicts.length > 0) {
        conflictApps = conflicts;
        return;
      }
    }
    await commands.startProfile();
  }

  async function startProfileIgnoringConflicts() {
    conflictApps = [];
    await commands.startProfile();
  }

  async function killConflictsAndStart() {
    const remaining = await commands.killConflictingApps(conflictApps.map((process) => process.pid));
    if (remaining.length > 0) {
      conflictApps = remaining;
      throw new Error($t("conflicts.stillRunning"));
    }
    conflictApps = [];
    await commands.startProfile();
  }

  function openSettings(tab: SettingsTab = "general") {
    settingsTab = tab;
    view = "settings";
  }

  async function updateProfile(patch: Partial<Profile>) {
    if (!activeProfile) return;
    await saveProfile({ ...activeProfile, ...patch });
  }

  async function updateSettings(patch: Partial<SettingsModel>) {
    await saveSettings({ ...get(settingsStore), ...patch });
  }

  async function updateWindowLayout(layoutOrientation: SettingsModel["layoutOrientation"]) {
    await updateSettings({ layoutOrientation });
    await commands.setWindowLayout(layoutOrientation);
  }

  async function addCustomPresetRoot() {
    const selected = await open({
      directory: true,
      multiple: false,
      title: $t("zapret.addCustomFolder")
    });
    if (!selected || Array.isArray(selected)) return;

    const current = get(settingsStore).customPresetRoots ?? [];
    if (!current.some((path) => path.toLowerCase() === selected.toLowerCase())) {
      await updateSettings({ customPresetRoots: [...current, selected] });
    }
    await Promise.all([loadSettings(), loadPresets(), loadDiagnostics()]);
  }

  async function removeCustomPresetRoot(root: string) {
    const current = get(settingsStore).customPresetRoots ?? [];
    await updateSettings({
      customPresetRoots: current.filter((path) => path.toLowerCase() !== root.toLowerCase())
    });
    await Promise.all([loadSettings(), loadPresets(), loadDiagnostics()]);
  }

  function presetLabel(preset: Preset | null) {
    return preset ? preset.relativePath : $t("zapret.noPreset");
  }

  async function runSelectedPresetTest() {
    if (!activeProfile?.zapretPresetId) {
      error = $t("test.noSelected");
      return;
    }
    await runAction(async () => runQuickPresetTest(activeProfile.zapretPresetId!));
  }

  async function findBestPreset() {
    const candidates = visiblePresets
      .filter((preset) => !preset.hidden && isExecutablePreset(preset))
      .slice(0, 40)
      .map((preset) => preset.id);
    if (candidates.length === 0) {
      error = $t("test.noExecutableFilter");
      return;
    }
    await runAction(async () => runBestPresetTest(candidates, 40));
  }

  async function testAllPresets() {
    const candidates = $presets
      .filter((preset) => !preset.hidden && isExecutablePreset(preset))
      .map((preset) => preset.id);
    if (candidates.length === 0) {
      error = $t("test.noExecutable");
      return;
    }
    await runAction(async () => runAllPresetTest(candidates));
  }

  async function stopPresetTest() {
    if (!$testRunning || $testStopping) return;
    error = null;
    try {
      await cancelPresetTest();
    } catch (caught) {
      error = String(caught);
    }
  }

  async function selectPreset(preset: Preset) {
    await updateProfile({ zapretPresetId: preset.id });
  }

  async function useRecommendedPreset(presetId: string) {
    const preset = $presets.find((item) => item.id === presetId);
    if (!preset) {
      error = $t("test.recommendedMissing");
      return;
    }
    await selectPreset(preset);
  }

  function domainListValue(profile: Profile) {
    return profile.tgWsCfDomains.join("\n");
  }

  function parseDomainList(value: string) {
    return value
      .split(/[\n,]/)
      .map((domain) => domain.trim())
      .filter(Boolean);
  }

  function isExecutablePreset(preset: Preset) {
    return preset.kind === "bat" || preset.kind === "cmd";
  }

  function targetKind(label: string) {
    if (label.includes("TLS1.3")) return "TLS 1.3";
    if (label.includes("TLS1.2")) return "TLS 1.2";
    if (label.includes("HTTP1.1")) return "HTTP 1.1";
    if (label.includes("Ping")) return "Ping";
    return $t("common.check");
  }

  function targetMeta(target: { label: string; status?: number | null; latencyMs?: number | null }) {
    const parts = [targetKind(target.label)];
    if (target.status) parts.push(`HTTP ${target.status}`);
    if (target.latencyMs) parts.push(`${target.latencyMs} ms`);
    return parts.join(" / ");
  }

  function extractTgProxyLink(message?: string | null) {
    return message?.match(/tg:\/\/proxy\?\S+/)?.[0] ?? null;
  }

  function extractTgProxySecret(link?: string | null) {
    if (!link) return null;
    try {
      return new URL(link).searchParams.get("secret");
    } catch {
      return link.match(/[?&]secret=([^&\s]+)/)?.[1] ?? null;
    }
  }

  function previewTgProxySecret(raw: string) {
    const value = raw.trim();
    if (!value) return $t("tg.generated");
    if (isTelegramLinkSecret(value)) return value;
    if (isHex(value) && (value.length === 32 || value.length === 34)) return `dd${value}`;
    return `dd${plainSecretToHex(value)}`;
  }

  function isTelegramLinkSecret(value: string) {
    return isHex(value) && ((value.length === 34 && value.startsWith("dd")) || value.startsWith("ee"));
  }

  function isHex(value: string) {
    return /^[0-9a-fA-F]+$/.test(value);
  }

  function plainSecretToHex(value: string) {
    const bytes = new Uint8Array(16);
    bytes.set(new TextEncoder().encode(value).slice(0, 16));
    return Array.from(bytes, (byte) => byte.toString(16).padStart(2, "0")).join("");
  }

  async function openTelegramProxyLink() {
    if (!tgProxyLink) return;
    error = null;
    try {
      await commands.openUrl(tgProxyLink);
    } catch (caught) {
      error = String(caught);
    }
  }

  function buildSupportReport() {
    const diagnosticsSnapshot = get(diagnostics);
    const appSnapshot = get(appState);
    const settingsSnapshot = get(settingsStore);
    const logSnapshot = get(logs);
    const lines = [
      "ZUI 2.0.0 support report",
      `Created: ${new Date().toISOString()}`,
      "",
      "[App]",
      `status=${appSnapshot.status}`,
      `lastError=${appSnapshot.lastError ?? ""}`,
      `activeProfileId=${appSnapshot.activeProfileId}`,
      `zapret=${appSnapshot.zapret.state} pid=${appSnapshot.zapret.pid ?? ""} message=${appSnapshot.zapret.message ?? ""}`,
      `tgWs=${appSnapshot.tgWs.state} message=${appSnapshot.tgWs.message ?? ""}`,
      "",
      "[Settings]",
      `theme=${settingsSnapshot.theme}`,
      `accent=${settingsSnapshot.accent}`,
      `language=${settingsSnapshot.language}`,
      `layout=${settingsSnapshot.layoutOrientation}`,
      "",
      "[Profile]",
      `name=${activeProfile?.name ?? ""}`,
      `zapretEnabled=${activeProfile?.zapretEnabled ?? false}`,
      `zapretPreset=${selectedPreset?.relativePath ?? activeProfile?.zapretPresetId ?? ""}`,
      `tgWsEnabled=${activeProfile?.tgWsEnabled ?? false}`,
      `tgWs=${activeProfile ? `${activeProfile.tgWsHost}:${activeProfile.tgWsPort}` : ""}`,
      "",
      "[Diagnostics]",
      diagnosticsSnapshot ? `admin=${diagnosticsSnapshot.isAdmin}` : "not loaded",
      diagnosticsSnapshot ? `resources=${diagnosticsSnapshot.resourcesPath}` : "",
      diagnosticsSnapshot ? `data=${diagnosticsSnapshot.dataPath}` : "",
      diagnosticsSnapshot ? `logs=${diagnosticsSnapshot.logsPath}` : "",
      diagnosticsSnapshot ? `presets=${diagnosticsSnapshot.presetCount}` : "",
      diagnosticsSnapshot ? `winwsFound=${diagnosticsSnapshot.winwsFound} winwsRunning=${diagnosticsSnapshot.winwsRunning}` : "",
      diagnosticsSnapshot ? `tgWsPortAvailable=${diagnosticsSnapshot.tgWsPortAvailable}` : "",
      diagnosticsSnapshot?.warnings.length ? `warnings=${diagnosticsSnapshot.warnings.join(" | ")}` : "warnings=",
      "",
      "[Recent logs]",
      ...logSnapshot.slice(-25).map((line) => `[${line.timestamp}] ${line.source}: ${line.message}`)
    ];
    return lines.filter((line) => line !== "").join("\n");
  }

  async function copySupportReport() {
    await navigator.clipboard?.writeText(buildSupportReport());
    reportCopied = true;
    window.setTimeout(() => (reportCopied = false), 1400);
  }

  async function startWindowDrag(event: PointerEvent) {
    if (event.button !== 0) return;
    const target = event.target as HTMLElement | null;
    if (target?.closest("button, input, select, textarea, summary, a")) return;
    await getCurrentWindow().startDragging();
  }
</script>

<main class="shell">
  <section class="utility-window">
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <header class="topbar" data-tauri-drag-region onpointerdown={startWindowDrag}>
      <StatusBadge state={$appState.status} testing={$testRunning} />
      <h1 class="top-title" data-tauri-drag-region>ZUI</h1>
      <div class="window-actions">
        <button class="icon-button" type="button" onclick={() => (aboutOpen = true)} title={$t("top.about")}>
          <CircleHelp size={16} />
        </button>
        <button class="icon-button" type="button" onclick={() => commands.minimizeToTray()} title={$t("top.hide")}>
          <Minus size={16} />
        </button>
        <button class="icon-button danger-button" type="button" onclick={() => commands.quitApp()} title={$t("top.close")}>
          <X size={16} />
        </button>
      </div>
    </header>

    {#if view === "main"}
      <section class="main-view">
        <div class="power-zone">
          <PowerButton status={$appState.status} disabled={powerDisabled} onclick={togglePower} />
          <div>
            <strong>{busy ? $t("power.working") : powerLabel}</strong>
            <p>{powerHint}</p>
          </div>
        </div>

        {#if activeProfile}
          <div class="service-list">
            <ServiceRow
              kind="zapret"
              title="zapret"
              subtitle={presetLabel(selectedPreset)}
              enabled={activeProfile.zapretEnabled}
              status={$appState.zapret}
              actionLabel={$t("service.changeProfile")}
              extraActionLabel={$t("common.logs")}
              disabled={serviceConfigLocked}
              onToggle={(checked) => updateProfile({ zapretEnabled: checked })}
              onAction={() => openSettings("services")}
              onExtraAction={() => (view = "activity")}
            />
            <ServiceRow
              kind="tg-ws"
              title="tg-ws"
              subtitle={`${activeProfile.tgWsHost}:${activeProfile.tgWsPort}`}
              enabled={activeProfile.tgWsEnabled}
              status={$appState.tgWs}
              actionLabel={$t("service.configure")}
              extraActionLabel={$t("service.telegram")}
              extraDisabled={!tgProxyLink}
              disabled={serviceConfigLocked}
              onToggle={(checked) => updateProfile({ tgWsEnabled: checked })}
              onAction={() => openSettings("services")}
              onExtraAction={tgProxyLink ? openTelegramProxyLink : null}
            />
          </div>
        {/if}

        <footer class="footer-actions">
          <button type="button" onclick={() => openSettings("general")}>
            <Settings size={16} /> {$t("common.settings")}
          </button>
          <button type="button" onclick={() => (view = "activity")}>
            <Activity size={16} /> {$t("common.activity")}
          </button>
          <button type="button" onclick={() => commands.minimizeToTray()}>
            <Minus size={16} /> {$t("common.tray")}
          </button>
        </footer>
      </section>
    {:else if view === "settings"}
      <section class="settings-view">
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <div class="view-heading" data-tauri-drag-region onpointerdown={startWindowDrag}>
          <h2>{$t("common.settings")}</h2>
          <button class="icon-button" type="button" onclick={() => (view = "main")} title={$t("common.close")}>
            <X size={18} />
          </button>
        </div>

        <nav class="settings-tabs" aria-label={$t("common.settings")}>
          <button
            type="button"
            class:active={settingsTab === "general"}
            onclick={() => (settingsTab = "general")}
          >
            {$t("settings.general")}
          </button>
          <button
            type="button"
            class:active={settingsTab === "services"}
            onclick={() => (settingsTab = "services")}
          >
            {$t("settings.services")}
          </button>
          <button
            type="button"
            class:active={settingsTab === "test"}
            onclick={() => (settingsTab = "test")}
          >
            {$t("settings.test")}
          </button>
          <button
            type="button"
            class:active={settingsTab === "presets"}
            onclick={() => (settingsTab = "presets")}
          >
            {$t("settings.presets")}
          </button>
          <button
            type="button"
            class:active={settingsTab === "diagnostics"}
            onclick={() => (settingsTab = "diagnostics")}
          >
            {$t("settings.checks")}
          </button>
        </nav>

        <div class="settings-grid">
          {#if settingsTab === "general"}
            <section>
              <h3>{$t("settings.general")}</h3>
              <label>
                {$t("settings.theme")}
                <OptionDropdown
                  options={themeOptions}
                  value={$settingsStore.theme}
                  onChange={(theme) => updateSettings({ theme: theme as SettingsModel["theme"] })}
                />
              </label>
              <label>
                {$t("settings.language")}
                <OptionDropdown
                  options={languageOptions}
                  value={$settingsStore.language}
                  onChange={(language) => updateSettings({ language })}
                />
              </label>
              <label>
                {$t("settings.orientation")}
                <OptionDropdown
                  options={orientationOptions}
                  value={$settingsStore.layoutOrientation}
                  onChange={(layoutOrientation) =>
                    updateWindowLayout(layoutOrientation as SettingsModel["layoutOrientation"])}
                />
              </label>
              <label>
                {$t("settings.accent")}
                <OptionDropdown
                  options={accentOptions}
                  value={$settingsStore.accent}
                  onChange={(accent) => updateSettings({ accent })}
                />
              </label>
              <label class="check-row">
                <input
                  type="checkbox"
                  checked={$settingsStore.autoStartActiveProfileOnLaunch}
                  onchange={(event) => updateSettings({ autoStartActiveProfileOnLaunch: event.currentTarget.checked })}
                />
                {$t("settings.autoStart")}
              </label>
              <div class="update-card">
                <div>
                  <strong>{$t("update.title")}</strong>
                  <span>
                    {#if updateMessage}
                      {updateMessage}
                    {:else if updateInfo?.updateAvailable}
                      {$t("update.availableShort", { version: updateInfo.latestVersion ?? "" })}
                    {:else if updateInfo}
                      {$t("update.current", { version: updateInfo.currentVersion })}
                    {:else}
                      {$t("update.startupHint")}
                    {/if}
                  </span>
                </div>
                <button
                  class="secondary-button"
                  type="button"
                  disabled={updateChecking}
                  onclick={checkForUpdatesManual}
                >
                  <RotateCw size={16} /> {updateChecking ? $t("update.checking") : $t("update.check")}
                </button>
              </div>
            </section>
          {/if}

          {#if settingsTab === "services"}
            {#if activeProfile}
              <section>
                <h3>{$t("settings.services")}</h3>
                <label class="check-row">
                  <input
                    type="checkbox"
                    checked={activeProfile.zapretEnabled}
                    disabled={serviceConfigLocked}
                    onchange={(event) => updateProfile({ zapretEnabled: event.currentTarget.checked })}
                  />
                  {$t("zapret.enablePower")}
                </label>
                <div class="field-group">
                  <span>{$t("zapret.preset")}</span>
                  <PresetDropdown
                    presets={$presets.filter((preset) => showHiddenPresets || !preset.hidden)}
                    selectedId={activeProfile.zapretPresetId}
                    disabled={serviceConfigLocked}
                    emptyText={$t("zapret.noPresetsFolder")}
                    onSelect={(presetId) => updateProfile({ zapretPresetId: presetId })}
                  />
                </div>
                {#if $presets.length === 0}
                  <div class="empty-callout">
                    <span>{$t("zapret.noPresets")}</span>
                    <button
                      class="secondary-button"
                      type="button"
                      disabled={!$diagnostics}
                      onclick={() => $diagnostics && commands.openPath($diagnostics.resourcesPath)}
                    >
                      <FolderOpen size={16} /> {$t("zapret.openFolder")}
                    </button>
                  </div>
                {/if}
                <button class="secondary-button" type="button" onclick={() => runAction(loadPresets)}>
                  <RotateCw size={16} /> {$t("zapret.rescan")}
                </button>
                <button class="secondary-button" type="button" onclick={() => runAction(addCustomPresetRoot)}>
                  <FolderOpen size={16} /> {$t("zapret.addCustom")}
                </button>
              </section>

              <section>
                <h3>tg-ws</h3>
                <label class="check-row">
                  <input
                    type="checkbox"
                    checked={activeProfile.tgWsEnabled}
                    disabled={serviceConfigLocked}
                    onchange={(event) => updateProfile({ tgWsEnabled: event.currentTarget.checked })}
                  />
                  {$t("tg.enablePower")}
                </label>
                <div class="form-row">
                  <label>
                    {$t("tg.host")}
                    <input
                      value={activeProfile.tgWsHost}
                      disabled={serviceConfigLocked}
                      oninput={(event) => updateProfile({ tgWsHost: event.currentTarget.value })}
                    />
                  </label>
                  <label>
                    {$t("tg.port")}
                    <input
                      type="number"
                      min="1"
                      max="65535"
                      value={activeProfile.tgWsPort}
                      disabled={serviceConfigLocked}
                      oninput={(event) => updateProfile({ tgWsPort: Number(event.currentTarget.value) })}
                    />
                  </label>
                </div>
                <label>
                  {$t("tg.secretSeed")}
                  <input
                    value={activeProfile.tgWsSecret}
                    disabled={serviceConfigLocked}
                    oninput={(event) => updateProfile({ tgWsSecret: event.currentTarget.value })}
                  />
                </label>
                <div class="secret-preview">
                  <span>{$t("tg.secret")}</span>
                  <code>{tgProxySecret}</code>
                </div>
                <details class="advanced-panel">
                  <summary>
                    <span>
                      <ChevronDown class="summary-chevron" size={16} />
                      {$t("tg.advancedRouting")}
                    </span>
                  </summary>
                  <label class="check-row">
                    <input
                      type="checkbox"
                      checked={activeProfile.tgWsDefaultDomains}
                      disabled={serviceConfigLocked}
                      onchange={(event) => updateProfile({ tgWsDefaultDomains: event.currentTarget.checked })}
                    />
                    {$t("tg.defaultDomains")}
                  </label>
                  <label>
                    {$t("tg.cfDomains")}
                    <textarea
                      rows="3"
                      value={domainListValue(activeProfile)}
                      disabled={serviceConfigLocked}
                      oninput={(event) => updateProfile({ tgWsCfDomains: parseDomainList(event.currentTarget.value) })}
                    ></textarea>
                  </label>
                  <label>
                    {$t("tg.cfWorker")}
                    <input
                      value={activeProfile.tgWsCfWorkerDomain ?? ""}
                      disabled={serviceConfigLocked}
                      oninput={(event) => updateProfile({ tgWsCfWorkerDomain: event.currentTarget.value || null })}
                    />
                  </label>
                  <label class="check-row">
                    <input
                      type="checkbox"
                      checked={activeProfile.tgWsCfPriority}
                      disabled={serviceConfigLocked}
                      onchange={(event) => updateProfile({ tgWsCfPriority: event.currentTarget.checked })}
                    />
                    {$t("tg.cfPriority")}
                  </label>
                  <label class="check-row">
                    <input
                      type="checkbox"
                      checked={activeProfile.tgWsCfBalance}
                      disabled={serviceConfigLocked}
                      onchange={(event) => updateProfile({ tgWsCfBalance: event.currentTarget.checked })}
                    />
                    {$t("tg.cfBalance")}
                  </label>
                </details>
              </section>
            {:else}
              <section>
                <h3>{$t("settings.services")}</h3>
                <p class="muted-text">{$t("settings.noProfile")}</p>
              </section>
            {/if}
          {/if}

          {#if settingsTab === "test"}
            <section>
              <h3>{$t("settings.test")}</h3>
              <TestSummary
                result={$testResults[$testResults.length - 1] ?? null}
                running={$testRunning}
                stopping={$testStopping}
                presetName={$currentPresetName}
                targets={$currentTargets}
                onDetails={(result) => (testDetails = result)}
              />
              <div class="button-row">
                <button
                  class="secondary-button"
                  type="button"
                  disabled={$testRunning || $testStopping || !activeProfile?.zapretPresetId}
                  onclick={runSelectedPresetTest}
                >
                  <RotateCw size={16} /> {$t("test.quick")}
                </button>
                <button
                  class="secondary-button"
                  type="button"
                  disabled={$testRunning || $testStopping}
                  onclick={testAllPresets}
                >
                  <RotateCw size={16} /> {$t("test.all")}
                </button>
                <button
                  class="secondary-button"
                  type="button"
                  disabled={$testRunning || $testStopping}
                  onclick={findBestPreset}
                >
                  <RotateCw size={16} /> {$t("test.findBest")}
                </button>
                <button
                  class="secondary-button"
                  type="button"
                  disabled={!$testRunning || $testStopping}
                  onclick={stopPresetTest}
                >
                  <X size={16} /> {$testStopping ? $t("test.stopping") : $t("test.stop")}
                </button>
              </div>
              <TestRecommendations
                results={$batchRecommendations}
                onUse={(result) => runAction(() => useRecommendedPreset(result.presetId))}
                onDetails={(result) => (testDetails = result)}
              />
            </section>
          {/if}

          {#if settingsTab === "presets"}
            <section class="presets-settings-section">
              <div class="presets-layout">
                <div class="presets-meta-column">
                  <h3>{$t("settings.presets")}</h3>
                  <div class="preset-summary">
                    <strong>{visiblePresets.length}</strong>
                    <span>{$t("presets.found")}</span>
                  </div>
                  <label class="check-row">
                    <input
                      type="checkbox"
                      checked={showHiddenPresets}
                      onchange={(event) => (showHiddenPresets = event.currentTarget.checked)}
                    />
                    {$t("presets.showHidden")}
                  </label>
                  <label class="check-row">
                    <input
                      type="checkbox"
                      checked={favoritePresetsOnly}
                      onchange={(event) => (favoritePresetsOnly = event.currentTarget.checked)}
                    />
                    {$t("presets.favoritesOnly")}
                  </label>
                  {#if ($settingsStore.customPresetRoots ?? []).length > 0}
                    <div class="custom-roots-card">
                      <div class="custom-roots-head">
                        <strong>{$t("presets.customRoots")}</strong>
                        <span>{$settingsStore.customPresetRoots.length}</span>
                      </div>
                      <div class="custom-roots-list">
                        {#each $settingsStore.customPresetRoots as root}
                          <article class="custom-root-row">
                            <button class="custom-root-path" type="button" onclick={() => commands.openPath(root)}>
                              <FolderOpen size={15} />
                              <span>{root}</span>
                            </button>
                            <button
                              class="icon-button"
                              type="button"
                              title={$t("presets.removeCustomRoot")}
                              onclick={() => runAction(() => removeCustomPresetRoot(root))}
                            >
                              <X size={15} />
                            </button>
                          </article>
                        {/each}
                      </div>
                    </div>
                  {/if}
                  {#if selectedPreset}
                    <button class="secondary-button" type="button" onclick={() => commands.revealPath(selectedPreset.path)}>
                      <FolderOpen size={16} /> {$t("presets.revealSelected")}
                    </button>
                  {/if}
                  <button class="secondary-button" type="button" onclick={() => runAction(addCustomPresetRoot)}>
                    <FolderOpen size={16} /> {$t("zapret.addCustom")}
                  </button>
                </div>
                <div class="presets-list-column">
                  <label>
                    {$t("common.search")}
                    <input
                      placeholder={$t("presets.search")}
                      value={presetSearch}
                      oninput={(event) => (presetSearch = event.currentTarget.value)}
                    />
                  </label>
                  <PresetManager
                    presets={visiblePresets}
                    selectedId={activeProfile?.zapretPresetId}
                    showHidden={showHiddenPresets}
                    onSelect={(preset) => runAction(() => selectPreset(preset))}
                    onFavorite={(preset) => runAction(() => setPresetFavorite(preset.id, !preset.favorite))}
                    onHidden={(preset) => runAction(() => setPresetHidden(preset.id, !preset.hidden))}
                    onReveal={(preset) => commands.revealPath(preset.path)}
                  />
                </div>
              </div>
            </section>
          {/if}

          {#if settingsTab === "diagnostics"}
            <section>
              <h3>{$t("diagnostics.title")}</h3>
              <DiagnosticsPanel diagnostics={$diagnostics} onOpenPath={(path) => commands.openPath(path)} />
              <div class="button-row">
                <button class="secondary-button" type="button" onclick={() => runAction(loadDiagnostics)}>
                  <RotateCw size={16} /> {$t("common.refresh")}
                </button>
                <button class="secondary-button" type="button" onclick={() => (reportOpen = true)}>
                  {$t("report.title")}
                </button>
              </div>
            </section>
          {/if}
        </div>
      </section>
    {:else}
      <section class="activity-view">
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <div class="view-heading" data-tauri-drag-region onpointerdown={startWindowDrag}>
          <h2>{$t("activity.title")}</h2>
          <button class="icon-button" type="button" onclick={() => (view = "main")} title={$t("common.close")}>
            <X size={18} />
          </button>
        </div>
        <LogViewer lines={$logs} />
        <button class="secondary-button" type="button" onclick={clearLogView}>{$t("activity.clear")}</button>
      </section>
    {/if}

    {#if error}
      <div class="error-strip">{error}</div>
    {/if}

    {#if $testStopping}
      <div class="wait-overlay" role="presentation" aria-live="polite">
        <div class="wait-panel">
          <div class="wait-orbit" aria-hidden="true">
            <span></span>
            <span></span>
          </div>
          <strong>{$t("test.stoppingTitle")}</strong>
          <p>{$t("test.stoppingText")}</p>
        </div>
      </div>
    {/if}

    {#if aboutOpen}
      <div class="about-overlay" role="presentation" onclick={(event) => event.currentTarget === event.target && (aboutOpen = false)}>
        <div class="about-panel" role="dialog" aria-modal="true" aria-label={$t("about.title")}>
          <header>
            <div class="about-mark" aria-hidden="true">
              <img src="/zui-icon.png" alt="" />
            </div>
            <div>
              <h3>ZUI</h3>
              <p>{$t("about.version")}</p>
            </div>
            <button class="icon-button" type="button" onclick={() => (aboutOpen = false)} title={$t("common.close")}>
              <X size={18} />
            </button>
          </header>

          <section>
            <h4>{$t("about.developer")}</h4>
            <p>{$t("about.developerText")}</p>
          </section>

          <section>
            <h4>{$t("about.usedProjects")}</h4>
            <div class="credit-list">
              <span>{$t("about.zapret")}</span>
              <span>{$t("about.tgProxy")}</span>
              <span>{$t("about.howdyho")}</span>
              <span>{$t("about.flowseal")}</span>
            </div>
          </section>

          <section>
            <h4>{$t("about.purpose")}</h4>
            <p>{$t("about.purposeText")}</p>
          </section>
        </div>
      </div>
    {/if}

    {#if reportOpen}
      <div class="about-overlay" role="presentation" onclick={(event) => event.currentTarget === event.target && (reportOpen = false)}>
        <div class="about-panel report-panel" role="dialog" aria-modal="true" aria-label={$t("report.title")}>
          <header>
            <div>
              <h3>{$t("report.title")}</h3>
              <p>ZUI 2.0.0</p>
            </div>
            <button class="icon-button" type="button" onclick={() => (reportOpen = false)} title={$t("common.close")}>
              <X size={18} />
            </button>
          </header>
          <textarea readonly rows="18" value={buildSupportReport()}></textarea>
          <button class="secondary-button" type="button" onclick={copySupportReport}>
            {reportCopied ? $t("report.copied") : $t("report.copy")}
          </button>
        </div>
      </div>
    {/if}

    {#if updateOpen && updateInfo}
      <div class="about-overlay" role="presentation" onclick={(event) => event.currentTarget === event.target && (updateOpen = false)}>
        <div class="about-panel update-panel" role="dialog" aria-modal="true" aria-label={$t("update.available")}>
          <header>
            <div class="about-mark" aria-hidden="true">
              <img src="/zui-icon.png" alt="" />
            </div>
            <div>
              <h3>{$t("update.available")}</h3>
              <p>
                {$t("update.versionLine", {
                  current: updateInfo.currentVersion,
                  latest: updateInfo.latestVersion ?? ""
                })}
              </p>
            </div>
            <button
              class="icon-button"
              type="button"
              disabled={updateInstalling}
              onclick={() => (updateOpen = false)}
              title={$t("update.later")}
            >
              <X size={18} />
            </button>
          </header>

          <section class="update-summary">
            <div>
              <strong>{updateInfo.releaseName ?? `ZUI ${updateInfo.latestVersion ?? ""}`}</strong>
              <span>
                {#if updateInfo.distribution === "portable"}
                  {$t("update.portableText")}
                {:else if updateInfo.distribution === "development"}
                  {$t("update.developmentText")}
                {:else if updateInfo.canInstall}
                  {$t("update.installedText")}
                {:else}
                  {$t("update.noInstaller")}
                {/if}
              </span>
            </div>
            {#if updateInfo.installerAsset || updateInfo.portableAsset}
              <div class="update-assets">
                {#if updateInfo.installerAsset}
                  <span>{updateInfo.installerAsset.name} {formatBytes(updateInfo.installerAsset.size)}</span>
                {/if}
                {#if updateInfo.portableAsset}
                  <span>{updateInfo.portableAsset.name} {formatBytes(updateInfo.portableAsset.size)}</span>
                {/if}
              </div>
            {/if}
          </section>

          {#if updateInfo.releaseNotes}
            <pre class="update-notes">{updateInfo.releaseNotes}</pre>
          {/if}

          <div class="update-actions">
            <button class="secondary-button" type="button" disabled={updateInstalling} onclick={() => (updateOpen = false)}>
              {$t("update.later")}
            </button>
            <button class="secondary-button" type="button" disabled={updateInstalling || !updateInfo.releaseUrl} onclick={openUpdateRelease}>
              <ExternalLink size={16} /> {$t("update.openRelease")}
            </button>
            {#if updateInfo.canInstall}
              <button class="primary-button" type="button" disabled={updateInstalling} onclick={installUpdate}>
                <Download size={16} /> {updateInstalling ? $t("update.installing") : $t("update.install")}
              </button>
            {/if}
          </div>
        </div>
      </div>
    {/if}

    {#if conflictApps.length > 0}
      <div class="conflict-overlay" role="presentation">
        <div class="conflict-panel" role="dialog" aria-modal="true" aria-label={$t("conflicts.title")}>
          <div>
            <h3>{$t("conflicts.title")}</h3>
            <p>{$t("conflicts.text")}</p>
          </div>
          <div class="conflict-list">
            {#each conflictApps as process}
              <div class="conflict-row">
                <strong>{process.image}</strong>
                <span>PID {process.pid}{process.title ? ` - ${process.title}` : ""}</span>
              </div>
            {/each}
          </div>
          <div class="conflict-actions">
            <button class="secondary-button" type="button" onclick={() => (conflictApps = [])}>
              {$t("common.cancel")}
            </button>
            <button
              class="secondary-button"
              type="button"
              onclick={() => runAction(startProfileIgnoringConflicts)}
            >
              {$t("common.ignore")}
            </button>
            <button class="primary-button" type="button" onclick={() => runAction(killConflictsAndStart)}>
              {$t("conflicts.killStart")}
            </button>
          </div>
        </div>
      </div>
    {/if}

    {#if testDetails}
      <div class="test-details-overlay" role="presentation" onclick={(event) => event.currentTarget === event.target && (testDetails = null)}>
        <div class="test-details-panel" role="dialog" aria-modal="true" aria-label={$t("test.details")}>
          <div class="test-details-head">
            <div>
              <h3>{testDetails.presetName}</h3>
              <p>{$t("test.passed", { ok: testDetails.ok, total: testDetails.total })}</p>
            </div>
            <button class="icon-button" type="button" onclick={() => (testDetails = null)} title={$t("common.close")}>
              <X size={18} />
            </button>
          </div>

          <div class="test-details-score">
            <div class="score-ring">{testDetails.score}</div>
            <div>
              <strong>{$t("test.overall")}</strong>
              <span>{testDetails.startedAt} - {testDetails.finishedAt}</span>
            </div>
          </div>

          <div class="test-details-list">
            {#each testDetails.services as service}
              <article class="test-service-card">
                <header>
                  <strong>{service.name}</strong>
                  <span class:passed={service.status === "passed"} class:partial={service.status === "partial"} class:failed={service.status === "failed"}>
                    {service.ok}/{service.total}
                  </span>
                </header>
                <div>
                  {#each service.targets as target}
                    <div class:failed={!target.ok} class="target-row">
                      <span>{target.ok ? $t("common.ok") : $t("common.fail")}</span>
                      <div class="target-body">
                        <div class="target-title-line">
                          <strong>{target.label || target.url}</strong>
                          <small class="protocol-chip">{targetKind(target.label)}</small>
                        </div>
                        <small>{target.url}</small>
                        <small>{targetMeta(target)}</small>
                      </div>
                      {#if target.error}
                        <em>{target.error}</em>
                      {/if}
                    </div>
                  {/each}
                </div>
              </article>
            {/each}
          </div>
        </div>
      </div>
    {/if}
  </section>
</main>
