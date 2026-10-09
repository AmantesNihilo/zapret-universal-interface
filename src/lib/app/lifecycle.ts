import { onMount } from "svelte";
import { get } from "svelte/store";
import { commands } from "$lib/api/commands";
import { onOperationFailed, onTrayAction } from "$lib/api/events";
import type { TestTargetConfig } from "$lib/api/types";
import { bindUiActivity } from "$lib/uiActivity";
import { bindAppStateEvents, loadAppState } from "$lib/stores/appState";
import { loadDiagnostics } from "$lib/stores/diagnostics";
import { bindLogEvents, loadLogs } from "$lib/stores/logs";
import { loadPresets } from "$lib/stores/presets";
import { loadProfiles } from "$lib/stores/profiles";
import { loadSettings, settings } from "$lib/stores/settings";
import { bindTestEvents, loadTestResults } from "$lib/stores/tests";

interface AppLifecycleOptions {
  setAppVersion: (version: string) => void;
  setDefaultTestTargets: (targets: TestTargetConfig[]) => void;
  setError: (message: string) => void;
  openSettings: () => void;
  checkForUpdatesOnLaunch: () => void | Promise<void>;
}

export function useAppLifecycle(options: AppLifecycleOptions) {
  onMount(() => {
    const unbindUiActivity = bindUiActivity();
    let unlistenState: (() => void) | undefined;
    let unlistenLogs: (() => void) | undefined;
    let unlistenTray: (() => void) | undefined;
    let unlistenTests: (() => void) | undefined;
    let unlistenFailures: (() => void) | undefined;
    const blockContextMenu = (event: MouseEvent) => event.preventDefault();

    document.addEventListener("contextmenu", blockContextMenu);

    async function init() {
      const startupTasks = [
        commands.getAppVersion().then(options.setAppVersion),
        loadAppState(),
        loadSettings(),
        loadProfiles(),
        loadPresets(),
        loadLogs(),
        loadTestResults(),
        loadDiagnostics(),
        commands.getTestTargetManifest().then(options.setDefaultTestTargets)
      ];
      const results = await Promise.allSettled(startupTasks);
      const failures = results
        .filter((result): result is PromiseRejectedResult => result.status === "rejected")
        .map((result) => String(result.reason));
      if (failures.length > 0) {
        options.setError([...new Set(failures)].join("; "));
      }

      try {
        unlistenState = await bindAppStateEvents();
        unlistenLogs = await bindLogEvents();
        unlistenTests = await bindTestEvents();
        unlistenTray = await onTrayAction((action) => {
          if (action === "settings") options.openSettings();
        });
        unlistenFailures = await onOperationFailed(options.setError);
        if (get(settings).checkUpdatesOnLaunch) {
          void options.checkForUpdatesOnLaunch();
        }
      } catch (caught) {
        options.setError(String(caught));
      }
    }

    void init();

    return () => {
      unlistenState?.();
      unlistenLogs?.();
      unlistenTray?.();
      unlistenTests?.();
      unlistenFailures?.();
      document.removeEventListener("contextmenu", blockContextMenu);
      unbindUiActivity();
    };
  });
}
