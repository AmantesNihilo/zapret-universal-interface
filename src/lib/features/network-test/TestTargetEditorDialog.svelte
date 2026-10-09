<script lang="ts">
  import { Check, Plus, X } from "@lucide/svelte";
  import type { TestTargetConfig } from "$lib/api/types";
  import { t } from "$lib/stores/i18n";

  let {
    open = false,
    targets = [],
    customService = "",
    customName = "",
    customValue = "",
    onCustomServiceChange,
    onCustomNameChange,
    onCustomValueChange,
    onToggle,
    onRemove,
    onAdd,
    onReset,
    onSave,
    onClose
  }: {
    open?: boolean;
    targets?: TestTargetConfig[];
    customService?: string;
    customName?: string;
    customValue?: string;
    onCustomServiceChange: (value: string) => void;
    onCustomNameChange: (value: string) => void;
    onCustomValueChange: (value: string) => void;
    onToggle: (index: number) => void;
    onRemove: (index: number) => void;
    onAdd: () => void | Promise<void>;
    onReset: () => void | Promise<void>;
    onSave: () => void | Promise<void>;
    onClose: () => void;
  } = $props();
</script>

{#if open}
  <div class="about-overlay workflow-dialog-overlay" role="presentation" onclick={(event) => event.currentTarget === event.target && onClose()}>
    <div class="workflow-dialog target-editor-panel" role="dialog" aria-modal="true" aria-label={$t("test.editTargets")}>
      <header class="dialog-header">
        <div>
          <h3>{$t("test.editTargets")}</h3>
          <p>{$t("test.editTargetsHint")}</p>
        </div>
        <button class="icon-button" type="button" onclick={onClose} title={$t("common.close")}>
          <X size={18} />
        </button>
      </header>
      <div class="dialog-body target-dialog-body">
        <div class="target-editor-list">
          {#each targets as target, index}
            <article class:disabled={!target.enabled}>
              <button class="target-enable-button" type="button" onclick={() => onToggle(index)}>
                <span>{#if target.enabled}<Check size={13} />{/if}</span>
              </button>
              <div>
                <strong>{target.name}</strong>
                <small>{target.service} · {target.value}</small>
              </div>
              <button class="icon-button" type="button" title={$t("common.close")} onclick={() => onRemove(index)}>
                <X size={14} />
              </button>
            </article>
          {/each}
        </div>
        <div class="custom-target-form">
          <strong>{$t("test.addTarget")}</strong>
          <div class="form-row target-form-row">
            <label>
              {$t("test.service")}
              <input value={customService} oninput={(event) => onCustomServiceChange(event.currentTarget.value)} />
            </label>
            <label>
              {$t("test.targetName")}
              <input value={customName} oninput={(event) => onCustomNameChange(event.currentTarget.value)} />
            </label>
          </div>
          <label>
            URL / PING
            <input
              value={customValue}
              placeholder="https://example.com или PING:1.1.1.1"
              oninput={(event) => onCustomValueChange(event.currentTarget.value)}
            />
          </label>
          <button class="secondary-button" type="button" onclick={onAdd}>
            <Plus size={16} /> {$t("test.addTarget")}
          </button>
        </div>
      </div>
      <footer class="dialog-footer">
        <div class="modal-action-row">
          <button class="secondary-button" type="button" onclick={onReset}>
            {$t("test.resetTargets")}
          </button>
          <button class="primary-button" type="button" onclick={onSave}>
            {$t("common.save")}
          </button>
        </div>
      </footer>
    </div>
  </div>
{/if}
