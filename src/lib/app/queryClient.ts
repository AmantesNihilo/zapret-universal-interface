import { QueryClient } from "@tanstack/svelte-query";

/** One client per mounted application root; feature policies live with their queries. */
export function createAppQueryClient() {
  return new QueryClient();
}
