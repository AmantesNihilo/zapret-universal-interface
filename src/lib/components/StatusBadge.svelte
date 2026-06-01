<script lang="ts">
  import type { AppStatus, ServiceState } from "$lib/api/types";
  import { t } from "$lib/stores/i18n";

  let { state, testing = false }: { state: AppStatus | ServiceState; testing?: boolean } = $props();

  const label = $derived.by(() => {
    if (testing) return $t("common.testing");
    if (state === "on") return $t("common.on");
    if (state === "off") return $t("common.off");
    if (state === "running") return $t("common.running");
    if (state === "stopped") return $t("common.stopped");
    if (state === "starting") return $t("common.starting");
    if (state === "stopping") return $t("common.stopping");
    if (state === "error") return $t("common.error");
    return state;
  });
</script>

<span class={`status status-${testing ? "testing" : state}`}>{label}</span>
