import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { flushSync, mount, unmount } from "svelte";
import { get } from "svelte/store";
import { settings } from "$lib/stores/settings";
import { profilesFile } from "$lib/stores/profiles";
import { appState } from "$lib/stores/appState";
import { diagnostics } from "$lib/stores/diagnostics";
import type { Settings, UpdateCheck } from "$lib/api/types";
import AppHarness from "./AppHarness.svelte";

const bridge = vi.hoisted(() => ({ invoke: vi.fn(), listen: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke: bridge.invoke }));
vi.mock("@tauri-apps/api/event", () => ({ listen: bridge.listen }));

const update: UpdateCheck = {
  currentVersion: "2.2.4", latestVersion: "2.2.4", updateAvailable: false,
  canInstall: false, distribution: "development"
};

function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (reason: unknown) => void;
  const promise = new Promise<T>((ok, fail) => { resolve = ok; reject = fail; });
  return { promise, resolve, reject };
}

let component: ReturnType<typeof mount> | null = null;
let target: HTMLElement;
let startupSettings: () => Promise<Settings>;
let check: () => Promise<UpdateCheck>;
const cleanups: ReturnType<typeof vi.fn>[] = [];

beforeEach(() => {
  settings.update(value => ({ ...value, language: "ru", checkUpdatesOnLaunch: false }));
  startupSettings = async () => get(settings);
  check = async () => update;
  cleanups.length = 0;
  bridge.invoke.mockReset().mockImplementation(async (name: string) => {
    switch (name) {
      case "get_app_version": return "2.2.4";
      case "get_app_state": return get(appState);
      case "get_settings": return startupSettings();
      case "get_profiles": return get(profilesFile);
      case "get_diagnostics": return get(diagnostics);
      case "list_presets":
      case "get_logs":
      case "get_test_results":
      case "get_test_target_manifest": return [];
      case "check_for_update": return check();
      default: throw new Error(`Unexpected native call: ${name}`);
    }
  });
  bridge.listen.mockReset().mockImplementation(async () => {
    const cleanup = vi.fn();
    cleanups.push(cleanup);
    return cleanup;
  });
  target = document.createElement("div");
  document.body.append(target);
});

afterEach(async () => {
  if (component) await unmount(component);
  component = null;
  target.remove();
});

function start() {
  component = mount(AppHarness, { target });
  flushSync();
}

function calls(name: string) {
  return bridge.invoke.mock.calls.filter(args => args[0] === name).length;
}

function openSettings() {
  target.querySelector<HTMLButtonElement>(".footer-dock button")!.click();
  flushSync();
}

describe("real root provider, App and lifecycle", () => {
  it("respects disabled auto-check through startup and repeated settings visits", async () => {
    start();
    await vi.waitFor(() => expect(bridge.listen).toHaveBeenCalledWith("operation_failed", expect.any(Function)));
    for (let i = 0; i < 3; i++) {
      openSettings();
      expect(target.querySelector(".update-card")?.textContent).toContain("Проверьте релизы");
      target.querySelector<HTMLButtonElement>(".settings-view .view-heading button")!.click();
      flushSync();
    }
    expect(calls("check_for_update")).toBe(0);
  });

  it("starts exactly one automatic check after settings resolve", async () => {
    const pending = deferred<Settings>();
    startupSettings = () => pending.promise;
    start();
    expect(calls("check_for_update")).toBe(0);
    pending.resolve({ ...get(settings), checkUpdatesOnLaunch: true });
    await vi.waitFor(() => expect(calls("check_for_update")).toBe(1));
    openSettings();
    await vi.waitFor(() => expect(target.querySelector(".update-card")?.textContent).toContain("2.2.4"));
    expect(calls("check_for_update")).toBe(1);
  });

  it("does not treat failed settings as permission from default settings", async () => {
    settings.update(value => ({ ...value, checkUpdatesOnLaunch: true }));
    startupSettings = async () => { throw new Error("settings unavailable"); };
    start();
    await vi.waitFor(() => expect(target.querySelector(".notification-error")?.textContent).toContain("settings unavailable"));
    expect(calls("check_for_update")).toBe(0);
    openSettings();
    target.querySelector<HTMLButtonElement>(".update-card button")!.click();
    await vi.waitFor(() => expect(calls("check_for_update")).toBe(1));
  });

  it("does not suppress auto-check when optional event subscriptions fail", async () => {
    settings.update(value => ({ ...value, checkUpdatesOnLaunch: true }));
    bridge.listen.mockRejectedValue(new Error("events unavailable"));
    start();
    await vi.waitFor(() => expect(calls("check_for_update")).toBe(1));
    expect(target.querySelector(".notification-error")?.textContent).toContain("events unavailable");
  });

  it("does not run late auto-check after a manual check during settings load", async () => {
    const pending = deferred<Settings>();
    startupSettings = () => pending.promise;
    start();
    openSettings();
    target.querySelector<HTMLButtonElement>(".update-card button")!.click();
    await vi.waitFor(() => expect(calls("check_for_update")).toBe(1));
    pending.resolve({ ...get(settings), checkUpdatesOnLaunch: true });
    await vi.waitFor(() => expect(bridge.listen).toHaveBeenCalledWith("operation_failed", expect.any(Function)));
    expect(calls("check_for_update")).toBe(1);
  });

  it("does not start a check or subscriptions after destruction during startup", async () => {
    const pending = deferred<Settings>();
    startupSettings = () => pending.promise;
    start();
    await unmount(component!);
    component = null;
    pending.resolve({ ...get(settings), checkUpdatesOnLaunch: true });
    await new Promise(resolve => setTimeout(resolve, 20));
    expect(calls("check_for_update")).toBe(0);
    expect(bridge.listen).not.toHaveBeenCalled();
  });

  it("cleans an event subscription that finishes registering after destruction", async () => {
    const pending = deferred<() => void>();
    const cleanup = vi.fn();
    bridge.listen.mockImplementationOnce(() => pending.promise);
    start();
    await vi.waitFor(() => expect(bridge.listen).toHaveBeenCalledTimes(1));
    await unmount(component!);
    component = null;
    pending.resolve(cleanup);
    await vi.waitFor(() => expect(cleanup).toHaveBeenCalledTimes(1));
    expect(bridge.listen).toHaveBeenCalledTimes(1);
  });

  it("cleans all registered event listeners when the root is unmounted", async () => {
    start();
    await vi.waitFor(() => expect(bridge.listen).toHaveBeenCalledWith("operation_failed", expect.any(Function)));
    await unmount(component!);
    component = null;
    expect(cleanups.length).toBeGreaterThan(5);
    for (const cleanup of cleanups) expect(cleanup).toHaveBeenCalledTimes(1);
  });

  it("clears the root cache safely while a native check is still in flight", async () => {
    const pending = deferred<UpdateCheck>();
    settings.update(value => ({ ...value, checkUpdatesOnLaunch: true }));
    check = () => pending.promise;
    start();
    await vi.waitFor(() => expect(calls("check_for_update")).toBe(1));
    await unmount(component!);
    component = null;
    pending.resolve({ ...update, updateAvailable: true, latestVersion: "2.2.5" });
    await new Promise(resolve => setTimeout(resolve, 20));
    expect(target.querySelector('[role="dialog"]')).toBeNull();
    expect(target.querySelector(".notification-error")).toBeNull();
    expect(calls("check_for_update")).toBe(1);
  });
});
