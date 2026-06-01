import { writable } from "svelte/store";
import { commands } from "$lib/api/commands";
import {
  onTestCancelled,
  onTestBatchFinished,
  onTestFinished,
  onTestPresetFinished,
  onTestPresetStarted,
  onTestStarted,
  onTestStopping,
  onTestTargetFinished
} from "$lib/api/events";
import type { TestResult, TestTargetResult } from "$lib/api/types";

export const testResults = writable<TestResult[]>([]);
export const testRunning = writable(false);
export const testStopping = writable(false);
export const currentTestId = writable<string | null>(null);
export const currentPresetName = writable<string | null>(null);
export const currentTargets = writable<TestTargetResult[]>([]);
export const batchRecommendations = writable<TestResult[]>([]);

export async function loadTestResults() {
  testResults.set(await commands.getTestResults());
}

export async function bindTestEvents() {
  const unlistenStarted = await onTestStarted((id) => {
    currentTestId.set(id);
    currentPresetName.set(null);
    currentTargets.set([]);
    batchRecommendations.set([]);
    testRunning.set(true);
    testStopping.set(false);
  });
  const unlistenStopping = await onTestStopping(() => {
    testStopping.set(true);
  });
  const unlistenPresetStarted = await onTestPresetStarted((preset) => {
    currentPresetName.set(preset.name);
    currentTargets.set([]);
  });
  const unlistenTarget = await onTestTargetFinished((result) => {
    currentTargets.update((items) => [...items, result]);
  });
  const unlistenPresetFinished = await onTestPresetFinished((result) => {
    testResults.update((items) => [...items, result]);
    batchRecommendations.update((items) =>
      [...items, result].sort((a, b) => b.score - a.score || b.ok - a.ok)
    );
  });
  const unlistenFinished = await onTestFinished((result) => {
    testResults.update((items) => [...items, result]);
    testRunning.set(false);
    testStopping.set(false);
    currentTestId.set(null);
    currentPresetName.set(null);
  });
  const unlistenBatchFinished = await onTestBatchFinished((results) => {
    batchRecommendations.set(results);
    testRunning.set(false);
    testStopping.set(false);
    currentTestId.set(null);
    currentPresetName.set(null);
  });
  const unlistenCancelled = await onTestCancelled(() => {
    testRunning.set(false);
    testStopping.set(false);
    currentTestId.set(null);
    currentPresetName.set(null);
  });

  return () => {
    unlistenStarted();
    unlistenStopping();
    unlistenPresetStarted();
    unlistenTarget();
    unlistenPresetFinished();
    unlistenFinished();
    unlistenBatchFinished();
    unlistenCancelled();
  };
}

export async function runQuickPresetTest(presetId: string) {
  await commands.runPresetTest(presetId);
}

export async function runBestPresetTest(presetIds: string[], maxCount = 12) {
  await commands.runBestPresetTest(presetIds, maxCount);
}

export async function runAllPresetTest(presetIds: string[]) {
  await commands.runAllPresetTest(presetIds);
}

export async function cancelPresetTest() {
  testStopping.set(true);
  try {
    await commands.cancelPresetTest();
  } catch (error) {
    testStopping.set(false);
    throw error;
  }
}
