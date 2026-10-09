import { get } from "svelte/store";
import { open, save } from "@tauri-apps/plugin-dialog";
import { commands } from "$lib/api/commands";
import type { Preset, Profile, Settings, TestResult, TestTargetConfig } from "$lib/api/types";
import { t } from "$lib/stores/i18n";
import {
  cancelPresetTest,
  importTestResults,
  runAllPresetTest,
  runSelectedPresetTests,
  testResults,
  testRunning,
  testStopping
} from "$lib/stores/tests";
import { isExecutablePreset } from "./model";

interface NetworkTestControllerOptions {
  setError: (message: string | null) => void;
  runAction: (action: () => Promise<unknown>) => Promise<void>;
  updateSettings: (patch: Partial<Settings>) => Promise<void>;
}

export class NetworkTestController {
  details = $state<TestResult | null>(null);
  selectionOpen = $state(false);
  selectionSearch = $state("");
  selectedPresetIds = $state<string[]>([]);
  targetEditorOpen = $state(false);
  targetEditorTargets = $state<TestTargetConfig[]>([]);
  defaultTargets = $state<TestTargetConfig[]>([]);
  customTargetService = $state("Custom");
  customTargetName = $state("");
  customTargetValue = $state("");
  realityExpanded = $state(false);
  targetsExpanded = $state(false);
  resultsExpanded = $state(true);

  constructor(private readonly options: NetworkTestControllerOptions) {}

  async testAll(presets: Preset[]) {
    const candidates = presets
      .filter((preset) => !preset.hidden && isExecutablePreset(preset))
      .map((preset) => preset.id);
    if (candidates.length === 0) {
      this.options.setError(get(t)("test.noExecutable"));
      return;
    }
    await this.options.runAction(async () => runAllPresetTest(candidates));
  }

  openSelection(activeProfile: Profile | null) {
    this.selectedPresetIds = activeProfile?.zapretPresetId
      ? [activeProfile.zapretPresetId]
      : [];
    this.selectionSearch = "";
    this.selectionOpen = true;
  }

  togglePreset(presetId: string) {
    this.selectedPresetIds = this.selectedPresetIds.includes(presetId)
      ? this.selectedPresetIds.filter((id) => id !== presetId)
      : [...this.selectedPresetIds, presetId];
  }

  async testSelected() {
    if (this.selectedPresetIds.length === 0) {
      this.options.setError(get(t)("test.selectAtLeastOne"));
      return;
    }
    this.selectionOpen = false;
    await this.options.runAction(async () => runSelectedPresetTests(this.selectedPresetIds));
  }

  async stop() {
    if (!get(testRunning) || get(testStopping)) return;
    this.options.setError(null);
    try {
      await cancelPresetTest();
    } catch (caught) {
      this.options.setError(String(caught));
    }
  }

  async exportResults() {
    if (get(testResults).length === 0) {
      this.options.setError(get(t)("test.noResults"));
      return;
    }
    const path = await save({
      title: get(t)("test.export"),
      defaultPath: "zui-test-results.json",
      filters: [{ name: "JSON", extensions: ["json"] }]
    });
    if (path) await this.options.runAction(async () => commands.exportTestResults(path));
  }

  async importResults() {
    const path = await open({
      directory: false,
      multiple: false,
      title: get(t)("test.import"),
      filters: [{ name: "JSON", extensions: ["json"] }]
    });
    if (!path || Array.isArray(path)) return;
    await this.options.runAction(async () => {
      const results = await importTestResults(path);
      if (results.length > 0) this.resultsExpanded = true;
    });
  }

  async resetTargets() {
    await this.options.updateSettings({ testTargets: [] });
    this.targetEditorTargets = this.defaultTargets.map((target) => ({ ...target }));
  }

  openTargetEditor(targets: TestTargetConfig[]) {
    this.targetEditorTargets = targets.map((target) => ({ ...target }));
    this.customTargetService = "Custom";
    this.customTargetName = "";
    this.customTargetValue = "";
    this.targetEditorOpen = true;
  }

  toggleEditorTarget(index: number) {
    this.targetEditorTargets = this.targetEditorTargets.map((target, itemIndex) =>
      itemIndex === index ? { ...target, enabled: !target.enabled } : target
    );
  }

  removeEditorTarget(index: number) {
    this.targetEditorTargets = this.targetEditorTargets.filter((_, itemIndex) => itemIndex !== index);
  }

  async addEditorTarget() {
    const name = this.customTargetName.trim();
    const value = this.customTargetValue.trim();
    if (!name) {
      this.options.setError(get(t)("test.invalidTarget"));
      return;
    }
    const target = {
      service: this.customTargetService.trim() || "Custom",
      name,
      value,
      enabled: true
    };
    try {
      await commands.validateTestTargets([target]);
      this.targetEditorTargets = [...this.targetEditorTargets, target];
    } catch (caught) {
      this.options.setError(String(caught));
      return;
    }
    this.customTargetName = "";
    this.customTargetValue = "";
  }

  async saveTargetEditor() {
    await commands.validateTestTargets(this.targetEditorTargets);
    await this.options.updateSettings({ testTargets: this.targetEditorTargets });
    this.targetEditorOpen = false;
  }
}
