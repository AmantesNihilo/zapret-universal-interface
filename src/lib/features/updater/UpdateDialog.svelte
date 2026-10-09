<script lang="ts">
  import { Download, ExternalLink, X } from "@lucide/svelte";
  import type { UpdateCheck } from "$lib/api/types";
  import { t } from "$lib/stores/i18n";

  let {
    open = false,
    info = null,
    installing = false,
    onClose,
    onOpenRelease,
    onOpenPortable,
    onInstall
  }: {
    open?: boolean;
    info?: UpdateCheck | null;
    installing?: boolean;
    onClose: () => void;
    onOpenRelease: () => void | Promise<void>;
    onOpenPortable: () => void | Promise<void>;
    onInstall: () => void | Promise<void>;
  } = $props();

  function formatBytes(bytes?: number | null) {
    if (!bytes || bytes <= 0) return "";
    const units = ["B", "KB", "MB", "GB"];
    let size = bytes;
    let unit = 0;
    while (size >= 1024 && unit < units.length - 1) {
      size /= 1024;
      unit += 1;
    }
    return `${size.toFixed(unit === 0 ? 0 : 1)} ${units[unit]}`;
  }
</script>

{#if open && info}
  <div class="about-overlay" role="presentation" onclick={(event) => event.currentTarget === event.target && onClose()}>
    <div class="about-panel update-panel" role="dialog" aria-modal="true" aria-label={$t("update.available")}>
      <header>
        <div class="about-mark" aria-hidden="true">
          <img src="/zui-icon.png" alt="" />
        </div>
        <div>
          <h3>{$t("update.available")}</h3>
          <p>
            {$t("update.versionLine", {
              current: info.currentVersion,
              latest: info.latestVersion ?? ""
            })}
          </p>
        </div>
        <button class="icon-button" type="button" disabled={installing} onclick={onClose} title={$t("update.later")}>
          <X size={18} />
        </button>
      </header>

      <section class="update-summary">
        <div>
          <strong>{info.releaseName ?? `ZUI ${info.latestVersion ?? ""}`}</strong>
          <span>
            {#if info.distribution === "portable"}
              {$t("update.portableText")}
            {:else if info.distribution === "development"}
              {$t("update.developmentText")}
            {:else if info.canInstall}
              {$t("update.installedText")}
            {:else}
              {$t("update.noInstaller")}
            {/if}
          </span>
        </div>
        {#if info.installerAsset || info.portableAsset}
          <div class="update-assets">
            {#if info.installerAsset}
              <span>{info.installerAsset.name} {formatBytes(info.installerAsset.size)}</span>
            {/if}
            {#if info.portableAsset}
              <span>{info.portableAsset.name} {formatBytes(info.portableAsset.size)}</span>
            {/if}
          </div>
        {/if}
      </section>

      {#if info.releaseNotes}
        <pre class="update-notes">{info.releaseNotes}</pre>
      {/if}

      <div class="update-actions">
        <button class="secondary-button" type="button" disabled={installing} onclick={onClose}>
          {$t("update.later")}
        </button>
        <button
          class="secondary-button"
          type="button"
          disabled={installing || (!info.releaseUrl && !info.portableAsset)}
          onclick={info.distribution === "portable" ? onOpenPortable : onOpenRelease}
        >
          <ExternalLink size={16} /> {$t("update.openRelease")}
        </button>
        {#if info.canInstall}
          <button class="primary-button" type="button" disabled={installing} onclick={onInstall}>
            <Download size={16} /> {installing ? $t("update.installing") : $t("update.install")}
          </button>
        {/if}
      </div>
    </div>
  </div>
{/if}
