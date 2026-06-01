import { writable } from "svelte/store";
import { commands } from "$lib/api/commands";
import { onAppStateChanged } from "$lib/api/events";
import type { AppState } from "$lib/api/types";

export const appState = writable<AppState>({
  status: "off",
  activeProfileId: "default",
  zapret: { service: "zapret", state: "stopped" },
  tgWs: { service: "tg-ws", state: "stopped" },
  lastError: null
});

export async function loadAppState() {
  appState.set(await commands.getAppState());
}

export async function bindAppStateEvents() {
  return onAppStateChanged((state) => appState.set(state));
}
