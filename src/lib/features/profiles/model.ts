import type { Profile } from "$lib/api/types";

/** Identity is selected by the owner, not edited by a settings form. */
export type ProfilePatch = Partial<Omit<Profile, "id">> & { id?: never };

export function applyProfilePatch(profile: Profile, patch: ProfilePatch): Profile {
  // Preserve identity at runtime too, including callers outside the TS contract.
  return { ...profile, ...patch, id: profile.id };
}
