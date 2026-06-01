<script lang="ts">
  import { Power } from "@lucide/svelte";
  import type { AppStatus } from "$lib/api/types";
  import { t } from "$lib/stores/i18n";

  let {
    status,
    disabled = false,
    onclick
  }: { status: AppStatus; disabled?: boolean; onclick: () => void } = $props();
  const busy = $derived(status === "starting" || status === "stopping");
  const active = $derived(status === "on");
</script>

<button class:active class:busy class="power-button" type="button" disabled={disabled || busy} onclick={onclick} aria-label={$t("common.power")}>
  <Power size={38} strokeWidth={2.2} />
</button>
