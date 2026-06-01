<script lang="ts">
  import { ChevronRight, FileText, Send, Settings2, Shield } from "@lucide/svelte";
  import StatusBadge from "./StatusBadge.svelte";
  import type { ServiceStatus } from "$lib/api/types";
  import { t } from "$lib/stores/i18n";

  let {
    kind = "zapret",
    title,
    subtitle,
    enabled,
    status,
    actionLabel,
    extraActionLabel = null,
    extraDisabled = false,
    disabled = false,
    onToggle,
    onAction,
    onExtraAction
  }: {
    kind?: "zapret" | "tg-ws";
    title: string;
    subtitle: string;
    enabled: boolean;
    status: ServiceStatus;
    actionLabel: string;
    extraActionLabel?: string | null;
    extraDisabled?: boolean;
    disabled?: boolean;
    onToggle: (checked: boolean) => void;
    onAction: () => void;
    onExtraAction?: (() => void) | null;
  } = $props();

  const running = $derived(status.state === "running");
</script>

<section class:enabled class:running class="service-row">
  <button
    class="service-icon"
    class:telegram={kind === "tg-ws"}
    type="button"
    disabled={disabled}
    onclick={() => onToggle(!enabled)}
    title={enabled ? $t("service.disable", { name: title }) : $t("service.enable", { name: title })}
  >
    {#if kind === "tg-ws"}
      <Send size={29} />
    {:else}
      <Shield size={29} />
    {/if}
  </button>

  <div class="service-card-head">
    <div class="service-main">
      <div class="service-title">{title}</div>
      <div class="service-subtitle">{subtitle}</div>
    </div>
    <StatusBadge state={status.state} />
    <button class="service-chevron" type="button" disabled={disabled} onclick={onAction} title={actionLabel}>
      <ChevronRight size={22} />
    </button>
  </div>

  <div class="service-actions">
    <button class="service-action" type="button" disabled={disabled} onclick={onAction} title={actionLabel}>
      <Settings2 size={17} />
      <span>{actionLabel}</span>
    </button>
    <span class="service-divider"></span>
    {#if extraActionLabel}
      <button
        class="service-action accent-action"
        type="button"
        disabled={extraDisabled || !onExtraAction}
        onclick={() => onExtraAction?.()}
        title={extraActionLabel}
      >
        {#if kind === "tg-ws"}
          <Send size={17} />
        {:else}
          <FileText size={17} />
        {/if}
        <span>{extraActionLabel}</span>
      </button>
    {:else}
      <button class="service-action" type="button" disabled>
        <FileText size={17} />
        <span>{$t("common.logs")}</span>
      </button>
    {/if}
  </div>
</section>
