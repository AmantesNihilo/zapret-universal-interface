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
    let disposed = false;
    const subscriptions: (() => void)[] = [];
    const blockContextMenu = (event: MouseEvent) => event.preventDefault();

    document.addEventListener("contextmenu", blockContextMenu);

    async function init() {
      let settingsLoaded = false;
      const startupTasks = [
        commands.getAppVersion().then((version) => {
          if (!disposed) options.setAppVersion(version);
        }),
        loadAppState(),
        loadSettings().then(() => { settingsLoaded = true; }),
        loadProfiles(),
        loadPresets(),
        loadLogs(),
        loadTestResults(),
        loadDiagnostics(),
        commands.getTestTargetManifest().then((targets) => {
          if (!disposed) options.setDefaultTestTargets(targets);
        })
      ];
      const results = await Promise.allSettled(startupTasks);
      if (disposed) return;
      const failures = results
        .filter((result): result is PromiseRejectedResult => result.status === "rejected")
        .map((result) => String(result.reason));
      if (failures.length > 0) {
        options.setError([...new Set(failures)].join("; "));
      }

      // A failed settings load must not opt the user in using store defaults.
      // Checking updates is independent of optional event subscriptions.
      if (settingsLoaded && get(settings).checkUpdatesOnLaunch) {
        void options.checkForUpdatesOnLaunch();
      }

      try {
        const binders = [
          bindAppStateEvents,
          bindLogEvents,
          bindTestEvents,
          () => onTrayAction((action) => {
            if (!disposed && action === "settings") options.openSettings();
          }),
          () => onOperationFailed((message) => {
            if (!disposed) options.setError(message);
          })
        ];
        for (const bind of binders) {
          const unlisten = await bind();
          // An asynchronous listener may finish registering after the owner was destroyed.
          if (disposed) {
            unlisten();
            return;
          }
          subscriptions.push(unlisten);
        }
      } catch (caught) {
        if (!disposed) options.setError(String(caught));
      }
    }

    void init();

    return () => {
      disposed = true;
      for (const unlisten of subscriptions) unlisten();
      document.removeEventListener("contextmenu", blockContextMenu);
      unbindUiActivity();
    };
  });
}
