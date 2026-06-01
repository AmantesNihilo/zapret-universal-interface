import { listen } from "@tauri-apps/api/event";
import type { AppState, LogLine, Preset, TestResult, TestTargetResult } from "./types";

export function onAppStateChanged(callback: (state: AppState) => void) {
  return listen<AppState>("app_state_changed", (event) => callback(event.payload));
}

export function onLogLine(callback: (line: LogLine) => void) {
  return listen<LogLine>("log_line", (event) => callback(event.payload));
}

export function onTrayAction(callback: (action: string) => void) {
  return listen<string>("tray_action", (event) => callback(event.payload));
}

export function onOperationFailed(callback: (message: string) => void) {
  return listen<string>("operation_failed", (event) => callback(event.payload));
}

export function onTestStarted(callback: (id: string) => void) {
  return listen<string>("test_started", (event) => callback(event.payload));
}

export function onTestTargetFinished(callback: (result: TestTargetResult) => void) {
  return listen<TestTargetResult>("test_target_finished", (event) => callback(event.payload));
}

export function onTestPresetStarted(callback: (preset: Preset) => void) {
  return listen<Preset>("test_preset_started", (event) => callback(event.payload));
}

export function onTestPresetFinished(callback: (result: TestResult) => void) {
  return listen<TestResult>("test_preset_finished", (event) => callback(event.payload));
}

export function onTestFinished(callback: (result: TestResult) => void) {
  return listen<TestResult>("test_finished", (event) => callback(event.payload));
}

export function onTestBatchFinished(callback: (results: TestResult[]) => void) {
  return listen<TestResult[]>("test_batch_finished", (event) => callback(event.payload));
}

export function onTestCancelled(callback: () => void) {
  return listen<string>("test_cancelled", () => callback());
}

export function onTestStopping(callback: () => void) {
  return listen<string>("test_stopping", () => callback());
}
