<script lang="ts">
  import { Check, ChevronDown, FlaskConical, RotateCw, X } from "@lucide/svelte";
  import { commands } from "$lib/api/commands";
  import type { Profile, TgWsConnectivityReport } from "$lib/api/types";
  import { t } from "$lib/stores/i18n";

  let {
    profile,
    locked = false,
    secretPreview,
    onUpdate
  } = $props<{
    profile: Profile;
    locked?: boolean;
    secretPreview: string;
    onUpdate: (patch: Partial<Profile>) => void | Promise<void>;
  }>();

  let testKind = $state<"cfProxy" | "cfWorker" | null>(null);
  let testReport = $state<TgWsConnectivityReport | null>(null);
  let testError = $state<string | null>(null);

  function splitLines(value: string): string[] {
    return value
      .replace(/[;,]/g, "\n")
      .split(/\r?\n/)
      .map((item) => item.trim())
      .filter(Boolean);
  }

  function generateSecret() {
    const bytes = crypto.getRandomValues(new Uint8Array(16));
    onUpdate({ tgWsSecret: Array.from(bytes, (byte) => byte.toString(16).padStart(2, "0")).join("") });
  }

  async function runConnectivityTest(kind: "cfProxy" | "cfWorker") {
    testKind = kind;
    testReport = null;
    testError = null;
    try {
      testReport = kind === "cfProxy"
        ? await commands.testTgWsCfProxy(profile)
        : await commands.testTgWsCfWorker(profile);
    } catch (caught) {
      testError = String(caught);
    } finally {
      testKind = null;
    }
  }
</script>

