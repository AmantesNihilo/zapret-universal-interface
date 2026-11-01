<script lang="ts">
  import { onDestroy } from "svelte";
  import { QueryClientProvider } from "@tanstack/svelte-query";
  import { createAppQueryClient } from "$lib/app/queryClient";
  import "../lib/styles/tokens.css";
  import "../lib/styles/global.css";

  let { children } = $props();
  const queryClient = createAppQueryClient();
  onDestroy(() => queryClient.clear());
</script>

<QueryClientProvider client={queryClient}>
  {@render children()}
</QueryClientProvider>
