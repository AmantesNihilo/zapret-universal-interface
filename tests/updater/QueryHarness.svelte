<script lang="ts">
  import { QueryClientProvider, type QueryClient } from "@tanstack/svelte-query";
  import type { commands } from "$lib/api/commands";
  import type { UpdateController } from "$lib/features/updater/updateController.svelte";
  import UpdateHarness from "./UpdateHarness.svelte";

  let { client, api, onReady, onError }: {
    client: QueryClient;
    api: Pick<typeof commands, "checkForUpdate" | "installUpdate" | "openUrl">;
    onReady: (updater: UpdateController) => void;
    onError: (message: string | null) => void;
  } = $props();
</script>

<QueryClientProvider {client}>
  <UpdateHarness {api} {onReady} {onError} />
</QueryClientProvider>
