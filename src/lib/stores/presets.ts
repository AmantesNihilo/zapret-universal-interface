import { writable } from "svelte/store";
import { commands } from "$lib/api/commands";
import type { Preset } from "$lib/api/types";

export const presets = writable<Preset[]>([]);

export async function loadPresets() {
  presets.set(await commands.listPresets());
}

export async function setPresetFavorite(presetId: string, favorite: boolean) {
  presets.set(await commands.setPresetFavorite(presetId, favorite));
}

export async function setPresetHidden(presetId: string, hidden: boolean) {
  presets.set(await commands.setPresetHidden(presetId, hidden));
}
