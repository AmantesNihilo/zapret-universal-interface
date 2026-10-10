<script lang="ts">
  import { get } from "svelte/store";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { open } from "@tauri-apps/plugin-dialog";
  import { CircleHelp, Minus, X } from "@lucide/svelte";
  import { commands } from "$lib/api/commands";
  import type {
    ConflictProcess,
    Preset,
    Profile,
    Settings as SettingsModel,
    TestTargetConfig,
    ZapretEngine
  } from "$lib/api/types";
  import StatusBadge from "$lib/components/StatusBadge.svelte";
  import { useAppLifecycle } from "$lib/app/lifecycle";
  import AppNotifications from "$lib/features/notifications/AppNotifications.svelte";
  import AboutDialog from "$lib/features/about/AboutDialog.svelte";
  import SupportReportDialog from "$lib/features/support/SupportReportDialog.svelte";
  import UpdateDialog from "$lib/features/updater/UpdateDialog.svelte";
  import { UpdateController } from "$lib/features/updater/updateController.svelte";
  import TestSelectionDialog from "$lib/features/network-test/TestSelectionDialog.svelte";
  import TestTargetEditorDialog from "$lib/features/network-test/TestTargetEditorDialog.svelte";
  import TestDetailsDialog from "$lib/features/network-test/TestDetailsDialog.svelte";
  import MinimizeChoiceDialog from "$lib/features/window/MinimizeChoiceDialog.svelte";
  import ConflictDialog from "$lib/features/profiles/ConflictDialog.svelte";
  import ActivityView from "$lib/features/activity/ActivityView.svelte";
  import MainView from "$lib/features/profiles/MainView.svelte";
  import GeneralSettings from "$lib/features/settings/GeneralSettings.svelte";
  import ServicesSettings from "$lib/features/settings/ServicesSettings.svelte";
  import SettingsView from "$lib/features/settings/SettingsView.svelte";
  import type { SettingsTab } from "$lib/features/settings/types";
  import NetworkTestSettings from "$lib/features/network-test/NetworkTestSettings.svelte";
  import { testSessionId } from "$lib/features/network-test/model";
  import { NetworkTestController } from "$lib/features/network-test/networkTestController.svelte";
  import PresetsSettings from "$lib/features/presets/PresetsSettings.svelte";
  import DiagnosticsSettings from "$lib/features/diagnostics/DiagnosticsSettings.svelte";
  import {
    extractTgProxyLink,
    extractTgProxySecret,
    previewTgProxySecret,
    redactProxySecret
  } from "$lib/features/telegram/proxy";
  import { appState, loadAppState } from "$lib/stores/appState";
  import { clearLogView, loadLogs, logs } from "$lib/stores/logs";
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
    batchRecommendations,
    currentProgress,
    currentTargets,
    testAbortReason,
    testResults,
    testRunning,
    testStopping
  } from "$lib/stores/tests";

  let view = $state<"main" | "settings" | "activity">("main");
  let error = $state<string | null>(null);
  let busy = $state(false);
  let presetSearch = $state("");
  let showHiddenPresets = $state(false);
  let favoritePresetsOnly = $state(false);
  let settingsTab = $state<SettingsTab>("general");
  let conflictApps = $state<ConflictProcess[]>([]);
  let aboutOpen = $state(false);
  let reportOpen = $state(false);
  let reportText = $state("");
  let reportCopied = $state(false);
  let appVersion = $state("2.2.4-network-test.2");
  let changingZapretEngine = $state(false);
  let engineTransition = $state<ZapretEngine | null>(null);
  let minimizeChoiceOpen = $state(false);
  let minimizeRememberChoice = $state(false);
  let activitySource = $state<"app" | "engine" | "tgWs">("app");
  let activityDirection = $state(1);
  let profileUpdateQueue: Promise<void> = Promise.resolve();
  const updater = new UpdateController((message) => (error = message));
  const networkTest = new NetworkTestController({
    setError: (message) => (error = message),
    runAction,
    updateSettings
  });

  const activeProfile = $derived.by(() => {
    const file = $profilesFile;
    return file.profiles.find((profile) => profile.id === file.activeProfileId) ?? file.profiles[0];
  });

  const selectedPreset = $derived.by(() => {
    if (!activeProfile?.zapretPresetId) return null;
    return $presets.find((preset) => preset.id === activeProfile.zapretPresetId) ?? null;
  });
  const activeEngine = $derived(activeProfile?.zapretEngine ?? null);
  const zapretConfigured = $derived(Boolean(activeEngine && selectedPreset));
  const enginePresets = $derived(
    $presets.filter((preset) => preset.engine === activeEngine)
  );
  const engineResults = $derived.by(() =>
    $testResults
      .filter((result) => result.engine === activeEngine)
      .sort((left, right) => Number(right.finishedAt) - Number(left.finishedAt))
  );
  const latestEngineSessionId = $derived(engineResults[0] ? testSessionId(engineResults[0]) : null);
  const latestEngineSessionResults = $derived.by(() => {
    if (!latestEngineSessionId) return [];
    return engineResults
      .filter((result) => testSessionId(result) === latestEngineSessionId)
      .sort((left, right) => right.score - left.score || right.ok - left.ok);
  });
  const displayedEngineResults = $derived(
    $testRunning && $batchRecommendations.length > 0
      ? $batchRecommendations.filter((result) => result.engine === activeEngine)
      : latestEngineSessionResults
  );
  const latestEngineResult = $derived(displayedEngineResults[0] ?? engineResults[0] ?? null);
  const effectiveTestTargets = $derived(
    ($settingsStore.testTargets ?? []).length > 0
      ? $settingsStore.testTargets
      : networkTest.defaultTargets
  );
  const targetGroups = $derived.by(() => {
    const groups = new Map<string, TestTargetConfig[]>();
    for (const target of effectiveTestTargets) {
      const items = groups.get(target.service) ?? [];
      items.push(target);
      groups.set(target.service, items);
    }
    return Array.from(groups.entries()).map(([service, targets]) => ({ service, targets }));
  });
  const selectableTestPresets = $derived.by(() => {
    const query = networkTest.selectionSearch.trim().toLowerCase();
    if (!query) return enginePresets.filter((preset) => !preset.hidden);
    return enginePresets.filter(
      (preset) =>
        !preset.hidden &&
        (preset.name.toLowerCase().includes(query) ||
          preset.relativePath.toLowerCase().includes(query))
    );
  });
  const tgProxyLink = $derived.by(() => extractTgProxyLink($appState.tgWs.message));
  const tgProxySecret = $derived.by(() => {
    return extractTgProxySecret(tgProxyLink) ?? previewTgProxySecret(activeProfile?.tgWsSecret ?? "", $t("tg.generated"));
  });

  const powerLabel = $derived($appState.status === "on" ? $t("power.turnOff") : $t("power.turnOn"));
  const serviceRuntimeLocked = $derived(
    $testRunning || $testStopping || ["starting", "on", "stopping"].includes($appState.status)
  );
  const serviceConfigLocked = $derived(busy || serviceRuntimeLocked);
  const activityLines = $derived.by(() => {
    if (activitySource === "engine") return $logs.filter((line) => line.source === "zapret");
    if (activitySource === "tgWs") return $logs.filter((line) => line.source === "tgWs");
    return $logs.filter((line) => line.source === "app" || line.source === "tests");
  });
  const activitySourceIndex = $derived(
    activitySource === "app" ? 0 : activitySource === "engine" ? 1 : 2
  );
  const canStartProfile = $derived.by(() => {
    if (!activeProfile) return false;
    const zapretReady =
      activeProfile.zapretEnabled &&
      Boolean(activeProfile.zapretEngine && activeProfile.zapretPresetId);
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
    if (selectedPreset && activeEngine) {
      return `${activeEngine === "zapret2" ? "Zapret 2" : "Classic"} · ${selectedPreset.name}`;
    }
    if (activeProfile?.tgWsEnabled) return $t("power.tgReady");
    return $t("power.selectService");
  });
  const visiblePresets = $derived.by(() => {
    const query = presetSearch.trim().toLowerCase();
    const source = enginePresets.filter((preset) => {
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

  $effect(() => {
    if (!error) return;
    const currentError = error;
    const timeout = window.setTimeout(() => {
      if (error === currentError) error = null;
    }, 9000);
    return () => window.clearTimeout(timeout);
  });

  useAppLifecycle({
    setAppVersion: (version) => (appVersion = version),
    setDefaultTestTargets: (targets) => (networkTest.defaultTargets = targets),
    setError: (message) => (error = message),
    openSettings: () => openSettings("general"),
    checkForUpdatesOnLaunch: () => updater.checkOnLaunch()
  });

  async function runAction(action: () => Promise<unknown>) {
    if (busy) return;
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

  function updateProfile(patch: Partial<Profile>): Promise<void> {
    if (!activeProfile) return Promise.resolve();
    const profileId = activeProfile.id;
    const task = profileUpdateQueue.then(async () => {
      const file = get(profilesFile);
      const current = file.profiles.find((profile) => profile.id === profileId);
      if (!current) return;
      await saveProfile({ ...current, ...patch });
    });
    profileUpdateQueue = task.catch((caught) => {
      error = String(caught);
    });
    return task;
  }

  async function chooseZapretEngine(engine: ZapretEngine) {
    if (!activeProfile || serviceRuntimeLocked || engineTransition) return;
    if (activeProfile.zapretEngine === engine && !changingZapretEngine) return;
    engineTransition = engine;
    try {
      await Promise.all([
        updateProfile({
          zapretEngine: engine,
          zapretPresetId: activeProfile.zapretEngine === engine ? activeProfile.zapretPresetId : null
        }),
        new Promise((resolve) => setTimeout(resolve, 620))
      ]);
      changingZapretEngine = false;
      presetSearch = "";
      networkTest.selectedPresetIds = [];
      batchRecommendations.set([]);
    } finally {
      engineTransition = null;
    }
  }

  function configureZapret() {
    changingZapretEngine = !activeProfile?.zapretEngine;
    openSettings("test");
  }

  function beginEngineChange() {
    if ($testRunning || $testStopping || serviceConfigLocked) return;
    changingZapretEngine = true;
    settingsTab = "test";
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

  async function selectPreset(preset: Preset) {
    await updateProfile({ zapretEngine: preset.engine, zapretPresetId: preset.id });
  }

  async function useRecommendedPreset(presetId: string) {
    const preset = $presets.find((item) => item.id === presetId);
    if (!preset) {
      error = $t("test.recommendedMissing");
      return;
    }
    await updateProfile({ zapretEngine: preset.engine, zapretPresetId: preset.id });
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
      `ZUI ${appVersion} support report`,
      `Created: ${new Date().toISOString()}`,
      "",
      "[App]",
      `status=${appSnapshot.status}`,
      `lastError=${appSnapshot.lastError ?? ""}`,
      `activeProfileId=${appSnapshot.activeProfileId}`,
      `zapret=${appSnapshot.zapret.state} pid=${appSnapshot.zapret.pid ?? ""} message=${appSnapshot.zapret.message ?? ""}`,
      `tgWs=${appSnapshot.tgWs.state} message=${redactProxySecret(appSnapshot.tgWs.message ?? "")}`,
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
      `zapretEngine=${activeProfile?.zapretEngine ?? ""}`,
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
      diagnosticsSnapshot ? `winws2Found=${diagnosticsSnapshot.winws2Found} winws2Running=${diagnosticsSnapshot.winws2Running}` : "",
      diagnosticsSnapshot ? `tgWsPortAvailable=${diagnosticsSnapshot.tgWsPortAvailable}` : "",
      diagnosticsSnapshot?.warnings.length ? `warnings=${diagnosticsSnapshot.warnings.join(" | ")}` : "warnings=",
      "",
      "[Recent logs]",
      ...logSnapshot.slice(-25).map((line) => `[${line.timestamp}] ${line.source}: ${line.message}`)
    ];
    return redactProxySecret(lines.filter((line) => line !== "").join("\n"));
  }

  async function openReport() {
    error = null;
    try {
      await Promise.all([loadAppState(), loadLogs(), loadDiagnostics()]);
      reportText = await commands.collectSupportReport();
      reportOpen = true;
    } catch (caught) {
      error = String(caught);
    }
  }

  async function copySupportReport() {
    if (!reportText) {
      reportText = buildSupportReport();
    }
    await navigator.clipboard?.writeText(reportText);
    reportCopied = true;
    window.setTimeout(() => (reportCopied = false), 1400);
  }

  async function startWindowDrag(event: PointerEvent) {
    if (event.button !== 0) return;
    const target = event.target as HTMLElement | null;
    if (target?.closest("button, input, select, textarea, summary, a")) return;
    await getCurrentWindow().startDragging();
  }

  async function requestMinimize() {
    try {
      if ($settingsStore.minimizeDontAsk) {
        await minimizeUsing($settingsStore.minimizeBehavior);
        return;
      }
      minimizeRememberChoice = false;
      minimizeChoiceOpen = true;
    } catch (caught) {
      error = String(caught);
    }
  }

  function selectActivitySource(source: "app" | "engine" | "tgWs") {
    const nextIndex = source === "app" ? 0 : source === "engine" ? 1 : 2;
    activityDirection = nextIndex >= activitySourceIndex ? 1 : -1;
    activitySource = source;
  }

  async function chooseMinimizeBehavior(behavior: "tray" | "taskbar") {
    try {
      await updateSettings({
        minimizeBehavior: behavior,
        minimizeDontAsk: minimizeRememberChoice
      });
      minimizeChoiceOpen = false;
      await minimizeUsing(behavior);
    } catch (caught) {
      error = String(caught);
    }
  }

  async function minimizeUsing(behavior: "tray" | "taskbar") {
    if (behavior === "tray") {
      await commands.minimizeToTray();
    } else {
      await commands.minimizeWindow();
    }
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
        <button class="icon-button" type="button" onclick={requestMinimize} title={$t("top.hide")}>
          <Minus size={16} />
        </button>
        <button class="icon-button danger-button" type="button" onclick={() => commands.quitApp()} title={$t("top.close")}>
          <X size={16} />
        </button>
      </div>
    </header>

    {#if view === "main"}
      <MainView
        appState={$appState}
        {activeProfile}
        {activeEngine}
        {selectedPreset}
        {zapretConfigured}
        {busy}
        {powerLabel}
        {powerHint}
        {powerDisabled}
        {serviceConfigLocked}
        {tgProxyLink}
        onTogglePower={togglePower}
        onUpdateProfile={updateProfile}
        onConfigureZapret={configureZapret}
        onOpenSettings={openSettings}
        onOpenActivity={() => (view = "activity")}
        onOpenTelegramProxy={openTelegramProxyLink}
      />
    {:else if view === "settings"}
      <SettingsView
        tab={settingsTab}
        onTabChange={(tab) => (settingsTab = tab)}
        onClose={() => (view = "main")}
        onWindowDrag={startWindowDrag}
      >
          {#if settingsTab === "general"}
            <GeneralSettings
              {activeEngine}
              {serviceConfigLocked}
              updateInfo={updater.info}
              updateChecking={updater.checking}
              updateMessage={updater.message}
              onChooseEngine={(engine) => runAction(() => chooseZapretEngine(engine))}
              onUpdateSettings={updateSettings}
              onUpdateWindowLayout={updateWindowLayout}
              onCheckUpdates={() => updater.checkManual()}
            />
          {/if}

          {#if settingsTab === "services"}
            <ServicesSettings
              {activeProfile}
              {activeEngine}
              {enginePresets}
              {showHiddenPresets}
              {serviceConfigLocked}
              diagnostics={$diagnostics}
              {tgProxySecret}
              onUpdateProfile={updateProfile}
              onBeginEngineChange={beginEngineChange}
              onConfigureZapret={configureZapret}
              onOpenResources={() => {
                if ($diagnostics) return commands.openPath($diagnostics.resourcesPath);
              }}
              onRescanPresets={() => runAction(loadPresets)}
              onAddCustomRoot={() => runAction(addCustomPresetRoot)}
            />
          {/if}

          {#if settingsTab === "test"}
            <NetworkTestSettings
              {activeEngine}
              changingEngine={changingZapretEngine}
              {engineTransition}
              {serviceConfigLocked}
              {enginePresets}
              latestResult={latestEngineResult}
              displayedResults={displayedEngineResults}
              effectiveTargets={effectiveTestTargets}
              {targetGroups}
              realityExpanded={networkTest.realityExpanded}
              targetsExpanded={networkTest.targetsExpanded}
              resultsExpanded={networkTest.resultsExpanded}
              onChooseEngine={(engine) => runAction(() => chooseZapretEngine(engine))}
              onBeginEngineChange={beginEngineChange}
              onExport={() => networkTest.exportResults()}
              onImport={() => networkTest.importResults()}
              onDetails={(result) => (networkTest.details = result)}
              onTestAll={() => networkTest.testAll(enginePresets)}
              onChoosePresets={() => networkTest.openSelection(activeProfile ?? null)}
              onStop={() => networkTest.stop()}
              onRealityExpandedChange={(value) => (networkTest.realityExpanded = value)}
              onResultsExpandedChange={(value) => (networkTest.resultsExpanded = value)}
              onTargetsExpandedChange={(value) => (networkTest.targetsExpanded = value)}
              onUseResult={(result) => runAction(() => useRecommendedPreset(result.presetId))}
              onEditTargets={() => networkTest.openTargetEditor(effectiveTestTargets)}
            />
          {/if}

          {#if settingsTab === "presets"}
            <PresetsSettings
              presets={visiblePresets}
              {selectedPreset}
              {activeProfile}
              showHidden={showHiddenPresets}
              favoritesOnly={favoritePresetsOnly}
              search={presetSearch}
              onShowHiddenChange={(value) => (showHiddenPresets = value)}
              onFavoritesOnlyChange={(value) => (favoritePresetsOnly = value)}
              onSearchChange={(value) => (presetSearch = value)}
              onOpenPath={(path) => commands.openPath(path)}
              onRemoveCustomRoot={(root) => runAction(() => removeCustomPresetRoot(root))}
              onRevealPreset={(preset) => commands.revealPath(preset.path)}
              onAddCustomRoot={() => runAction(addCustomPresetRoot)}
              onSelect={(preset) => runAction(() => selectPreset(preset))}
              onFavorite={(preset) => runAction(() => setPresetFavorite(preset.id, !preset.favorite))}
              onHidden={(preset) => runAction(() => setPresetHidden(preset.id, !preset.hidden))}
            />
          {/if}

          {#if settingsTab === "diagnostics"}
            <DiagnosticsSettings
              diagnostics={$diagnostics}
              onRefresh={() => runAction(loadDiagnostics)}
              onOpenReport={openReport}
              onOpenPath={(path) => commands.openPath(path)}
            />
          {/if}
      </SettingsView>
    {:else}
      <ActivityView
        source={activitySource}
        sourceIndex={activitySourceIndex}
        direction={activityDirection}
        lines={activityLines}
        allLogs={$logs}
        appState={$appState}
        {activeEngine}
        onSelectSource={selectActivitySource}
        onClear={clearLogView}
        onClose={() => (view = "main")}
        onWindowDrag={startWindowDrag}
      />
    {/if}

    <AppNotifications
      {error}
      abortReason={$testAbortReason}
      stopping={$testStopping}
      onCloseError={() => (error = null)}
      onCloseAbort={() => ($testAbortReason = null)}
    />

    <AboutDialog open={aboutOpen} {appVersion} onClose={() => (aboutOpen = false)} />

    <SupportReportDialog
      open={reportOpen}
      {appVersion}
      {reportText}
      copied={reportCopied}
      getReport={buildSupportReport}
      onCopy={copySupportReport}
      onClose={() => (reportOpen = false)}
    />

    <UpdateDialog
      open={updater.open}
      info={updater.info}
      installing={updater.installing}
      onClose={() => updater.postpone()}
      onOpenRelease={() => updater.openRelease()}
      onOpenPortable={() => updater.openPortableDownload()}
      onInstall={() => updater.install()}
    />

    <TestSelectionDialog
      open={networkTest.selectionOpen}
      search={networkTest.selectionSearch}
      presets={selectableTestPresets}
      selectedIds={networkTest.selectedPresetIds}
      onSearchChange={(value) => (networkTest.selectionSearch = value)}
      onToggle={(presetId) => networkTest.togglePreset(presetId)}
      onSelectAll={() => (networkTest.selectedPresetIds = selectableTestPresets.map((preset) => preset.id))}
      onStart={() => networkTest.testSelected()}
      onClose={() => (networkTest.selectionOpen = false)}
    />

    <TestTargetEditorDialog
      open={networkTest.targetEditorOpen}
      targets={networkTest.targetEditorTargets}
      customService={networkTest.customTargetService}
      customName={networkTest.customTargetName}
      customValue={networkTest.customTargetValue}
      onCustomServiceChange={(value) => (networkTest.customTargetService = value)}
      onCustomNameChange={(value) => (networkTest.customTargetName = value)}
      onCustomValueChange={(value) => (networkTest.customTargetValue = value)}
      onToggle={(index) => networkTest.toggleEditorTarget(index)}
      onRemove={(index) => networkTest.removeEditorTarget(index)}
      onAdd={() => networkTest.addEditorTarget()}
      onReset={() => networkTest.resetTargets()}
      onSave={() => networkTest.saveTargetEditor()}
      onClose={() => (networkTest.targetEditorOpen = false)}
    />

    <MinimizeChoiceDialog
      open={minimizeChoiceOpen}
      remember={minimizeRememberChoice}
      onRememberChange={(value) => (minimizeRememberChoice = value)}
      onChoose={chooseMinimizeBehavior}
      onClose={() => (minimizeChoiceOpen = false)}
    />

    <ConflictDialog
      conflicts={conflictApps}
      onCancel={() => (conflictApps = [])}
      onIgnore={() => runAction(startProfileIgnoringConflicts)}
      onKillAndStart={() => runAction(killConflictsAndStart)}
    />

    <TestDetailsDialog result={networkTest.details} onClose={() => (networkTest.details = null)} />
  </section>
</main>
