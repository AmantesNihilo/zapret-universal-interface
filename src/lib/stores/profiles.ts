import { writable } from "svelte/store";
import { commands } from "$lib/api/commands";
import type { Profile, ProfilesFile } from "$lib/api/types";

export const profilesFile = writable<ProfilesFile>({
  activeProfileId: "default",
  profiles: []
});

export async function loadProfiles() {
  profilesFile.set(await commands.getProfiles());
}

export async function saveProfile(profile: Profile) {
  profilesFile.set(await commands.saveProfile(profile));
}

export async function setActiveProfile(profileId: string) {
  profilesFile.set(await commands.setActiveProfile(profileId));
}

export async function deleteProfile(profileId: string) {
  profilesFile.set(await commands.deleteProfile(profileId));
}
