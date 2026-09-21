import { writable } from "svelte/store";
import { commands } from "$lib/api/commands";
import {
  onTestCancelled,
  onTestBatchFinished,
  onTestFinished,
  onTestPresetFinished,
  onTestPresetStarted,
  onTestProgress,
  onTestStarted,
  onTestStopping,
  onTestTargetFinished
} from "$lib/api/events";
import type { TestProgress, TestResult, TestTargetResult } from "$lib/api/types";

const MAX_STORED_RESULTS = 100;

export const testResults = writable<TestResult[]>([]);
export const testRunning = writable(false);
export const testStopping = writable(false);
export const currentTestId = writable<string | null>(null);
export const currentPresetName = writable<string | null>(null);
export const currentTargets = writable<TestTargetResult[]>([]);
export const currentProgress = writable<TestProgress | null>(null);
export const batchRecommendations = writable<TestResult[]>([]);

export async function loadTestResults() {
  testResults.set((await commands.getTestResults()).slice(-MAX_STORED_RESULTS));
}

export async function importTestResults(path: string) {
  const results = await commands.importTestResults(path);
  testResults.set(results.slice(-MAX_STORED_RESULTS));
  return results;
}

export async function bindTestEvents() {
  const unlistenStarted = await onTestStarted((id) => {
    currentTestId.set(id);
    currentPresetName.set(null);
    currentTargets.set([]);
    currentProgress.set(null);
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
  const unlistenProgress = await onTestProgress((progress) => {
    currentProgress.set(progress);
  });
  const unlistenPresetFinished = await onTestPresetFinished((result) => {
    testResults.update((items) => [...items, result].slice(-MAX_STORED_RESULTS));
    batchRecommendations.update((items) =>
      [...items, result].sort((a, b) => b.score - a.score || b.ok - a.ok)
    );
  });
  const unlistenFinished = await onTestFinished((result) => {
    testResults.update((items) => [...items, result].slice(-MAX_STORED_RESULTS));
    testRunning.set(false);
    testStopping.set(false);
    currentTestId.set(null);
    currentPresetName.set(null);
    currentTargets.set([]);
    currentProgress.set(null);
    batchRecommendations.set([]);
  });
  const unlistenBatchFinished = await onTestBatchFinished(() => {
    testRunning.set(false);
    testStopping.set(false);
    currentTestId.set(null);
    currentPresetName.set(null);
    currentTargets.set([]);
    currentProgress.set(null);
    batchRecommendations.set([]);
  });
  const unlistenCancelled = await onTestCancelled(() => {
    testRunning.set(false);
    testStopping.set(false);
    currentTestId.set(null);
    currentPresetName.set(null);
    currentTargets.set([]);
    currentProgress.set(null);
    batchRecommendations.set([]);
  });

  return () => {
    unlistenStarted();
    unlistenStopping();
    unlistenPresetStarted();
    unlistenTarget();
    unlistenProgress();
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

export async function runSelectedPresetTests(presetIds: string[]) {
  await commands.runSelectedPresetTest(presetIds);
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
