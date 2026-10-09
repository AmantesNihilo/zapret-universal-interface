import type { Preset, TestResult } from "$lib/api/types";

export function isExecutablePreset(preset: Preset) {
  return preset.engine === "zapret2"
    ? preset.kind === "config"
    : preset.kind === "bat" || preset.kind === "cmd";
}

export function testSessionId(result: TestResult) {
  return result.id.startsWith("batch-") ? result.id.replace(/-\d+$/, "") : result.id;
}
