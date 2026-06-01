import { writable } from "svelte/store";
import { commands } from "$lib/api/commands";
import { onLogLine } from "$lib/api/events";
import type { LogLine } from "$lib/api/types";

export const logs = writable<LogLine[]>([]);

export async function loadLogs() {
  logs.set(await commands.getLogs());
}

export async function bindLogEvents() {
  return onLogLine((line) => {
    logs.update((items) => [...items.slice(-199), line]);
  });
}

export async function clearLogView() {
  await commands.clearLogs();
  logs.set([]);
}
