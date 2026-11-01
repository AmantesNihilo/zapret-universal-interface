import { onDestroy } from "svelte";
import { get } from "svelte/store";
import { createQuery, useQueryClient, type QueryClient } from "@tanstack/svelte-query";
import { commands } from "$lib/api/commands";
import { t } from "$lib/stores/i18n";
import { updateQueryOptions } from "./updateQuery";

type UpdateCommands = Pick<typeof commands, "checkForUpdate" | "installUpdate" | "openUrl">;
type CheckCycle = { manual: boolean; promise: Promise<void> };

export class UpdateController {
  open = $state(false);
  installing = $state(false);
  manualChecked = $state(false);
  postponedVersion = $state<string | null>(null);

  private readonly query;
  private readonly options;
  private cycle: CheckCycle | null = null;
  private startupRequested = false;
  private manualRequested = false;
  private disposed = false;

  constructor(
    private readonly onError: (message: string | null) => void,
    private readonly client: QueryClient = useQueryClient(),
    private readonly api: UpdateCommands = commands
  ) {
    this.options = updateQueryOptions(() => this.api.checkForUpdate());
    // Observing must never opt the user into an automatic check.
    this.query = createQuery(() => ({ ...this.options, enabled: false }), () => this.client);
    onDestroy(() => this.dispose());
  }

  get info() {
    return this.query.data ?? null;
  }

  get checking() {
    return this.query.isFetching;
  }

  get error() {
    return this.query.error?.message ?? null;
  }

  get lastChecked() {
    return this.query.dataUpdatedAt;
  }

  checkOnLaunch(): Promise<void> {
    if (this.disposed || this.installing || this.startupRequested || this.manualRequested) {
      return Promise.resolve();
    }
    this.startupRequested = true;
    return this.check(false);
  }

  checkManual(): Promise<void> {
    if (this.disposed || this.installing) return Promise.resolve();
    this.manualRequested = true;
    this.onError(null);
    if (this.cycle) {
      // Joining an automatic check gives its single completion manual semantics.
      this.cycle.manual = true;
      return this.cycle.promise;
    }
    return this.check(true);
  }

  private check(manual: boolean): Promise<void> {
    const cycle: CheckCycle = { manual, promise: Promise.resolve() };
    this.cycle = cycle;
    this.manualChecked = false;
    const request = this.client.fetchQuery({
      ...this.options,
      // A deliberate click asks for fresh data, but joins a query already in flight.
      staleTime: manual ? 0 : this.options.staleTime
    });
    cycle.promise = request.then(
      (info) => {
        if (this.disposed) return;
        this.manualChecked = cycle.manual;
        this.open = info.updateAvailable &&
          (cycle.manual || (info.latestVersion ?? "") !== this.postponedVersion);
      },
      (caught: unknown) => {
        if (this.disposed) return;
        // Cached data can remain visible, but must not reopen a stale dialog.
        this.open = false;
        this.manualChecked = cycle.manual;
        if (cycle.manual) {
          this.onError(`${get(t)("update.failed")}: ${String(caught instanceof Error ? caught.message : caught)}`);
        }
      }
    ).finally(() => {
      if (this.cycle === cycle) this.cycle = null;
    });
    return cycle.promise;
  }

  async install() {
    if (this.disposed || this.installing || this.cycle || this.checking || !this.info?.canInstall) return;
    this.installing = true;
    this.onError(null);
    try {
      await this.api.installUpdate();
      // Success hands control to the installer and exits ZUI.
    } catch (caught) {
      if (this.disposed) return;
      this.installing = false;
      this.onError(`${get(t)("update.installFailed")}: ${String(caught)}`);
    }
  }

  async openRelease() {
    if (!this.disposed && !this.installing && this.info?.releaseUrl) {
      await this.api.openUrl(this.info.releaseUrl);
    }
  }

  async openPortableDownload() {
    if (this.disposed || this.installing) return;
    const url = this.info?.portableAsset?.downloadUrl ?? this.info?.releaseUrl;
    if (url) await this.api.openUrl(url);
  }

  postpone() {
    if (this.disposed || this.installing) return;
    this.postponedVersion = this.info?.latestVersion ?? null;
    this.open = false;
  }

  dispose() {
    // The adapter owns observer cleanup. Native IPC cannot be aborted here.
    this.disposed = true;
  }
}
