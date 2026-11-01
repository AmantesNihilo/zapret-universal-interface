import { beforeEach, describe, expect, it, vi } from "vitest";
import { invokeValidated } from "$lib/api/client";
import { commands } from "$lib/api/commands";
import { parseConnectivityReport, parseUpdateCheck } from "$lib/api/decoders";
import { readString, ResponseValidationError } from "$lib/api/validation";
import type { UpdateCheck, TgWsConnectivityReport } from "$lib/api/types";

const bridge = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke: bridge.invoke }));

const update: UpdateCheck = {
  updateAvailable: true, currentVersion: "2.2.4", latestVersion: "2.2.5",
  canInstall: true, distribution: "installed",
  installerAsset: { name: "ZUI-setup.exe", downloadUrl: "https://example.test/setup", size: 123, kind: "setup" }
};
const report: TgWsConnectivityReport = {
  kind: "cfProxy", allOk: true,
  probes: [{ domain: "example.test", dc: 2, target: "kws2.example.test", ok: true, latencyMs: 10, detail: "HTTP 101" }]
};

beforeEach(() => { bridge.invoke.mockReset(); });

describe("validated native responses", () => {
  it("infers a result from the decoder and forwards arguments", async () => {
    bridge.invoke.mockResolvedValue("2.2.4");
    const result = await invokeValidated("version", value => readString(value, "version"), { verbose: false });
    expect(result.toUpperCase()).toBe("2.2.4");
    expect(bridge.invoke).toHaveBeenCalledWith("version", { verbose: false });
  });

  it("uses the validated boundary in real update and connectivity commands", async () => {
    bridge.invoke.mockResolvedValueOnce(update).mockResolvedValueOnce(report);
    expect(await commands.checkForUpdate()).toMatchObject(update);
    expect(await invokeValidated("test", parseConnectivityReport)).toEqual(report);
  });

  it("passes native rejection through without pretending it is a response", async () => {
    bridge.invoke.mockRejectedValue("offline");
    const result = await invokeValidated("version", value => readString(value, "version"))
      .catch((caught: unknown) => caught);
    expect(result).toBe("offline");
  });

  it("reports a field and command without echoing received secrets", async () => {
    bridge.invoke.mockResolvedValue({ ...update, canInstall: "secret-value" });
    const error = await commands.checkForUpdate().catch((caught: unknown) => caught);
    expect(error).toBeInstanceOf(ResponseValidationError);
    if (!(error instanceof ResponseValidationError)) throw new Error("Expected validation error");
    expect(error.command).toBe("check_for_update");
    expect(error.issue.path).toBe("update.canInstall");
    expect(error.message).not.toContain("secret-value");
  });

  it.each([null, [], "response", 42, true])("rejects a non-object update response: %j", value => {
    expect(() => parseUpdateCheck(value)).toThrow(ResponseValidationError);
  });

  it.each([
    { ...update, updateAvailable: "yes" },
    { ...update, currentVersion: null },
    { ...update, distribution: 123 },
    { ...update, latestVersion: [] },
    { ...update, installerAsset: {} },
    { ...update, installerAsset: { ...update.installerAsset, size: -1 } },
    { ...update, installerAsset: { ...update.installerAsset, size: 1.5 } },
    { ...update, installerAsset: { ...update.installerAsset, size: Infinity } },
    { ...update, portableAsset: { ...update.installerAsset, downloadUrl: false } }
  ])("rejects malformed fields and nested assets: %j", value => {
    expect(() => parseUpdateCheck(value)).toThrow(ResponseValidationError);
  });

  it("accepts absent and null optional fields and keeps future distribution names", () => {
    expect(parseUpdateCheck({
      updateAvailable: false, currentVersion: "2.2.4", distribution: "future-mode", canInstall: false,
      latestVersion: null, installerAsset: null
    })).toMatchObject({ distribution: "future-mode", latestVersion: null, installerAsset: null });
  });

  it.each([
    { ...report, kind: "other" },
    { ...report, allOk: "yes" },
    { ...report, probes: null },
    { ...report, probes: [null] },
    { ...report, probes: [{ domain: "x" }] },
    { ...report, probes: [{ ...report.probes[0], dc: 1.5 }] },
    { ...report, probes: [{ ...report.probes[0], latencyMs: NaN }] },
    { ...report, probes: [{ ...report.probes[0], ok: "true" }] }
  ])("rejects invalid connectivity data: %j", value => {
    expect(() => parseConnectivityReport(value)).toThrow(ResponseValidationError);
  });

  it("accepts an empty probe list and nullable latency without inventing success", () => {
    expect(parseConnectivityReport({ kind: "cfWorker", allOk: false, probes: [] }).probes).toEqual([]);
    const parsed = parseConnectivityReport({
      ...report, probes: [{ ...report.probes[0], latencyMs: null }]
    });
    expect(parsed.probes[0]?.latencyMs).toBeNull();
  });
});
