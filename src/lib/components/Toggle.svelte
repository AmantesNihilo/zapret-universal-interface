<script lang="ts">
  let { checked = false, label = "", disabled = false, onchange }: {
    checked: boolean;
    label?: string;
    disabled?: boolean;
    onchange?: (checked: boolean) => void;
  } = $props();
</script>

<label class:disabled class="toggle">
  <input
    type="checkbox"
    checked={checked}
    disabled={disabled}
    onchange={(event) => onchange?.((event.currentTarget as HTMLInputElement).checked)}
  />
  <span aria-hidden="true"></span>
  {#if label}<strong>{label}</strong>{/if}
</label>
