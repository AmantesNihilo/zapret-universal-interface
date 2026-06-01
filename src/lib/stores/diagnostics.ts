import { writable } from "svelte/store";
import { commands } from "$lib/api/commands";
import type { Diagnostics } from "$lib/api/types";

export const diagnostics = writable<Diagnostics | null>(null);

export async function loadDiagnostics() {
  diagnostics.set(await commands.getDiagnostics());
}
