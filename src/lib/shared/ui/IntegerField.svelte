<script lang="ts">
  import { onDestroy, untrack } from "svelte";
  import { t } from "$lib/stores/i18n";
  import { validateInteger } from "$lib/shared/lib/integer";

  let { value, min, max, label, disabled = false, resetKey, onCommit }: {
    value: number;
    min: number;
    max: number;
    label: string;
    disabled?: boolean;
    resetKey: string;
    onCommit: (value: number) => void | Promise<void>;
  } = $props();

  const id = $props.id();
  let draft = $state("");
  let touched = $state(false);
  let saving = $state(false);
  let saveError = $state<string | null>(null);
  let lastValue: number | undefined;
  let lastKey: string | undefined;
  let generation = 0;
  const validation = $derived(validateInteger(draft, min, max));
  const message = $derived(saveError ?? (touched && validation.status === "invalid"
    ? $t("form.integer." + validation.reason, { min, max }) : null));

  $effect(() => {
    const next = value;
    const key = resetKey;
    untrack(() => {
      if (lastKey !== key || lastValue !== next) {
        if (lastKey !== key) {
          generation++;
          saving = false;
        }
        lastKey = key;
        lastValue = next;
        draft = String(next);
        touched = false;
        saveError = null;
      }
    });
  });
  onDestroy(() => { generation++; });

  async function commit() {
    if (disabled || saving) return;
    touched = true;
    saveError = null;
    if (validation.status !== "valid") return;
    const next = validation.value;
    if (next === value) {
      draft = String(value);
      touched = false;
      return;
    }
    const current = generation;
    saving = true;
    try {
      await onCommit(next);
      if (current === generation) {
        draft = String(next);
        touched = false;
      }
    } catch (caught: unknown) {
      if (current === generation) {
        saveError = $t("form.saveFailed", {
          message: caught instanceof Error ? caught.message : String(caught)
        });
      }
    } finally {
      if (current === generation) saving = false;
    }
  }
</script>

<span class="integer-field">
  <input
    type="number"
    {min}
    {max}
    step="1"
    value={draft}
    disabled={disabled || saving}
    aria-label={label}
    aria-invalid={Boolean(message)}
    aria-describedby={message ? id + "-error" : undefined}
    oninput={(event) => { draft = event.currentTarget.value; saveError = null; }}
    onblur={() => void commit()}
    onkeydown={(event) => {
      if (event.key === "Enter") {
        event.preventDefault();
        void commit();
      }
    }}
  />
  {#if message}<small id={id + "-error"} class="field-error" role="alert">{message}</small>{/if}
</span>

<style>
  .integer-field { display: grid; gap: 5px; min-width: 0; }
  input { min-width: 0; width: 100%; }
  input[aria-invalid="true"] { border-color: #ef6464; }
  .field-error { color: #ef6464; font-size: 12px; font-weight: 400; overflow-wrap: anywhere; }
</style>
