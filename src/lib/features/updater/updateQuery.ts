import { queryOptions } from "@tanstack/svelte-query";
import { commands } from "$lib/api/commands";
import type { UpdateCheck } from "$lib/api/types";

export const UPDATE_QUERY_KEY = ["updater", "latest"] as const;
export const UPDATE_STALE_TIME = 5 * 60 * 1000;
export const UPDATE_GC_TIME = 10 * 60 * 1000;

export function updateQueryOptions(check: () => Promise<UpdateCheck> = commands.checkForUpdate) {
  return queryOptions({
    queryKey: UPDATE_QUERY_KEY,
    queryFn: async () => {
      try {
        return await check();
      } catch (caught) {
        // Tauri may reject with a string rather than an Error.
        throw caught instanceof Error ? caught : new Error(String(caught));
      }
    },
    staleTime: UPDATE_STALE_TIME,
    gcTime: UPDATE_GC_TIME,
    retry: false,
    refetchOnMount: false,
    refetchOnWindowFocus: false,
    refetchOnReconnect: false,
    refetchInterval: false,
    // The native HTTP client, not navigator.onLine in the WebView, decides availability.
    networkMode: "always"
  });
}
