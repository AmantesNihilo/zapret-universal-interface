<script lang="ts">
  import { onMount, untrack } from "svelte";
  import type { Profile } from "$lib/api/types";
  import { applyProfilePatch, type ProfilePatch } from "$lib/features/profiles/model";
  import TgWsSettings from "$lib/components/TgWsSettings.svelte";

  let { initialProfile, onUpdate, onReady }: {
    initialProfile: Profile;
    onUpdate: (patch: ProfilePatch) => void | Promise<void>;
    onReady: (controls: { setProfile: (profile: Profile) => void }) => void;
  } = $props();
  let profile = $state(untrack(() => initialProfile));
  onMount(() => { onReady({ setProfile: value => { profile = value; } }); });
  async function update(patch: ProfilePatch) {
    await onUpdate(patch);
    profile = applyProfilePatch(profile, patch);
  }
</script>
<TgWsSettings {profile} secretPreview="hidden" onUpdate={update} />
