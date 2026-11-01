import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { flushSync, mount, unmount } from "svelte";
import { focusManager, onlineManager, QueryClient } from "@tanstack/svelte-query";
import type { UpdateCheck } from "$lib/api/types";
import { settings } from "$lib/stores/settings";
import { createAppQueryClient } from "$lib/app/queryClient";
import { updateQueryOptions, UPDATE_GC_TIME, UPDATE_QUERY_KEY, UPDATE_STALE_TIME } from "$lib/features/updater/updateQuery";
import type { UpdateController } from "$lib/features/updater/updateController.svelte";
import QueryHarness from "./QueryHarness.svelte";

function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (reason: unknown) => void;
  const promise = new Promise<T>((ok, fail) => { resolve = ok; reject = fail; });
  return { promise, resolve, reject };
}

function release(patch: Partial<UpdateCheck> = {}): UpdateCheck {
  return {
    currentVersion: "2.2.4", latestVersion: "2.2.5", updateAvailable: true,
    distribution: "installed", canInstall: true, releaseName: "ZUI test release",
    releaseUrl: "https://github.com/AmantesNihilo/zapret-universal-interface/releases/tag/v2.2.5",
    ...patch
  };
}

const mounted: { component: ReturnType<typeof mount>; target: HTMLElement; client: QueryClient }[] = [];

async function render(client = createAppQueryClient()) {
  const api = {
    checkForUpdate: vi.fn<() => Promise<UpdateCheck>>().mockResolvedValue(release()),
    installUpdate: vi.fn<() => Promise<void>>().mockResolvedValue(),
    openUrl: vi.fn<(url: string) => Promise<void>>().mockResolvedValue()
  };
  const onError = vi.fn<(message: string | null) => void>();
  let updater!: UpdateController;
  const target = document.createElement("div");
  document.body.append(target);
  const component = mount(QueryHarness, {
    target, props: { client, api, onError, onReady: (value) => { updater = value; } }
  });
  mounted.push({ component, target, client });
  flushSync();
  await vi.waitFor(() => expect(updater).toBeDefined());
  const card = () => target.querySelector(".update-card")!;
  const button = () => card().querySelector("button")!;
  return { api, client, onError, target, component, updater, card, button };
}

beforeEach(() => {
  settings.update(value => ({ ...value, language: "ru" }));
});

afterEach(async () => {
  vi.useRealTimers();
  for (const { component, target, client } of mounted.splice(0)) {
    await unmount(component);
    client.clear();
    target.remove();
  }
  onlineManager.setOnline(true);
  focusManager.setFocused(undefined);
});

