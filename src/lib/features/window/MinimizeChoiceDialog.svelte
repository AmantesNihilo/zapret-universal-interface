<script lang="ts">
  import { ChevronDown, Minus, X } from "@lucide/svelte";
  import { t } from "$lib/stores/i18n";

  let {
    open = false,
    remember = false,
    onRememberChange,
    onChoose,
    onClose
  }: {
    open?: boolean;
    remember?: boolean;
    onRememberChange: (value: boolean) => void;
    onChoose: (behavior: "tray" | "taskbar") => void | Promise<void>;
    onClose: () => void;
  } = $props();
</script>

{#if open}
  <div class="about-overlay workflow-dialog-overlay" role="presentation">
    <div class="workflow-dialog minimize-choice-panel" role="dialog" aria-modal="true" aria-label={$t("minimize.title")}>
      <header class="dialog-header">
        <div>
          <h3>{$t("minimize.title")}</h3>
          <p>{$t("minimize.text")}</p>
        </div>
        <button class="icon-button" type="button" onclick={onClose} title={$t("common.close")}>
          <X size={18} />
        </button>
      </header>
      <div class="dialog-body minimize-choice-grid">
        <button type="button" onclick={() => onChoose("taskbar")}>
          <span class="minimize-choice-icon"><Minus size={20} /></span>
          <span>
            <strong>{$t("minimize.taskbar")}</strong>
            <small>{$t("minimize.taskbarHint")}</small>
          </span>
        </button>
        <button type="button" onclick={() => onChoose("tray")}>
          <span class="minimize-choice-icon"><ChevronDown size={20} /></span>
          <span>
            <strong>{$t("minimize.tray")}</strong>
            <small>{$t("minimize.trayHint")}</small>
          </span>
        </button>
      </div>
      <footer class="dialog-footer minimize-choice-footer">
        <label class="check-row">
          <input
            type="checkbox"
            checked={remember}
            onchange={(event) => onRememberChange(event.currentTarget.checked)}
          />
          {$t("minimize.dontAsk")}
        </label>
      </footer>
    </div>
  </div>
{/if}
