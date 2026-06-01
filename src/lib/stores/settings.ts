import { writable } from "svelte/store";
import { commands } from "$lib/api/commands";
import type { Settings } from "$lib/api/types";

export const settings = writable<Settings>({
  theme: "dark",
  accent: "cyan",
  language: "ru",
  layoutOrientation: "portrait",
  launchMinimized: false,
  closeToTray: false,
  startWithWindows: false,
  autoStartActiveProfileOnLaunch: false,
  customPresetRoots: []
});

export async function loadSettings() {
  settings.set(await commands.getSettings());
}

export async function saveSettings(value: Settings) {
  settings.set(await commands.saveSettings(value));
}
