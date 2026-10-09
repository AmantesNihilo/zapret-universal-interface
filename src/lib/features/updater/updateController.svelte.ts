import { get } from "svelte/store";
import { commands } from "$lib/api/commands";
import type { UpdateCheck } from "$lib/api/types";
import { t } from "$lib/stores/i18n";

export class UpdateController {
  info = $state<UpdateCheck | null>(null);
  open = $state(false);
  checking = $state(false);
  installing = $state(false);
  message = $state<string | null>(null);
  postponedVersion = $state<string | null>(null);

  constructor(private readonly onError: (message: string | null) => void) {}

  async checkOnLaunch() {
    try {
      const info = await commands.checkForUpdate();
      this.info = info;
      const version = info.latestVersion ?? "";
      this.open = info.updateAvailable && version !== this.postponedVersion;
    } catch {
      // Update checks should never block app startup.
    }
  }

  async checkManual() {
    this.checking = true;
    this.onError(null);
    try {
      const info = await commands.checkForUpdate();
      this.info = info;
      this.open = info.updateAvailable;
      this.message = info.updateAvailable
        ? null
        : get(t)("update.none", { version: info.currentVersion });
    } catch (caught) {
      this.message = null;
      this.onError(`${get(t)("update.failed")}: ${String(caught)}`);
    } finally {
      this.checking = false;
    }
  }

  async install() {
    this.installing = true;
    this.onError(null);
    try {
      await commands.installUpdate();
    } catch (caught) {
      this.installing = false;
      this.onError(`${get(t)("update.installFailed")}: ${String(caught)}`);
    }
  }

  async openRelease() {
    if (this.info?.releaseUrl) await commands.openUrl(this.info.releaseUrl);
  }

  async openPortableDownload() {
    const url = this.info?.portableAsset?.downloadUrl ?? this.info?.releaseUrl;
    if (url) await commands.openUrl(url);
  }

  postpone() {
    this.postponedVersion = this.info?.latestVersion ?? null;
    this.open = false;
  }
}
