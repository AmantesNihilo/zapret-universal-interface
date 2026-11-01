<script lang="ts">
  import { onMount } from "svelte";
  import type { commands } from "$lib/api/commands";
  import { UpdateController } from "$lib/features/updater/updateController.svelte";
  import GeneralSettings from "$lib/features/settings/GeneralSettings.svelte";
  import UpdateDialog from "$lib/features/updater/UpdateDialog.svelte";
  import AppNotifications from "$lib/features/notifications/AppNotifications.svelte";

  let { api, onReady, onError }: {
    api: Pick<typeof commands, "checkForUpdate" | "installUpdate" | "openUrl">;
    onReady: (updater: UpdateController) => void;
    onError: (message: string | null) => void;
  } = $props();
  let error = $state<string | null>(null);
  let panel = $state(true);
  // svelte-ignore state_referenced_locally - the transport belongs to this instance
  const updater = new UpdateController((message) => { error = message; onError(message); }, undefined, api);
  onMount(() => onReady(updater));
</script>

<button data-testid="toggle-panel" onclick={() => panel = !panel}>Toggle panel</button>
{#if panel}
  <GeneralSettings
    activeEngine={null}
    serviceConfigLocked={false}
    updateInfo={updater.info}
    updateChecking={updater.checking}
    updateError={updater.error}
    updateLastChecked={updater.lastChecked}
    updateManualChecked={updater.manualChecked}
    updateInstalling={updater.installing}
    onChooseEngine={() => {}}
    onUpdateSettings={() => {}}
    onUpdateWindowLayout={() => {}}
    onCheckUpdates={() => updater.checkManual()}
  />
{/if}
<UpdateDialog
  open={updater.open}
  info={updater.info}
  installing={updater.installing}
  onClose={() => updater.postpone()}
  onOpenRelease={() => updater.openRelease()}
  onOpenPortable={() => updater.openPortableDownload()}
  onInstall={() => updater.install()}
/>
<AppNotifications {error} onCloseError={() => error = null} onCloseAbort={() => {}} />
