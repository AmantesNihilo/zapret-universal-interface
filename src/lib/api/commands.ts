import { invoke } from "@tauri-apps/api/core";
import type {
  AppState,
  ConflictProcess,
  Diagnostics,
  LogLine,
  Preset,
  Profile,
  ProfilesFile,
  ServiceStatus,
  Settings,
  TestResult,
  UpdateCheck
} from "./types";

export const commands = {
  getAppState: () => invoke<AppState>("get_app_state"),
  getSettings: () => invoke<Settings>("get_settings"),
  saveSettings: (settingsPayload: Settings) =>
    invoke<Settings>("save_settings", { settingsPayload }),
  getProfiles: () => invoke<ProfilesFile>("get_profiles"),
  saveProfile: (profile: Profile) => invoke<ProfilesFile>("save_profile", { profile }),
  setActiveProfile: (profileId: string) =>
    invoke<ProfilesFile>("set_active_profile", { profileId }),
  deleteProfile: (profileId: string) => invoke<ProfilesFile>("delete_profile", { profileId }),
  discoverZapret: () => invoke<Preset[]>("discover_zapret"),
  listPresets: () => invoke<Preset[]>("list_presets"),
  setPresetFavorite: (presetId: string, favorite: boolean) =>
    invoke<Preset[]>("set_preset_favorite", { presetId, favorite }),
  setPresetHidden: (presetId: string, hidden: boolean) =>
    invoke<Preset[]>("set_preset_hidden", { presetId, hidden }),
  startProfile: () => invoke<AppState>("start_profile"),
  stopProfile: () => invoke<AppState>("stop_profile"),
  startZapret: (presetId: string) => invoke<ServiceStatus>("start_zapret", { presetId }),
  stopZapret: () => invoke<ServiceStatus>("stop_zapret"),
  getConflictingApps: () => invoke<ConflictProcess[]>("get_conflicting_apps"),
  killConflictingApps: (pids: number[]) =>
    invoke<ConflictProcess[]>("kill_conflicting_apps", { pids }),
  startTgWs: (host: string, port: number, secret: string) =>
    invoke<ServiceStatus>("start_tg_ws", { host, port, secret }),
  stopTgWs: () => invoke<ServiceStatus>("stop_tg_ws"),
  getLogs: () => invoke<LogLine[]>("get_logs"),
  clearLogs: () => invoke<void>("clear_logs"),
  openPath: (path: string) => invoke<void>("open_path", { path }),
  openUrl: (url: string) => invoke<void>("open_url", { url }),
  revealPath: (path: string) => invoke<void>("reveal_path", { path }),
  minimizeToTray: () => invoke<void>("minimize_to_tray"),
  quitApp: () => invoke<void>("quit_app"),
  showMainWindow: () => invoke<void>("show_main_window"),
  setWindowLayout: (layout: Settings["layoutOrientation"]) =>
    invoke<void>("set_window_layout", { layout }),
  runPresetTest: (presetId: string) => invoke<string>("run_preset_test", { presetId }),
  runBestPresetTest: (presetIds: string[], maxCount: number) =>
    invoke<string>("run_best_preset_test", { presetIds, maxCount }),
  runAllPresetTest: (presetIds: string[]) =>
    invoke<string>("run_all_preset_test", { presetIds }),
  cancelPresetTest: () => invoke<void>("cancel_preset_test"),
  getTestResults: () => invoke<TestResult[]>("get_test_results"),
  getDiagnostics: () => invoke<Diagnostics>("get_diagnostics"),
  collectSupportReport: () => invoke<string>("collect_support_report"),
  checkForUpdate: () => invoke<UpdateCheck>("check_for_update"),
  installUpdate: () => invoke<void>("install_update")
};
