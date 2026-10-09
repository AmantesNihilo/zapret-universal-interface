<script lang="ts">
  import { Shield, X } from "@lucide/svelte";
  import { commands } from "$lib/api/commands";
  import { t } from "$lib/stores/i18n";

  let {
    open = false,
    appVersion,
    onClose
  }: {
    open?: boolean;
    appVersion: string;
    onClose: () => void;
  } = $props();
</script>

{#if open}
  <div class="about-overlay" role="presentation" onclick={(event) => event.currentTarget === event.target && onClose()}>
    <div class="about-panel about-app-panel" role="dialog" aria-modal="true" aria-label={$t("about.title")}>
      <header class="about-hero">
        <div class="about-brand">
          <div class="about-mark" aria-hidden="true">
            <img src="/zui-icon.png" alt="" />
          </div>
          <div>
            <div class="about-title-line">
              <h3>ZUI</h3>
              <span>v{appVersion}</span>
            </div>
            <p>{$t("about.tagline")}</p>
          </div>
        </div>
        <button class="icon-button" type="button" onclick={onClose} title={$t("common.close")}>
          <X size={18} />
        </button>
      </header>

      <section class="about-section">
        <div class="about-section-heading">
          <div>
            <span class="about-eyebrow">{$t("about.developer")}</span>
            <h4>amantesnihilo</h4>
          </div>
        </div>
        <div class="about-developer-line">
          <button type="button" onclick={() => commands.openUrl("https://github.com/AmantesNihilo")}>
            github.com/AmantesNihilo
          </button>
          <span>{$t("about.developerRole")}</span>
        </div>
      </section>

      <section class="about-section">
        <div class="about-section-heading">
          <div>
            <span class="about-eyebrow">Open source</span>
            <h4>{$t("about.usedProjects")}</h4>
          </div>
        </div>
        <div class="credit-list">
          <div>
            <button type="button" onclick={() => commands.openUrl("https://github.com/bol-van/zapret")}>bol-van/zapret</button>
            <span>{$t("about.zapretRole")}</span>
          </div>
          <div>
            <button type="button" onclick={() => commands.openUrl("https://github.com/bol-van/zapret2")}>bol-van/zapret2</button>
            <span>{$t("about.zapret2Role")}</span>
          </div>
          <div>
            <button type="button" onclick={() => commands.openUrl("https://github.com/Flowseal/tg-ws-proxy")}>Flowseal/tg-ws-proxy</button>
            <span>{$t("about.tgProxyRole")}</span>
          </div>
          <div>
            <button type="button" onclick={() => commands.openUrl("https://github.com/Flowseal/zapret-discord-youtube")}>Flowseal/zapret-discord-youtube</button>
            <span>{$t("about.flowsealRole")}</span>
          </div>
          <div>
            <button type="button" onclick={() => commands.openUrl("https://github.com/hyperion-cs/dpi-checkers")}>hyperion-cs/dpi-checkers</button>
            <span>{$t("about.testSuiteRole")}</span>
          </div>
        </div>
      </section>

      <div class="about-note">
        <Shield size={17} />
        <p>{$t("about.openSourceNote")}</p>
      </div>
    </div>
  </div>
{/if}
