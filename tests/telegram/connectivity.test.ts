import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { flushSync, mount, unmount } from "svelte";
import { get } from "svelte/store";
import type { Profile, TgWsConnectivityReport } from "$lib/api/types";
import { profilesFile } from "$lib/stores/profiles";
import { settings } from "$lib/stores/settings";
import { applyProfilePatch, type ProfilePatch } from "$lib/features/profiles/model";
import TgHarness from "./TgHarness.svelte";

const bridge = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke: bridge.invoke }));
let component: ReturnType<typeof mount> | undefined;
let target: HTMLDivElement;
const report: TgWsConnectivityReport = { kind: "cfProxy", allOk: true, probes: [] };

function profile(): Profile {
  const value = get(profilesFile).profiles[0];
  if (!value) throw new Error("Default profile missing");
  return { ...value, tgWsCfWorkerDomain: "worker.example.test" };
}

beforeEach(() => {
  bridge.invoke.mockReset().mockResolvedValue(report);
  settings.update(value => ({ ...value, language: "ru" }));
  target = document.createElement("div"); document.body.append(target);
});
afterEach(async () => {
  if (component) await unmount(component);
  component = undefined; target.remove();
});

function render() {
  const onUpdate = vi.fn<(patch: ProfilePatch) => void | Promise<void>>();
  let controls: { setProfile: (profile: Profile) => void } | undefined;
  component = mount(TgHarness, {
    target, props: { initialProfile: profile(), onUpdate, onReady: value => { controls = value; } }
  });
  flushSync();
  const buttons = () => {
    const result = Array.from(target.querySelectorAll<HTMLButtonElement>(".test-button"));
    const proxy = result[0]; const worker = result[1];
    if (!proxy || !worker) throw new Error("Connectivity buttons missing");
    return { proxy, worker };
  };
  return { onUpdate, buttons, setProfile: (value: Profile) => {
    if (!controls) throw new Error("Controls missing");
    controls.setProfile(value); flushSync();
  } };
}

describe("TG connectivity state and real settings form", () => {
  it("keeps profile identity even if a runtime caller attempts to override it", () => {
    const source = profile();
    const external: ProfilePatch = { tgWsPort: 8080 };
    Object.defineProperty(external, "id", { value: "other", enumerable: true });
    expect(applyProfilePatch(source, external).id).toBe(source.id);
  });
  it("shows one loading operation, blocks both buttons, then renders success", async () => {
    let finish: ((value: TgWsConnectivityReport) => void) | undefined;
    bridge.invoke.mockReturnValue(new Promise<TgWsConnectivityReport>(resolve => { finish = resolve; }));
    const h = render();
    h.buttons().proxy.click(); flushSync();
    expect(h.buttons().proxy.disabled).toBe(true);
    expect(h.buttons().worker.disabled).toBe(true);
    expect(bridge.invoke).toHaveBeenCalledTimes(1);
    finish?.(report);
    await vi.waitFor(() => expect(target.querySelector(".test-report")).not.toBeNull());
    expect(h.buttons().proxy.disabled).toBe(false);
    expect(target.querySelector(".test-report.error-result")).toBeNull();
  });
  it("shows a native error and allows retry", async () => {
    bridge.invoke.mockRejectedValueOnce("offline").mockResolvedValueOnce(report);
    const h = render();
    h.buttons().proxy.click();
    await vi.waitFor(() => expect(target.textContent).toContain("offline"));
    h.buttons().proxy.click();
    await vi.waitFor(() => expect(target.querySelector(".test-report.error-result")).toBeNull());
    expect(bridge.invoke).toHaveBeenCalledTimes(2);
  });
  it("rejects malformed and mismatched native reports", async () => {
    bridge.invoke.mockResolvedValueOnce({ ...report, probes: "bad" })
      .mockResolvedValueOnce({ ...report, kind: "cfWorker" });
    const h = render();
    h.buttons().proxy.click();
    await vi.waitFor(() => expect(target.textContent).toContain("connectivity.probes"));
    h.buttons().proxy.click();
    await vi.waitFor(() => expect(target.textContent).toContain("другому типу"));
  });
  it("ignores a late response after the profile changes", async () => {
    let finish: ((value: TgWsConnectivityReport) => void) | undefined;
    bridge.invoke.mockReturnValue(new Promise<TgWsConnectivityReport>(resolve => { finish = resolve; }));
    const h = render();
    h.buttons().proxy.click(); flushSync();
    h.setProfile({ ...profile(), id: "another-profile" });
    finish?.(report);
    await Promise.resolve(); await Promise.resolve(); flushSync();
    expect(target.querySelector(".test-report")).toBeNull();
    expect(h.buttons().proxy.disabled).toBe(false);
  });
  it("validates the actual port field and resets its draft on profile switch", async () => {
    const h = render();
    const port = target.querySelector<HTMLInputElement>('input[type="number"]');
    if (!port) throw new Error("Port input missing");
    port.value = "0"; port.dispatchEvent(new Event("input", { bubbles: true }));
    port.dispatchEvent(new FocusEvent("blur")); flushSync();
    expect(h.onUpdate).not.toHaveBeenCalled();
    expect(port.getAttribute("aria-invalid")).toBe("true");
    h.setProfile({ ...profile(), id: "another-profile", tgWsPort: 8888 });
    expect(port.value).toBe("8888");
    expect(target.querySelector('[role="alert"]')).toBeNull();
  });
});