<section class="tg-settings">
  <div class="section-title">
    <div>
      <h3>tg-ws</h3>
      <p>{$t("tg.description")}</p>
    </div>
    <label class="switch-row">
      <input
        type="checkbox"
        checked={profile.tgWsEnabled}
        disabled={locked}
        onchange={(event) => onUpdate({ tgWsEnabled: event.currentTarget.checked })}
      />
      {$t("tg.enablePower")}
    </label>
  </div>

  <fieldset disabled={locked}>
    <legend>{$t("tg.connection")}</legend>
    <div class="form-row connection-row">
      <label>
        {$t("tg.host")}
        <input
          value={profile.tgWsHost}
          oninput={(event) => onUpdate({ tgWsHost: event.currentTarget.value })}
        />
      </label>
      <label class="port-field">
        {$t("tg.port")}
        <input
          type="number"
          min="1"
          max="65535"
          value={profile.tgWsPort}
          oninput={(event) => onUpdate({ tgWsPort: Number(event.currentTarget.value) })}
        />
      </label>
    </div>
    <label>
      {$t("tg.secret")}
      <div class="input-action">
        <input
          value={profile.tgWsSecret}
          autocomplete="off"
          spellcheck="false"
          oninput={(event) => onUpdate({ tgWsSecret: event.currentTarget.value })}
        />
        <button type="button" class="icon-action" title={$t("tg.regenerateSecret")} onclick={generateSecret}>
          <RotateCw size={17} />
        </button>
      </div>
      <small class="field-hint">{$t("tg.secretLinkHint")}: <code>{secretPreview}</code></small>
    </label>
  </fieldset>

  <fieldset disabled={locked}>
    <legend>{$t("tg.dcRoutes")}</legend>
    <label>
      {$t("tg.dcRoutesHint")}
      <textarea
        rows="4"
        spellcheck="false"
        value={profile.tgWsDcIps.join("\n")}
        oninput={(event) => onUpdate({ tgWsDcIps: splitLines(event.currentTarget.value) })}
      ></textarea>
    </label>
  </fieldset>

  <fieldset>
    <legend>Cloudflare Proxy</legend>
    <div class="test-row">
      <label class="switch-row">
        <input
          type="checkbox"
          checked={profile.tgWsCfProxyEnabled}
          disabled={locked}
          onchange={(event) => onUpdate({ tgWsCfProxyEnabled: event.currentTarget.checked })}
        />
        {$t("tg.cfEnable")}
      </label>
      <button
        type="button"
        class="test-button"
        disabled={testKind !== null || (profile.tgWsCfCustomEnabled && profile.tgWsCfDomains.length === 0)}
        onclick={() => runConnectivityTest("cfProxy")}
      >
        {#if testKind === "cfProxy"}<span class="spin"><RotateCw size={16} /></span>{:else}<FlaskConical size={16} />{/if}
        {$t("tg.test")}
      </button>
    </div>
    <label class="switch-row secondary-switch">
      <input
        type="checkbox"
        checked={profile.tgWsCfCustomEnabled}
        disabled={locked || !profile.tgWsCfProxyEnabled}
        onchange={(event) => onUpdate({
          tgWsCfCustomEnabled: event.currentTarget.checked,
          tgWsDefaultDomains: !event.currentTarget.checked
        })}
      />
      {$t("tg.cfCustom")}
    </label>
    {#if profile.tgWsCfCustomEnabled}
      <label>
        {$t("tg.cfDomains")}
        <textarea
          rows="3"
          placeholder="example.com"
          disabled={locked || !profile.tgWsCfProxyEnabled}
          value={profile.tgWsCfDomains.join("\n")}
          oninput={(event) => onUpdate({ tgWsCfDomains: splitLines(event.currentTarget.value) })}
        ></textarea>
        <small class="field-hint">{$t("tg.cfDomainHint")}</small>
      </label>
    {:else}
      <p class="field-hint automatic-hint">{$t("tg.cfAutomaticHint")}</p>
    {/if}
  </fieldset>

  <fieldset>
    <legend>Cloudflare Worker</legend>
    <div class="test-row">
      <label class="switch-row">
        <input
          type="checkbox"
          checked={profile.tgWsCfWorkerEnabled}
          disabled={locked}
          onchange={(event) => onUpdate({ tgWsCfWorkerEnabled: event.currentTarget.checked })}
        />
        {$t("tg.workerEnable")}
      </label>
      <button
        type="button"
        class="test-button"
        disabled={testKind !== null || !(profile.tgWsCfWorkerDomain ?? "").trim()}
        onclick={() => runConnectivityTest("cfWorker")}
      >
        {#if testKind === "cfWorker"}<span class="spin"><RotateCw size={16} /></span>{:else}<FlaskConical size={16} />{/if}
        {$t("tg.test")}
      </button>
    </div>
    <label>
      {$t("tg.workerDomains")}
      <textarea
        rows="3"
        placeholder="one.workers.dev, two.workers.dev"
        disabled={locked}
        value={profile.tgWsCfWorkerDomain ?? ""}
        oninput={(event) => onUpdate({ tgWsCfWorkerDomain: event.currentTarget.value || null })}
      ></textarea>
      <small class="field-hint">{$t("tg.workerHint")}</small>
    </label>
  </fieldset>

  {#if testReport || testError}
    <div class:error-result={Boolean(testError || (testReport && !testReport.allOk))} class="test-report">
      <div class="report-title">
        {#if testError || !testReport?.allOk}<X size={17} />{:else}<Check size={17} />{/if}
        <strong>
          {testError
            ? $t("tg.testFailed")
            : testReport?.allOk
              ? $t("tg.testPassed")
              : $t("tg.testPartial")}
        </strong>
      </div>
      {#if testError}
        <p>{testError}</p>
      {:else if testReport}
        <p>{testReport.probes.filter((probe) => probe.ok).length}/{testReport.probes.length} {$t("tg.dcPassed")}</p>
        <div class="probe-grid">
          {#each testReport.probes as probe}
            <div class:failed={!probe.ok} class="probe-row" title={`${probe.target}\n${probe.detail}`}>
              <span>DC{probe.dc} · {probe.domain}</span>
              <strong>{probe.ok ? `${probe.latencyMs ?? 0} ms` : "FAIL"}</strong>
            </div>
          {/each}
        </div>
      {/if}
    </div>
  {/if}

  <details class="advanced-panel">
    <summary>
      <span><ChevronDown class="summary-chevron" size={16} /> {$t("tg.advancedRouting")}</span>
    </summary>
    <div class="advanced-content">
      <label>
        {$t("tg.frontingDomain")}
        <input
          value={profile.tgWsFrontingDomain ?? ""}
          placeholder="sprinthost.ru"
          disabled={locked}
          oninput={(event) => onUpdate({ tgWsFrontingDomain: event.currentTarget.value || null })}
        />
        <small class="field-hint">{$t("tg.frontingHint")}</small>
      </label>
      <div class="form-row">
        <label>
          {$t("tg.bufferKb")}
          <input
            type="number"
            min="4"
            max="16384"
            value={profile.tgWsBufKb}
            disabled={locked}
            oninput={(event) => onUpdate({ tgWsBufKb: Number(event.currentTarget.value) })}
          />
        </label>
        <label>
          {$t("tg.poolSize")}
          <input
            type="number"
            min="0"
            max="64"
            value={profile.tgWsPoolSize}
            disabled={locked}
            oninput={(event) => onUpdate({ tgWsPoolSize: Number(event.currentTarget.value) })}
          />
        </label>
      </div>
      <label>
        {$t("tg.logMaxMb")}
        <input
          type="number"
          min="1"
          max="1024"
          step="1"
          value={profile.tgWsLogMaxMb}
          disabled={locked}
          oninput={(event) => onUpdate({ tgWsLogMaxMb: Number(event.currentTarget.value) })}
        />
      </label>
      <label class="switch-row">
        <input
          type="checkbox"
          checked={profile.tgWsCfPriority}
          disabled={locked || (!profile.tgWsCfProxyEnabled && !profile.tgWsCfWorkerEnabled)}
          onchange={(event) => onUpdate({ tgWsCfPriority: event.currentTarget.checked })}
        />
        {$t("tg.cfPriority")}
      </label>
      <label class="switch-row">
        <input
          type="checkbox"
          checked={profile.tgWsCfBalance}
          disabled={locked || !profile.tgWsCfProxyEnabled}
          onchange={(event) => onUpdate({ tgWsCfBalance: event.currentTarget.checked })}
        />
        {$t("tg.cfBalance")}
      </label>
      <label class="switch-row">
        <input
          type="checkbox"
          checked={profile.tgWsVerbose}
          disabled={locked}
          onchange={(event) => onUpdate({ tgWsVerbose: event.currentTarget.checked })}
        />
        {$t("tg.verbose")}
      </label>
      <label class="switch-row">
        <input
          type="checkbox"
          checked={profile.tgWsForceTestDc}
          disabled={locked}
          onchange={(event) => onUpdate({ tgWsForceTestDc: event.currentTarget.checked })}
        />
        {$t("tg.forceTestDc")}
      </label>
      <small class="field-hint">{$t("tg.forceTestDcHint")}</small>
    </div>
  </details>
</section>

<style>
  .tg-settings { display: grid; gap: 14px; }
  .section-title, .test-row { display: flex; align-items: center; justify-content: space-between; gap: 14px; }
  .section-title h3 { margin-bottom: 3px; }
  .section-title p, .test-report p { margin: 0; color: var(--muted); font-size: 12px; }
  fieldset { min-width: 0; margin: 0; padding: 14px; border: 1px solid var(--line); border-radius: 14px; display: grid; gap: 12px; }
  fieldset:disabled { opacity: .65; }
  legend { padding: 0 7px; font-weight: 700; font-size: 13px; }
  label { min-width: 0; }
  .connection-row { grid-template-columns: minmax(0, 1fr) 132px; }
  .switch-row { display: flex; align-items: center; gap: 9px; font-weight: 600; }
  .switch-row input { width: 18px; height: 18px; flex: 0 0 auto; }
  .secondary-switch { font-weight: 500; }
  .input-action { display: grid; grid-template-columns: minmax(0, 1fr) 42px; gap: 8px; }
  .input-action input { min-width: 0; }
  .icon-action, .test-button { display: inline-flex; align-items: center; justify-content: center; gap: 7px; border: 1px solid color-mix(in srgb, var(--accent) 55%, var(--line)); background: color-mix(in srgb, var(--accent) 14%, transparent); color: var(--text); border-radius: 10px; cursor: pointer; }
  .icon-action { width: 42px; }
  .test-button { min-height: 36px; padding: 0 13px; font-weight: 700; }
  .icon-action:disabled, .test-button:disabled { opacity: .45; cursor: default; }
  .automatic-hint { margin: 0; }
  .test-report { border: 1px solid color-mix(in srgb, #3dcc80 55%, var(--line)); background: color-mix(in srgb, #3dcc80 8%, transparent); border-radius: 14px; padding: 12px; display: grid; gap: 9px; }
  .test-report.error-result { border-color: color-mix(in srgb, #ef6464 55%, var(--line)); background: color-mix(in srgb, #ef6464 8%, transparent); }
  .report-title { display: flex; align-items: center; gap: 7px; }
  .probe-grid { display: grid; gap: 5px; max-height: 220px; overflow: auto; }
  .probe-row { display: flex; justify-content: space-between; gap: 10px; padding: 7px 9px; border-radius: 8px; background: color-mix(in srgb, #3dcc80 9%, transparent); font-size: 12px; }
  .probe-row.failed { background: color-mix(in srgb, #ef6464 10%, transparent); }
  .probe-row span { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .advanced-content { display: grid; gap: 12px; padding-top: 12px; }
  code { overflow-wrap: anywhere; }
  .spin { display: inline-flex; animation: spin .8s linear infinite; }
  @keyframes spin { to { transform: rotate(360deg); } }
  @media (max-width: 560px) {
    .section-title { align-items: flex-start; flex-direction: column; }
    .connection-row { grid-template-columns: minmax(0, 1fr) 108px; }
  }
</style>
