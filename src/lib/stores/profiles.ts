import { writable } from "svelte/store";
import { commands } from "$lib/api/commands";
import type { Profile, ProfilesFile } from "$lib/api/types";

export const profilesFile = writable<ProfilesFile>({
  activeProfileId: "default",
  profiles: [
    {
      id: "default",
      name: "Default",
      zapretEnabled: false,
      zapretEngine: null,
      zapretPresetId: null,
      tgWsEnabled: false,
      tgWsHost: "127.0.0.1",
      tgWsPort: 1443,
      tgWsSecret: "",
      tgWsDcIps: ["2:149.154.167.220", "4:149.154.167.220"],
      tgWsCfProxyEnabled: true,
      tgWsCfCustomEnabled: false,
      tgWsDefaultDomains: true,
      tgWsCfDomains: [],
      tgWsCfWorkerEnabled: false,
      tgWsCfWorkerDomain: null,
      tgWsFrontingDomain: "sprinthost.ru",
      tgWsCfPriority: false,
      tgWsCfBalance: false,
      tgWsBufKb: 256,
      tgWsPoolSize: 4,
      tgWsVerbose: false,
      tgWsLogMaxMb: 5,
      tgWsForceTestDc: false,
      autostartOnAppLaunch: false,
      notes: null
    }
  ]
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
