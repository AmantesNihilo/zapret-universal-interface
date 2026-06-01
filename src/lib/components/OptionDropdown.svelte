<script lang="ts">
  import { onMount } from "svelte";
  import { Check, ChevronDown } from "@lucide/svelte";

  export type DropdownOption = {
    value: string;
    label: string;
    hint?: string;
    color?: string;
  };

  let {
    options,
    value,
    disabled = false,
    onChange
  }: {
    options: DropdownOption[];
    value: string;
    disabled?: boolean;
    onChange: (value: string) => void;
  } = $props();

  let open = $state(false);
  let root: HTMLDivElement;
  const selected = $derived(options.find((option) => option.value === value) ?? options[0]);

  onMount(() => {
    const closeOutside = (event: PointerEvent) => {
      if (!root?.contains(event.target as Node)) open = false;
    };
    const closeOnEscape = (event: KeyboardEvent) => {
      if (event.key === "Escape") open = false;
    };

    document.addEventListener("pointerdown", closeOutside);
    document.addEventListener("keydown", closeOnEscape);
    return () => {
      document.removeEventListener("pointerdown", closeOutside);
      document.removeEventListener("keydown", closeOnEscape);
    };
  });

  function choose(nextValue: string) {
    onChange(nextValue);
    open = false;
  }
</script>

<div class="option-dropdown" bind:this={root}>
  <button
    class:open
    class="option-dropdown-button"
    type="button"
    disabled={disabled}
    aria-expanded={open}
    onclick={() => (open = !open)}
  >
    {#if selected?.color}
      <span class="option-swatch" style={`--swatch: ${selected.color}`}></span>
    {/if}
    <span class="option-copy">
      <strong>{selected?.label}</strong>
      {#if selected?.hint}<small>{selected.hint}</small>{/if}
    </span>
    <ChevronDown class="option-chevron" size={17} />
  </button>

  {#if open}
    <div class="option-dropdown-menu">
      {#each options as option}
        <button class:active={option.value === value} type="button" onclick={() => choose(option.value)}>
          {#if option.color}
            <span class="option-swatch" style={`--swatch: ${option.color}`}></span>
          {/if}
          <span class="option-copy">
            <strong>{option.label}</strong>
            {#if option.hint}<small>{option.hint}</small>{/if}
          </span>
          {#if option.value === value}
            <Check size={15} />
          {/if}
        </button>
      {/each}
    </div>
  {/if}
</div>
