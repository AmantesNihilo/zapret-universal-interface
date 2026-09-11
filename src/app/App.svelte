<script lang="ts">
  import { onMount } from "svelte";
  import { get } from "svelte/store";
  import { fly, slide } from "svelte/transition";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { open, save } from "@tauri-apps/plugin-dialog";
  import {
    Activity,
    AlertTriangle,
    Check,
    ChevronDown,
    CircleHelp,
    Cpu,
    Download,
    ExternalLink,
    FolderOpen,
    Minus,
    Pencil,
    Plus,
    RotateCw,
    Settings,
    Shield,
    Upload,
    X
  } from "@lucide/svelte";
  import { commands } from "$lib/api/commands";
  import { onOperationFailed, onTrayAction } from "$lib/api/events";
  import type {
    ConflictProcess,
    Preset,
    Profile,
    Settings as SettingsModel,
    TestTargetConfig,
    TestResult,
    UpdateCheck,
    ZapretEngine
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
  import TgWsSettings from "$lib/components/TgWsSettings.svelte";
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
    currentProgress,
    currentTargets,
    importTestResults,
    loadTestResults,
    runAllPresetTest,
    runSelectedPresetTests,
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
  let reportText = $state("");
  let reportCopied = $state(false);
  let updateInfo = $state<UpdateCheck | null>(null);
  let updateOpen = $state(false);
  let updateChecking = $state(false);
  let updateInstalling = $state(false);
  let updateMessage = $state<string | null>(null);
  let updatePostponedVersion = $state<string | null>(null);
  let appVersion = $state("2.2.1");
  let changingZapretEngine = $state(false);
  let engineTransition = $state<ZapretEngine | null>(null);
  let testSelectionOpen = $state(false);
  let testSelectionSearch = $state("");
  let selectedTestPresetIds = $state<string[]>([]);
  let targetEditorOpen = $state(false);
  let targetEditorTargets = $state<TestTargetConfig[]>([]);
  let customTargetService = $state("Custom");
  let customTargetName = $state("");
  let customTargetValue = $state("");
  let realityExpanded = $state(false);
  let targetsExpanded = $state(false);
  let testResultsExpanded = $state(true);
  let minimizeChoiceOpen = $state(false);
  let minimizeRememberChoice = $state(false);
  let activitySource = $state<"app" | "engine" | "tgWs">("app");
  let activityDirection = $state(1);
  let profileUpdateQueue: Promise<void> = Promise.resolve();

  const settingsTabOrder: SettingsTab[] = ["general", "services", "test", "presets", "diagnostics"];
  const settingsTabIndex = $derived(Math.max(0, settingsTabOrder.indexOf(settingsTab)));

  const defaultTestTargets: TestTargetConfig[] = [
    { service: "Discord", name: "Discord", value: "https://discord.com", enabled: true },
    { service: "Discord", name: "Discord Gateway", value: "https://gateway.discord.gg", enabled: true },
    { service: "Discord", name: "Discord CDN", value: "https://cdn.discordapp.com", enabled: true },
    { service: "Discord", name: "Discord Updates", value: "https://updates.discord.com", enabled: true },
    { service: "YouTube", name: "YouTube", value: "https://www.youtube.com", enabled: true },
    { service: "YouTube", name: "YouTube Short", value: "https://youtu.be", enabled: true },
    { service: "YouTube", name: "YouTube Images", value: "https://i.ytimg.com", enabled: true },
    { service: "YouTube", name: "GoogleVideo", value: "https://redirector.googlevideo.com", enabled: true },
    { service: "Google", name: "Google", value: "https://www.google.com", enabled: true },
    { service: "Google", name: "Google Static", value: "https://www.gstatic.com", enabled: true },
    { service: "Google", name: "Google DNS", value: "PING:8.8.8.8", enabled: true },
    { service: "Google", name: "Google DNS 2", value: "PING:8.8.4.4", enabled: true },
    { service: "Cloudflare", name: "Cloudflare", value: "https://www.cloudflare.com", enabled: true },
    { service: "Cloudflare", name: "Cloudflare CDN", value: "https://cdnjs.cloudflare.com", enabled: true },
    { service: "Cloudflare", name: "Cloudflare DNS", value: "PING:1.1.1.1", enabled: true },
    { service: "Cloudflare", name: "Cloudflare DNS 2", value: "PING:1.0.0.1", enabled: true },
    { service: "DNS", name: "Quad9", value: "PING:9.9.9.9", enabled: true }
  ];

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
  const minimizeOptions = $derived.by((): DropdownOption[] => [
    { value: "taskbar", label: $t("minimize.taskbar"), hint: $t("minimize.taskbarHint") },
    { value: "tray", label: $t("minimize.tray"), hint: $t("minimize.trayHint") }
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
  const activeEngine = $derived(activeProfile?.zapretEngine ?? null);
  const engineOptions = $derived.by((): DropdownOption[] => [
    ...(!activeEngine
      ? [{ value: "", label: $t("engine.notConfigured"), hint: $t("engine.configureHint") }]
      : []),
    { value: "classic", label: "Zapret Classic", hint: $t("engine.classicShort") },
    { value: "zapret2", label: "Zapret 2", hint: $t("engine.zapret2Short") }
  ]);
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
      : defaultTestTargets
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
    const query = testSelectionSearch.trim().toLowerCase();
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
    return extractTgProxySecret(tgProxyLink) ?? previewTgProxySecret(activeProfile?.tgWsSecret ?? "");
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

  onMount(() => {
    let unlistenState: (() => void) | undefined;
    let unlistenLogs: (() => void) | undefined;
    let unlistenTray: (() => void) | undefined;
    let unlistenTests: (() => void) | undefined;
    let unlistenFailures: (() => void) | undefined;
    const blockContextMenu = (event: MouseEvent) => event.preventDefault();

    document.addEventListener("contextmenu", blockContextMenu);

    async function init() {
      const startupTasks = [
        commands.getAppVersion().then((version) => (appVersion = version)),
        loadAppState(),
        loadSettings(),
        loadProfiles(),
        loadPresets(),
        loadLogs(),
        loadTestResults(),
        loadDiagnostics()
      ];
      const results = await Promise.allSettled(startupTasks);
      const failures = results
        .filter((result): result is PromiseRejectedResult => result.status === "rejected")
        .map((result) => String(result.reason));
      if (failures.length > 0) {
        error = [...new Set(failures)].join("; ");
      }

      try {
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
        if (get(settingsStore).checkUpdatesOnLaunch) {
          void checkForUpdatesOnLaunch();
        }
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

  async function checkForUpdatesOnLaunch() {
    try {
      const info = await commands.checkForUpdate();
      updateInfo = info;
      const version = info.latestVersion ?? "";
      updateOpen = info.updateAvailable && version !== updatePostponedVersion;
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

  async function openPortableDownload() {
    const url = updateInfo?.portableAsset?.downloadUrl ?? updateInfo?.releaseUrl;
    if (!url) return;
    await commands.openUrl(url);
  }

  function postponeUpdate() {
    updatePostponedVersion = updateInfo?.latestVersion ?? null;
    updateOpen = false;
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
      selectedTestPresetIds = [];
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

  function presetLabel(preset: Preset | null) {
    return preset ? preset.relativePath : $t("zapret.noPreset");
  }

  async function testAllPresets() {
    const candidates = enginePresets
      .filter((preset) => !preset.hidden && isExecutablePreset(preset))
      .map((preset) => preset.id);
    if (candidates.length === 0) {
      error = $t("test.noExecutable");
      return;
    }
    await runAction(async () => runAllPresetTest(candidates));
  }

  function openPresetTestSelection() {
    const initial = activeProfile?.zapretPresetId
      ? [activeProfile.zapretPresetId]
      : [];
    selectedTestPresetIds = initial;
    testSelectionSearch = "";
    testSelectionOpen = true;
  }

  function toggleTestPreset(presetId: string) {
    selectedTestPresetIds = selectedTestPresetIds.includes(presetId)
      ? selectedTestPresetIds.filter((id) => id !== presetId)
      : [...selectedTestPresetIds, presetId];
  }

  async function testSelectedPresets() {
    if (selectedTestPresetIds.length === 0) {
      error = $t("test.selectAtLeastOne");
      return;
    }
    testSelectionOpen = false;
    await runAction(async () => runSelectedPresetTests(selectedTestPresetIds));
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

  async function exportTestResultsFile() {
    if ($testResults.length === 0) {
      error = $t("test.noResults");
      return;
    }
    const path = await save({
      title: $t("test.export"),
      defaultPath: "zui-test-results.json",
      filters: [{ name: "JSON", extensions: ["json"] }]
    });
    if (!path) return;
    await runAction(async () => commands.exportTestResults(path));
  }

  async function importTestResultsFile() {
    const path = await open({
      directory: false,
      multiple: false,
      title: $t("test.import"),
      filters: [{ name: "JSON", extensions: ["json"] }]
    });
    if (!path || Array.isArray(path)) return;
    await runAction(async () => {
      const results = await importTestResults(path);
      if (results.length > 0) {
        testResultsExpanded = true;
      }
    });
  }

  async function resetTestTargets() {
    await updateSettings({ testTargets: [] });
    targetEditorTargets = defaultTestTargets.map((target) => ({ ...target }));
  }

  function openTargetEditor() {
    targetEditorTargets = effectiveTestTargets.map((target) => ({ ...target }));
    customTargetService = "Custom";
    customTargetName = "";
    customTargetValue = "";
    targetEditorOpen = true;
  }

  function toggleEditorTarget(index: number) {
    targetEditorTargets = targetEditorTargets.map((target, itemIndex) =>
      itemIndex === index ? { ...target, enabled: !target.enabled } : target
    );
  }

  function removeEditorTarget(index: number) {
    targetEditorTargets = targetEditorTargets.filter((_, itemIndex) => itemIndex !== index);
  }

  function addEditorTarget() {
    const name = customTargetName.trim();
    const value = customTargetValue.trim();
    if (!name || (!value.startsWith("http://") && !value.startsWith("https://") && !value.toUpperCase().startsWith("PING:"))) {
      error = $t("test.invalidTarget");
      return;
    }
    targetEditorTargets = [
      ...targetEditorTargets,
      {
        service: customTargetService.trim() || "Custom",
        name,
        value,
        enabled: true
      }
    ];
    customTargetName = "";
    customTargetValue = "";
  }

  async function saveTargetEditor() {
    await updateSettings({ testTargets: targetEditorTargets });
    targetEditorOpen = false;
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

  function isExecutablePreset(preset: Preset) {
    return preset.engine === "zapret2"
      ? preset.kind === "config"
      : preset.kind === "bat" || preset.kind === "cmd";
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

  function recommendationLabel(value: TestResult["recommendation"]) {
    if (value === "recommended") return $t("test.recommended");
    if (value === "partial") return $t("test.partial");
    return $t("test.notRecommended");
  }

  function modeLabel(value: TestResult["mode"]) {
    return value === "all" ? $t("test.modeAll") : $t("test.modeSelected");
  }

  function testSessionId(result: TestResult) {
    return result.id.startsWith("batch-") ? result.id.replace(/-\d+$/, "") : result.id;
  }

  function formatTestTime(value: string) {
    const timestamp = Number(value);
    if (!Number.isFinite(timestamp) || timestamp <= 0) return value;
    return new Date(timestamp * 1000).toLocaleString($settingsStore.language === "ru" ? "ru-RU" : "en-US");
  }

  function extractTgProxyLink(message?: string | null) {
    return message?.match(/tg:\/\/proxy\?\S+/)?.[0] ?? null;
  }

  function redactProxySecret(value: string) {
    return value.replace(/([?&]secret=)[^&\s]+/gi, "$1<redacted>");
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
            <div class:setup-required={!zapretConfigured} class="service-setup-shell">
              <ServiceRow
                kind="zapret"
                title={activeEngine === "zapret2" ? "zapret 2" : "zapret"}
                subtitle={zapretConfigured
                  ? `${activeEngine === "zapret2" ? "Zapret 2" : "Classic"} · ${presetLabel(selectedPreset)}`
                  : $t("engine.notConfigured")}
                enabled={activeProfile.zapretEnabled}
                status={$appState.zapret}
                actionLabel={$t("service.changeProfile")}
                extraActionLabel={$t("common.logs")}
                disabled={serviceConfigLocked || !zapretConfigured}
                onToggle={(checked) => updateProfile({ zapretEnabled: checked })}
                onAction={() => openSettings("services")}
                onExtraAction={() => (view = "activity")}
              />
              {#if !zapretConfigured}
                <button class="configure-zapret-overlay" type="button" onclick={configureZapret}>
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
          <nav class="footer-dock two-actions" aria-label={$t("common.settings")}>
            <button type="button" onclick={() => openSettings("general")}>
              <span class="footer-action-icon"><Settings size={17} /></span>
              <span>{$t("common.settings")}</span>
            </button>
            <button type="button" onclick={() => (view = "activity")}>
              <span class="footer-action-icon"><Activity size={17} /></span>
              <span>{$t("common.activity")}</span>
            </button>
          </nav>
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

        <nav class="settings-tabs" style={`--active-index: ${settingsTabIndex}`} aria-label={$t("common.settings")}>
          <span class="settings-tab-indicator" aria-hidden="true"></span>
          <button
            type="button"
            data-tab="general"
            class:active={settingsTab === "general"}
            onclick={() => (settingsTab = "general")}
          >
            {$t("settings.general")}
          </button>
          <button
            type="button"
            data-tab="services"
            class:active={settingsTab === "services"}
            onclick={() => (settingsTab = "services")}
          >
            {$t("settings.services")}
          </button>
          <button
            type="button"
            data-tab="test"
            class:active={settingsTab === "test"}
            onclick={() => (settingsTab = "test")}
          >
            {$t("settings.test")}
          </button>
          <button
            type="button"
            data-tab="presets"
            class:active={settingsTab === "presets"}
            onclick={() => (settingsTab = "presets")}
          >
            {$t("settings.presets")}
          </button>
          <button
            type="button"
            data-tab="diagnostics"
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
                {$t("engine.title")}
                <OptionDropdown
                  options={engineOptions}
                  value={activeEngine ?? ""}
                  disabled={serviceConfigLocked}
                  onChange={(engine) => {
                    if (engine) void runAction(() => chooseZapretEngine(engine as ZapretEngine));
                  }}
                />
              </label>
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
              <div class="settings-subsection minimize-settings">
                <div>
                  <strong>{$t("minimize.settingsTitle")}</strong>
                  <span>{$t("minimize.settingsHint")}</span>
                </div>
                <label>
                  {$t("minimize.defaultAction")}
                  <OptionDropdown
                    options={minimizeOptions}
                    value={$settingsStore.minimizeBehavior}
                    onChange={(minimizeBehavior) =>
                      updateSettings({
                        minimizeBehavior: minimizeBehavior as SettingsModel["minimizeBehavior"]
                      })}
                  />
                </label>
                <label class="check-row">
                  <input
                    type="checkbox"
                    checked={$settingsStore.minimizeDontAsk}
                    onchange={(event) => updateSettings({ minimizeDontAsk: event.currentTarget.checked })}
                  />
                  {$t("minimize.dontAsk")}
                </label>
              </div>
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
                  checked={$settingsStore.startWithWindows}
                  onchange={(event) => updateSettings({ startWithWindows: event.currentTarget.checked })}
                />
                {$t("settings.startWithWindows")}
              </label>
              <label class="check-row startup-tray-option" class:disabled={!$settingsStore.startWithWindows}>
                <input
                  type="checkbox"
                  checked={$settingsStore.startWithWindowsInTray}
                  disabled={!$settingsStore.startWithWindows}
                  onchange={(event) => updateSettings({ startWithWindowsInTray: event.currentTarget.checked })}
                />
                <span>
                  {$t("settings.startWithWindowsInTray")}
                  <small>{$t("settings.startWithWindowsInTrayHint")}</small>
                </span>
              </label>
              <label class="check-row">
                <input
                  type="checkbox"
                  checked={$settingsStore.autoStartActiveProfileOnLaunch}
                  onchange={(event) => updateSettings({ autoStartActiveProfileOnLaunch: event.currentTarget.checked })}
                />
                {$t("settings.autoStart")}
              </label>
              <label class="check-row">
                <input
                  type="checkbox"
                  checked={$settingsStore.checkUpdatesOnLaunch}
                  onchange={(event) => updateSettings({ checkUpdatesOnLaunch: event.currentTarget.checked })}
                />
                {$t("settings.checkUpdatesOnLaunch")}
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
                <div class="engine-inline-card">
                  <span>
                    <small>{$t("engine.title")}</small>
                    <strong>{activeEngine === "zapret2" ? "Zapret 2" : activeEngine === "classic" ? "Zapret Classic" : $t("engine.notConfigured")}</strong>
                  </span>
                  <button class="secondary-button compact-button" type="button" disabled={serviceConfigLocked} onclick={beginEngineChange}>
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
                      onSelect={(presetId) => updateProfile({ zapretPresetId: presetId })}
                    />
                  </div>
                {:else}
                  <button class="primary-button" type="button" onclick={configureZapret}>
                    <Cpu size={16} /> {$t("engine.configure")}
                  </button>
                {/if}
                {#if activeEngine && enginePresets.length === 0}
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

              <TgWsSettings
                profile={activeProfile}
                locked={serviceConfigLocked}
                secretPreview={tgProxySecret}
                onUpdate={updateProfile}
              />
            {:else}
              <section>
                <h3>{$t("settings.services")}</h3>
                <p class="muted-text">{$t("settings.noProfile")}</p>
              </section>
            {/if}
          {/if}

          {#if settingsTab === "test"}
            <section class="test-section">
              {#if !activeEngine || changingZapretEngine}
                <div class:transitioning={engineTransition !== null} class="engine-onboarding">
                  <div class="engine-onboarding-head">
                    <div>
                      <h3>{$t("engine.chooseTitle")}</h3>
                      <p>{$t("engine.chooseText")}</p>
                    </div>
                  </div>
                  <div class="engine-choice-grid">
                    <button
                      class:active={activeEngine === "classic"}
                      class:selected-transition={engineTransition === "classic"}
                      class:transition-muted={engineTransition !== null && engineTransition !== "classic"}
                      class="engine-choice-card"
                      type="button"
                      disabled={serviceConfigLocked || engineTransition !== null}
                      onclick={() => runAction(() => chooseZapretEngine("classic"))}
                    >
                      <span class="engine-choice-icon"><Shield size={23} /></span>
                      <span>
                        <strong>Zapret Classic</strong>
                        <small>{$t("engine.classicDescription")}</small>
                      </span>
                      {#if activeEngine === "classic"}<Check size={18} />{/if}
                    </button>
                    <button
                      class:active={activeEngine === "zapret2"}
                      class:selected-transition={engineTransition === "zapret2"}
                      class:transition-muted={engineTransition !== null && engineTransition !== "zapret2"}
                      class="engine-choice-card"
                      type="button"
                      disabled={serviceConfigLocked || engineTransition !== null}
                      onclick={() => runAction(() => chooseZapretEngine("zapret2"))}
                    >
                      <span class="engine-choice-icon"><Cpu size={23} /></span>
                      <span>
                        <strong>Zapret 2</strong>
                        <small>{$t("engine.zapret2Description")}</small>
                      </span>
                      {#if activeEngine === "zapret2"}<Check size={18} />{/if}
                    </button>
                  </div>
                  <div class="engine-choice-note">
                    <strong>{$t("engine.whyChoice")}</strong>
                    <span>{$t("engine.whyChoiceText")}</span>
                  </div>
                </div>
              {:else}
                <div class="test-workspace">
                <div class="section-title-row">
                  <div>
                    <h3>{$t("settings.test")}</h3>
                    <p class="muted-text">
                      {activeEngine === "zapret2" ? "Zapret 2" : "Zapret Classic"} · {enginePresets.length} {$t("engine.presetsAvailable")}
                    </p>
                  </div>
                  <div class="test-head-actions">
                    <button
                      class="secondary-button compact-button"
                      type="button"
                      disabled={$testRunning || $testStopping}
                      onclick={beginEngineChange}
                    >
                      <Cpu size={15} /> {$t("engine.change")}
                    </button>
                    <button
                      class="icon-button"
                      type="button"
                      title={$t("test.export")}
                      disabled={$testRunning || $testStopping || $testResults.length === 0}
                      onclick={exportTestResultsFile}
                    >
                      <Download size={16} />
                    </button>
                    <button
                      class="icon-button"
                      type="button"
                      title={$t("test.import")}
                      disabled={$testRunning || $testStopping}
                      onclick={importTestResultsFile}
                    >
                      <Upload size={16} />
                    </button>
                  </div>
                </div>

                <TestSummary
                  result={latestEngineResult}
                  running={$testRunning}
                  stopping={$testStopping}
                  progress={$currentProgress}
                  targets={$currentTargets}
                  onDetails={(result) => (testDetails = result)}
                />

                <div class="test-mode-grid two-modes">
                  <article class="test-mode-card accent">
                    <div>
                      <strong>{$t("test.all")}</strong>
                      <span>{$t("test.allHintNew")}</span>
                    </div>
                    <button
                      class="primary-button"
                      type="button"
                      disabled={$testRunning || $testStopping || enginePresets.length === 0}
                      onclick={testAllPresets}
                    >
                      <RotateCw size={16} /> {$t("test.start")}
                    </button>
                  </article>

                  <article class="test-mode-card">
                    <div>
                      <strong>{$t("test.selected")}</strong>
                      <span>{$t("test.selectedHint")}</span>
                    </div>
                    <button
                      class="secondary-button"
                      type="button"
                      disabled={$testRunning || $testStopping || enginePresets.length === 0}
                      onclick={openPresetTestSelection}
                    >
                      <Check size={16} /> {$t("test.choose")}
                    </button>
                  </article>
                </div>

                {#if $testRunning || $testStopping}
                  <button
                    class="secondary-button stop-test-button"
                    type="button"
                    disabled={!$testRunning || $testStopping}
                    onclick={stopPresetTest}
                  >
                    <X size={16} /> {$testStopping ? $t("test.stopping") : $t("test.stop")}
                  </button>
                {/if}

                <details class="test-note" bind:open={realityExpanded}>
                  <summary>
                    <strong>{$t("test.realityTitle")}</strong>
                    <ChevronDown size={16} />
                  </summary>
                  <span>{$t("test.realityText")}</span>
                </details>

                {#if displayedEngineResults.length > 0}
                  <section class="test-results-section">
                    <button
                      class="test-results-toggle"
                      type="button"
                      aria-expanded={testResultsExpanded}
                      onclick={() => (testResultsExpanded = !testResultsExpanded)}
                    >
                      <span>
                        <strong>{$t("test.savedResults")}</strong>
                        <small>
                          {$t("test.completedAt", { time: formatTestTime(displayedEngineResults[0].finishedAt) })}
                          · {displayedEngineResults.length} {$t("test.resultsCount")}
                        </small>
                      </span>
                      <ChevronDown size={17} />
                    </button>
                    {#if testResultsExpanded}
                      <div class="collapsible-transition" transition:slide={{ duration: 220 }}>
                        <TestRecommendations
                          results={displayedEngineResults}
                          onUse={(result) => runAction(() => useRecommendedPreset(result.presetId))}
                          onDetails={(result) => (testDetails = result)}
                        />
                      </div>
                    {/if}
                  </section>
                {/if}

                <details class="test-targets-card" bind:open={targetsExpanded}>
                  <summary class="test-targets-head">
                    <div>
                      <strong>{$t("test.targets")}</strong>
                      <span>{effectiveTestTargets.filter((target) => target.enabled).length} {$t("test.targetsEnabled")}</span>
                    </div>
                    <span class="test-target-actions">
                      <button
                        class="icon-button"
                        type="button"
                        title={$t("test.editTargets")}
                        disabled={$testRunning || $testStopping}
                        onclick={(event) => {
                          event.preventDefault();
                          openTargetEditor();
                        }}
                      >
                        <Pencil size={16} />
                      </button>
                      <ChevronDown size={17} />
                    </span>
                  </summary>
                  <div class="test-target-groups">
                    {#each targetGroups as group}
                      <article>
                        <span class="target-service-dot"></span>
                        <div>
                          <strong>{group.service}</strong>
                          <small>
                            {group.targets.filter((target) => target.enabled).length}/{group.targets.length} {$t("test.active")}
                          </small>
                        </div>
                      </article>
                    {/each}
                  </div>
                </details>
                </div>
              {/if}
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
            <section class="diagnostics-workspace">
              <div class="section-title-row diagnostics-title-row">
                <div>
                  <h3>{$t("diagnostics.title")}</h3>
                  <p class="muted-text">{$t("diagnostics.subtitle")}</p>
                </div>
                <div class="diagnostics-title-actions">
                  <button class="icon-button" type="button" title={$t("common.refresh")} onclick={() => runAction(loadDiagnostics)}>
                    <RotateCw size={16} />
                  </button>
                  <button class="secondary-button compact-button" type="button" onclick={openReport}>
                    {$t("report.title")}
                  </button>
                </div>
              </div>
              <DiagnosticsPanel diagnostics={$diagnostics} onOpenPath={(path) => commands.openPath(path)} />
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
        <div class="activity-source-grid" style={`--activity-index: ${activitySourceIndex}`}>
          <span class="activity-tab-slider" aria-hidden="true"></span>
          <button class:active={activitySource === "app"} type="button" onclick={() => selectActivitySource("app")}>
            <span class="activity-source-indicator app"></span>
            <span>
              <strong>App</strong>
              <small>{$logs.filter((line) => line.source === "app" || line.source === "tests").length} {$t("activity.events")}</small>
            </span>
          </button>
          <button class:active={activitySource === "engine"} type="button" onclick={() => selectActivitySource("engine")}>
            <span class="activity-source-indicator engine"></span>
            <span>
              <strong>{activeEngine === "zapret2" ? "Zapret 2" : "Zapret"}</strong>
              <small>{$appState.zapret.state} · {$logs.filter((line) => line.source === "zapret").length} {$t("activity.events")}</small>
            </span>
          </button>
          <button class:active={activitySource === "tgWs"} type="button" onclick={() => selectActivitySource("tgWs")}>
            <span class="activity-source-indicator tg"></span>
            <span>
              <strong>tg-ws</strong>
              <small>{$appState.tgWs.state} · {$logs.filter((line) => line.source === "tgWs").length} {$t("activity.events")}</small>
            </span>
          </button>
        </div>
        <div class="activity-log-viewport">
          {#key activitySource}
            <div
              class="activity-log-slide"
              in:fly={{ x: activityDirection * 42, duration: 220, opacity: 0 }}
              out:fly={{ x: activityDirection * -28, duration: 150, opacity: 0 }}
            >
              <div class="activity-log-head">
                <div>
                  <strong>{activitySource === "app" ? "ZUI" : activitySource === "engine" ? (activeEngine === "zapret2" ? "Zapret 2" : "Zapret") : "tg-ws"}</strong>
                  <span>{$t("activity.latestEvents")}</span>
                </div>
                <button class="secondary-button compact-button" type="button" onclick={clearLogView}>{$t("activity.clear")}</button>
              </div>
              <LogViewer lines={activityLines} emptyText={$t("activity.channelEmpty")} />
            </div>
          {/key}
        </div>
      </section>
    {/if}

    {#if error}
      <div class="notification-region" aria-live="assertive" aria-atomic="true">
        <article class="app-notification notification-error">
          <span class="notification-icon" aria-hidden="true"><AlertTriangle size={18} /></span>
          <div class="notification-content">
            <strong>{$t("common.error")}</strong>
            <p>{error}</p>
          </div>
          <button class="notification-close" type="button" onclick={() => (error = null)} title={$t("common.close")}>
            <X size={15} />
          </button>
          <span class="notification-timer" aria-hidden="true"></span>
        </article>
      </div>
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
        <div class="about-panel about-app-panel" role="dialog" aria-modal="true" aria-label={$t("about.title")}>
          <header class="about-hero">
            <div class="about-brand">
              <div class="about-mark" aria-hidden="true">
                <img src="/zui-icon.png" alt="" />
              </div>
              <div>
                <div class="about-title-line">
                  <h3>ZUI</h3>
                  <span>v{appVersion}</span>
                </div>
                <p>{$t("about.tagline")}</p>
              </div>
            </div>
            <button class="icon-button" type="button" onclick={() => (aboutOpen = false)} title={$t("common.close")}>
              <X size={18} />
            </button>
          </header>

          <section class="about-section">
            <div class="about-section-heading">
              <div>
                <span class="about-eyebrow">{$t("about.developer")}</span>
                <h4>amantesnihilo</h4>
              </div>
            </div>
            <div class="about-developer-line">
              <button type="button" onclick={() => commands.openUrl("https://github.com/AmantesNihilo")}>
                github.com/AmantesNihilo
              </button>
              <span>{$t("about.developerRole")}</span>
            </div>
          </section>

          <section class="about-section">
            <div class="about-section-heading">
              <div>
                <span class="about-eyebrow">Open source</span>
                <h4>{$t("about.usedProjects")}</h4>
              </div>
            </div>
            <div class="credit-list">
              <div>
                <button type="button" onclick={() => commands.openUrl("https://github.com/bol-van/zapret")}>bol-van/zapret</button>
                <span>{$t("about.zapretRole")}</span>
              </div>
              <div>
                <button type="button" onclick={() => commands.openUrl("https://github.com/bol-van/zapret2")}>bol-van/zapret2</button>
                <span>{$t("about.zapret2Role")}</span>
              </div>
              <div>
                <button type="button" onclick={() => commands.openUrl("https://github.com/Flowseal/tg-ws-proxy")}>Flowseal/tg-ws-proxy</button>
                <span>{$t("about.tgProxyRole")}</span>
              </div>
              <div>
                <button type="button" onclick={() => commands.openUrl("https://github.com/Flowseal/zapret-discord-youtube")}>Flowseal/zapret-discord-youtube</button>
                <span>{$t("about.flowsealRole")}</span>
              </div>
              <div>
                <button type="button" onclick={() => commands.openUrl("https://github.com/hyperion-cs/dpi-checkers")}>hyperion-cs/dpi-checkers</button>
                <span>{$t("about.testSuiteRole")}</span>
              </div>
            </div>
          </section>

          <div class="about-note">
            <Shield size={17} />
            <p>{$t("about.openSourceNote")}</p>
          </div>
        </div>
      </div>
    {/if}

    {#if reportOpen}
      <div class="about-overlay" role="presentation" onclick={(event) => event.currentTarget === event.target && (reportOpen = false)}>
        <div class="about-panel report-panel" role="dialog" aria-modal="true" aria-label={$t("report.title")}>
          <header>
            <div>
              <h3>{$t("report.title")}</h3>
              <p>ZUI {appVersion}</p>
            </div>
            <button class="icon-button" type="button" onclick={() => (reportOpen = false)} title={$t("common.close")}>
              <X size={18} />
            </button>
          </header>
          <textarea readonly rows="18" value={reportText || buildSupportReport()}></textarea>
          <button class="secondary-button" type="button" onclick={copySupportReport}>
            {reportCopied ? $t("report.copied") : $t("report.copy")}
          </button>
        </div>
      </div>
    {/if}

    {#if updateOpen && updateInfo}
      <div class="about-overlay" role="presentation" onclick={(event) => event.currentTarget === event.target && postponeUpdate()}>
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
              onclick={postponeUpdate}
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
            <button class="secondary-button" type="button" disabled={updateInstalling} onclick={postponeUpdate}>
              {$t("update.later")}
            </button>
            <button
              class="secondary-button"
              type="button"
              disabled={updateInstalling || (!updateInfo.releaseUrl && !updateInfo.portableAsset)}
              onclick={updateInfo.distribution === "portable" ? openPortableDownload : openUpdateRelease}
            >
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

    {#if testSelectionOpen}
      <div class="about-overlay workflow-dialog-overlay" role="presentation" onclick={(event) => event.currentTarget === event.target && (testSelectionOpen = false)}>
        <div class="workflow-dialog test-selection-panel" role="dialog" aria-modal="true" aria-label={$t("test.selected")}>
          <header class="dialog-header">
            <div>
              <h3>{$t("test.choosePresets")}</h3>
              <p>{$t("test.selectedCount", { count: selectedTestPresetIds.length })}</p>
            </div>
            <button class="icon-button" type="button" onclick={() => (testSelectionOpen = false)} title={$t("common.close")}>
              <X size={18} />
            </button>
          </header>
          <div class="dialog-body preset-dialog-body">
          <label class="modal-search">
            {$t("common.search")}
            <input
              value={testSelectionSearch}
              placeholder={$t("presets.search")}
              oninput={(event) => (testSelectionSearch = event.currentTarget.value)}
            />
          </label>
          <div class="test-preset-picker">
            {#each selectableTestPresets as preset}
              <button
                class:active={selectedTestPresetIds.includes(preset.id)}
                type="button"
                onclick={() => toggleTestPreset(preset.id)}
              >
                <span class="picker-check">
                  {#if selectedTestPresetIds.includes(preset.id)}<Check size={14} />{/if}
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
            <button class="secondary-button" type="button" onclick={() => (selectedTestPresetIds = selectableTestPresets.map((preset) => preset.id))}>
              {$t("test.selectAll")}
            </button>
            <button class="primary-button" type="button" disabled={selectedTestPresetIds.length === 0} onclick={testSelectedPresets}>
              <RotateCw size={16} /> {$t("test.startSelected", { count: selectedTestPresetIds.length })}
            </button>
          </div>
          </footer>
        </div>
      </div>
    {/if}

    {#if targetEditorOpen}
      <div class="about-overlay workflow-dialog-overlay" role="presentation" onclick={(event) => event.currentTarget === event.target && (targetEditorOpen = false)}>
        <div class="workflow-dialog target-editor-panel" role="dialog" aria-modal="true" aria-label={$t("test.editTargets")}>
          <header class="dialog-header">
            <div>
              <h3>{$t("test.editTargets")}</h3>
              <p>{$t("test.editTargetsHint")}</p>
            </div>
            <button class="icon-button" type="button" onclick={() => (targetEditorOpen = false)} title={$t("common.close")}>
              <X size={18} />
            </button>
          </header>
          <div class="dialog-body target-dialog-body">
          <div class="target-editor-list">
            {#each targetEditorTargets as target, index}
              <article class:disabled={!target.enabled}>
                <button class="target-enable-button" type="button" onclick={() => toggleEditorTarget(index)}>
                  <span>{#if target.enabled}<Check size={13} />{/if}</span>
                </button>
                <div>
                  <strong>{target.name}</strong>
                  <small>{target.service} · {target.value}</small>
                </div>
                <button class="icon-button" type="button" title={$t("common.close")} onclick={() => removeEditorTarget(index)}>
                  <X size={14} />
                </button>
              </article>
            {/each}
          </div>
          <div class="custom-target-form">
            <strong>{$t("test.addTarget")}</strong>
            <div class="form-row target-form-row">
              <label>
                {$t("test.service")}
                <input value={customTargetService} oninput={(event) => (customTargetService = event.currentTarget.value)} />
              </label>
              <label>
                {$t("test.targetName")}
                <input value={customTargetName} oninput={(event) => (customTargetName = event.currentTarget.value)} />
              </label>
            </div>
            <label>
              URL / PING
              <input
                value={customTargetValue}
                placeholder="https://example.com или PING:1.1.1.1"
                oninput={(event) => (customTargetValue = event.currentTarget.value)}
              />
            </label>
            <button class="secondary-button" type="button" onclick={addEditorTarget}>
              <Plus size={16} /> {$t("test.addTarget")}
            </button>
          </div>
          </div>
          <footer class="dialog-footer">
          <div class="modal-action-row">
            <button class="secondary-button" type="button" onclick={resetTestTargets}>
              {$t("test.resetTargets")}
            </button>
            <button class="primary-button" type="button" onclick={saveTargetEditor}>
              {$t("common.save")}
            </button>
          </div>
          </footer>
        </div>
      </div>
    {/if}

    {#if minimizeChoiceOpen}
      <div class="about-overlay workflow-dialog-overlay" role="presentation">
        <div class="workflow-dialog minimize-choice-panel" role="dialog" aria-modal="true" aria-label={$t("minimize.title")}>
          <header class="dialog-header">
            <div>
              <h3>{$t("minimize.title")}</h3>
              <p>{$t("minimize.text")}</p>
            </div>
            <button class="icon-button" type="button" onclick={() => (minimizeChoiceOpen = false)} title={$t("common.close")}>
              <X size={18} />
            </button>
          </header>
          <div class="dialog-body minimize-choice-grid">
            <button type="button" onclick={() => chooseMinimizeBehavior("taskbar")}>
              <span class="minimize-choice-icon"><Minus size={20} /></span>
              <span>
                <strong>{$t("minimize.taskbar")}</strong>
                <small>{$t("minimize.taskbarHint")}</small>
              </span>
            </button>
            <button type="button" onclick={() => chooseMinimizeBehavior("tray")}>
              <span class="minimize-choice-icon"><ChevronDown size={20} /></span>
              <span>
                <strong>{$t("minimize.tray")}</strong>
                <small>{$t("minimize.trayHint")}</small>
              </span>
            </button>
          </div>
          <footer class="dialog-footer minimize-choice-footer">
            <label class="check-row">
              <input
                type="checkbox"
                checked={minimizeRememberChoice}
                onchange={(event) => (minimizeRememberChoice = event.currentTarget.checked)}
              />
              {$t("minimize.dontAsk")}
            </label>
          </footer>
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
              <p>{recommendationLabel(testDetails.recommendation)} · {modeLabel(testDetails.mode)}</p>
            </div>
            <button class="icon-button" type="button" onclick={() => (testDetails = null)} title={$t("common.close")}>
              <X size={18} />
            </button>
          </div>

          <div class="test-details-score">
            <div class="score-ring" class:partial={testDetails.recommendation === "partial"} class:failed={testDetails.recommendation === "notRecommended"}>{testDetails.score}</div>
            <div>
              <strong>{$t("test.overall")}</strong>
              <span>{$t("test.passed", { ok: testDetails.ok, total: testDetails.total })}</span>
            </div>
          </div>

          <div class="test-details-meta">
            <span><strong>{$t("test.version")}</strong>{testDetails.presetVersion || "-"}</span>
            <span><strong>{$t("test.cachedAt")}</strong>{formatTestTime(testDetails.cachedAt || testDetails.finishedAt)}</span>
            <span><strong>{$t("test.mode")}</strong>{modeLabel(testDetails.mode)}</span>
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