describe("updater query and real Svelte presentation", () => {
  it("mounts idle without opting into a check or showing a spinner", async () => {
    const h = await render();
    expect(h.api.checkForUpdate).not.toHaveBeenCalled();
    expect(h.button().disabled).toBe(false);
    expect(h.card().textContent).toContain("Проверьте релизы");
    expect(h.updater.info).toBeNull();
    expect(h.updater.checking).toBe(false);
  });

  it("shows loading, disables the button and renders the one cached result", async () => {
    const h = await render();
    const pending = deferred<UpdateCheck>();
    h.api.checkForUpdate.mockReturnValueOnce(pending.promise);
    h.button().click();
    await vi.waitFor(() => expect(h.button().disabled).toBe(true));
    expect(h.card().textContent).toContain("Проверяем");
    pending.resolve(release());
    await vi.waitFor(() => expect(h.target.querySelector('[role="dialog"]')).not.toBeNull());
    expect(h.card().textContent).toContain("2.2.5");
    expect(h.updater.info).toBe(h.client.getQueryData(UPDATE_QUERY_KEY));
    expect(h.client.getQueryCache().getAll()).toHaveLength(1);
    expect(h.button().disabled).toBe(false);
  });

  it("joins auto and manual requests, with one completion and one error notification", async () => {
    const h = await render();
    const pending = deferred<UpdateCheck>();
    h.api.checkForUpdate.mockReturnValueOnce(pending.promise);
    const auto = h.updater.checkOnLaunch();
    const manual = h.updater.checkManual();
    const twice = h.updater.checkManual();
    expect(manual).toBe(auto);
    expect(twice).toBe(auto);
    expect(h.api.checkForUpdate).toHaveBeenCalledTimes(1);
    pending.reject("network offline");
    await auto;
    await vi.waitFor(() => expect(h.updater.error).toBe("network offline"));
    expect(h.onError.mock.calls.filter(([message]) => message !== null)).toHaveLength(1);
    expect(h.button().disabled).toBe(false);
    expect(h.target.querySelector(".notification-error")?.textContent).toContain("network offline");
  });

  it("does not follow an earlier manual check with a late automatic check", async () => {
    const h = await render();
    await h.updater.checkManual();
    await h.updater.checkOnLaunch();
    expect(h.api.checkForUpdate).toHaveBeenCalledTimes(1);
  });

  it("runs auto once, keeps its failure quiet and allows a successful manual retry", async () => {
    const h = await render();
    h.api.checkForUpdate.mockRejectedValueOnce(new Error("auto failed"));
    await h.updater.checkOnLaunch();
    await h.updater.checkOnLaunch();
    await vi.waitFor(() => expect(h.card().textContent).toContain("auto failed"));
    expect(h.onError).not.toHaveBeenCalled();
    expect(h.target.querySelector(".notification-error")).toBeNull();
    expect(h.api.checkForUpdate).toHaveBeenCalledTimes(1);
    await h.updater.checkManual();
    await vi.waitFor(() => expect(h.updater.open).toBe(true));
    expect(h.api.checkForUpdate).toHaveBeenCalledTimes(2);
  });

  it("shows a successful no-update message and translates it without another request", async () => {
    const h = await render();
    h.api.checkForUpdate.mockResolvedValueOnce(release({ updateAvailable: false, canInstall: false }));
    await h.updater.checkManual();
    await vi.waitFor(() => expect(h.card().textContent).toContain("Обновлений нет"));
    expect(h.updater.open).toBe(false);
    settings.update(value => ({ ...value, language: "en" }));
    await vi.waitFor(() => expect(h.card().textContent).toContain("No updates found"));
    expect(h.api.checkForUpdate).toHaveBeenCalledTimes(1);
  });

  it("keeps old data after refetch failure without claiming the last check succeeded", async () => {
    const h = await render();
    await h.updater.checkManual();
    await vi.waitFor(() => expect(h.updater.info?.latestVersion).toBe("2.2.5"));
    const old = h.updater.info;
    h.api.checkForUpdate.mockRejectedValueOnce("latest request failed");
    await h.updater.checkManual();
    await vi.waitFor(() => expect(h.card().textContent).toContain("latest request failed"));
    expect(h.updater.info).toBe(old);
    expect(h.card().textContent).toContain("Последняя успешная проверка");
    expect(h.card().textContent).not.toContain("Доступна версия");
    expect(h.target.querySelector('[role="dialog"]')).toBeNull();
    expect(h.button().disabled).toBe(false);
  });

  it("honours postponement and only reopens the same version on an explicit check", async () => {
    const h = await render();
    await h.updater.checkOnLaunch();
    await vi.waitFor(() => expect(h.updater.open).toBe(true));
    h.updater.postpone();
    flushSync();
    expect(h.target.querySelector('[role="dialog"]')).toBeNull();
    h.client.setQueryData(UPDATE_QUERY_KEY, release());
    flushSync();
    expect(h.updater.open).toBe(false);
    await h.updater.checkOnLaunch();
    expect(h.api.checkForUpdate).toHaveBeenCalledTimes(1);
    await h.updater.checkManual();
    await vi.waitFor(() => expect(h.updater.open).toBe(true));
    expect(h.api.checkForUpdate).toHaveBeenCalledTimes(2);
  });

  it("does not refetch on focus/reconnect, language change, or panel remount", async () => {
    const h = await render();
    await h.updater.checkManual();
    h.updater.postpone();
    for (let i = 0; i < 3; i++) {
      focusManager.setFocused(false);
      focusManager.setFocused(true);
      onlineManager.setOnline(false);
      onlineManager.setOnline(true);
      h.target.querySelector<HTMLButtonElement>('[data-testid="toggle-panel"]')!.click();
      flushSync();
      h.target.querySelector<HTMLButtonElement>('[data-testid="toggle-panel"]')!.click();
      flushSync();
    }
    await new Promise(resolve => setTimeout(resolve, 30));
    expect(h.api.checkForUpdate).toHaveBeenCalledTimes(1);
    expect(h.client.getQueryCache().getAll()).toHaveLength(1);
    expect(h.updater.open).toBe(false);
  });

  it("lets native IPC report offline instead of pausing on navigator state", async () => {
    const h = await render();
    onlineManager.setOnline(false);
    h.api.checkForUpdate.mockRejectedValueOnce("native offline");
    await h.updater.checkManual();
    await vi.waitFor(() => expect(h.updater.error).toBe("native offline"));
    expect(h.api.checkForUpdate).toHaveBeenCalledTimes(1);
    expect(h.updater.checking).toBe(false);
  });

  it("uses fresh cache for automatic access but always refreshes deliberate clicks", async () => {
    const client = createAppQueryClient();
    client.setQueryData(UPDATE_QUERY_KEY, release({ updateAvailable: false, canInstall: false }));
    const h = await render(client);
    await h.updater.checkOnLaunch();
    expect(h.api.checkForUpdate).not.toHaveBeenCalled();
    await h.updater.checkManual();
    expect(h.api.checkForUpdate).toHaveBeenCalledTimes(1);
    await h.updater.checkManual();
    expect(h.api.checkForUpdate).toHaveBeenCalledTimes(2);
    expect(client.getQueryCache().getAll()).toHaveLength(1);
  });

  it("refreshes an expired entry on automatic access", async () => {
    const client = createAppQueryClient();
    client.setQueryData(UPDATE_QUERY_KEY, release(), { updatedAt: Date.now() - UPDATE_STALE_TIME - 1 });
    const h = await render(client);
    await h.updater.checkOnLaunch();
    expect(h.api.checkForUpdate).toHaveBeenCalledTimes(1);
  });

  it("does not send delayed notifications or reopen the dialog after owner destruction", async () => {
    const h = await render();
    const pending = deferred<UpdateCheck>();
    h.api.checkForUpdate.mockReturnValueOnce(pending.promise);
    const request = h.updater.checkManual();
    await unmount(h.component);
    mounted.splice(mounted.findIndex(row => row.component === h.component), 1);
    expect(h.client.getQueryCache().find({ queryKey: UPDATE_QUERY_KEY })?.getObserversCount()).toBe(0);
    pending.reject("late error");
    await request;
    expect(h.onError.mock.calls.filter(([message]) => message !== null)).toHaveLength(0);
    expect(h.updater.open).toBe(false);
    await h.updater.checkManual();
    expect(h.api.checkForUpdate).toHaveBeenCalledTimes(1);
    h.client.clear();
    h.target.remove();
  });

  it("blocks duplicate installation and checks, then unlocks after installation failure", async () => {
    const h = await render();
    await h.updater.checkManual();
    await vi.waitFor(() => expect(h.updater.info?.canInstall).toBe(true));
    const pending = deferred<void>();
    h.api.installUpdate.mockReturnValueOnce(pending.promise);
    const install = h.updater.install();
    await h.updater.install();
    await h.updater.checkManual();
    h.updater.postpone();
    flushSync();
    expect(h.api.installUpdate).toHaveBeenCalledTimes(1);
    expect(h.api.checkForUpdate).toHaveBeenCalledTimes(1);
    expect(h.button().disabled).toBe(true);
    expect(h.updater.open).toBe(true);
    pending.reject("installer failed");
    await install;
    await vi.waitFor(() => expect(h.button().disabled).toBe(false));
    expect(h.target.querySelector(".notification-error")?.textContent).toContain("installer failed");
  });

  it("does not install stale data during an in-flight check", async () => {
    const h = await render();
    await h.updater.checkManual();
    const pending = deferred<UpdateCheck>();
    h.api.checkForUpdate.mockReturnValueOnce(pending.promise);
    const request = h.updater.checkManual();
    await h.updater.install();
    expect(h.api.installUpdate).not.toHaveBeenCalled();
    pending.resolve(release());
    await request;
  });

  it("keeps one cache entry and one observer across 100 manual checks", async () => {
    const h = await render();
    for (let i = 0; i < 100; i++) {
      await h.updater.checkManual();
    }
    await vi.waitFor(() => expect(h.updater.checking).toBe(false));
    expect(h.api.checkForUpdate).toHaveBeenCalledTimes(100);
    expect(h.client.getQueryCache().getAll()).toHaveLength(1);
    expect(h.client.getQueryCache().find({ queryKey: UPDATE_QUERY_KEY })?.getObserversCount()).toBe(1);
  });

  it("keeps installation locked on success while control passes to the installer", async () => {
    const h = await render();
    await h.updater.checkManual();
    await vi.waitFor(() => expect(h.updater.info?.canInstall).toBe(true));
    await h.updater.install();
    await h.updater.install();
    await h.updater.checkManual();
    expect(h.api.installUpdate).toHaveBeenCalledTimes(1);
    expect(h.api.checkForUpdate).toHaveBeenCalledTimes(1);
    expect(h.updater.installing).toBe(true);
  });

  it.each(["portable", "development"])("keeps %s navigation and never installs it", async distribution => {
    const h = await render();
    const portableAsset = { name: "portable.zip", downloadUrl: "https://github.com/example/portable.zip", size: 2048, kind: "portable" };
    h.api.checkForUpdate.mockResolvedValueOnce(release({ distribution, canInstall: false, portableAsset }));
    await h.updater.checkManual();
    await vi.waitFor(() => expect(h.updater.info?.distribution).toBe(distribution));
    await h.updater.install();
    expect(h.api.installUpdate).not.toHaveBeenCalled();
    await h.updater.openPortableDownload();
    expect(h.api.openUrl).toHaveBeenCalledWith(portableAsset.downloadUrl);
    await h.updater.openRelease();
    expect(h.api.openUrl).toHaveBeenLastCalledWith(h.updater.info?.releaseUrl);
  });
});

describe("query cache policy", () => {
  it("deduplicates concurrent callers with the same key", async () => {
    const client = createAppQueryClient();
    const pending = deferred<UpdateCheck>();
    const check = vi.fn(() => pending.promise);
    const options = updateQueryOptions(check);
    const a = client.fetchQuery(options);
    const b = client.fetchQuery(options);
    expect(check).toHaveBeenCalledTimes(1);
    pending.resolve(release());
    expect(await a).toEqual(await b);
    client.clear();
  });

  it("collects inactive entries, without pretending to cancel the native request", async () => {
    vi.useFakeTimers();
    const client = createAppQueryClient();
    const options = updateQueryOptions(async () => release());
    await client.fetchQuery(options);
    await vi.advanceTimersByTimeAsync(UPDATE_GC_TIME + 1);
    expect(client.getQueryCache().getAll()).toHaveLength(0);
    client.clear();
  });
});
