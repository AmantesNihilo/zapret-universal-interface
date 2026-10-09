<script lang="ts">
  import { RotateCw } from "@lucide/svelte";
  import OptionDropdown, { type DropdownOption } from "$lib/components/OptionDropdown.svelte";
  import type { Settings, UpdateCheck, ZapretEngine } from "$lib/api/types";
  import { settings } from "$lib/stores/settings";
  import { t } from "$lib/stores/i18n";

  let {
    activeEngine,
    serviceConfigLocked,
    updateInfo,
    updateChecking,
    updateMessage,
    onChooseEngine,
    onUpdateSettings,
    onUpdateWindowLayout,
    onCheckUpdates
  }: {
    activeEngine: ZapretEngine | null;
    serviceConfigLocked: boolean;
    updateInfo: UpdateCheck | null;
    updateChecking: boolean;
    updateMessage: string | null;
    onChooseEngine: (engine: ZapretEngine) => void | Promise<void>;
    onUpdateSettings: (patch: Partial<Settings>) => void | Promise<void>;
    onUpdateWindowLayout: (layout: Settings["layoutOrientation"]) => void | Promise<void>;
    onCheckUpdates: () => void | Promise<void>;
  } = $props();

  const engineOptions = $derived.by((): DropdownOption[] => [
    ...(!activeEngine
      ? [{ value: "", label: $t("engine.notConfigured"), hint: $t("engine.configureHint") }]
      : []),
    { value: "classic", label: "Zapret Classic", hint: $t("engine.classicShort") },
    { value: "zapret2", label: "Zapret 2", hint: $t("engine.zapret2Short") }
  ]);
  const themeOptions = $derived.by((): DropdownOption[] => [
    { value: "dark", label: $t("theme.dark"), hint: $t("theme.darkHint") },
    { value: "oled", label: $t("theme.oled"), hint: $t("theme.oledHint") },
    { value: "light", label: $t("theme.light"), hint: $t("theme.lightHint") },
    { value: "system", label: $t("theme.system"), hint: $t("theme.systemHint") }
  ]);
  const languageOptions = $derived.by((): DropdownOption[] => [
    { value: "ru", label: $t("language.ru"), hint: $t("language.ruHint") },
    { value: "en", label: $t("language.en"), hint: $t("language.enHint") }
  ]);
  const orientationOptions = $derived.by((): DropdownOption[] => [
    { value: "portrait", label: $t("orientation.portrait"), hint: $t("orientation.portraitHint") },
    { value: "landscape", label: $t("orientation.landscape"), hint: $t("orientation.landscapeHint") }
  ]);
  const minimizeOptions = $derived.by((): DropdownOption[] => [
    { value: "taskbar", label: $t("minimize.taskbar"), hint: $t("minimize.taskbarHint") },
    { value: "tray", label: $t("minimize.tray"), hint: $t("minimize.trayHint") }
  ]);
  const accentOptions = $derived.by((): DropdownOption[] => [
    { value: "cyan", label: $t("accent.cyan"), color: "#25c7d9" },
    { value: "teal", label: $t("accent.teal"), color: "#19d3b5" },
    { value: "green", label: $t("accent.green"), color: "#48c774" },
    { value: "lime", label: $t("accent.lime"), color: "#a3e635" },
    { value: "blue", label: $t("accent.blue"), color: "#4c8df6" },
    { value: "violet", label: $t("accent.violet"), color: "#8b5cf6" },
    { value: "pink", label: $t("accent.pink"), color: "#ec5ead" },
    { value: "red", label: $t("accent.red"), color: "#ef5b5b" },
    { value: "orange", label: $t("accent.orange"), color: "#f97316" },
    { value: "amber", label: $t("accent.amber"), color: "#e5ae38" }
  ]);
</script>

<section>
  <h3>{$t("settings.general")}</h3>
  <label>
    {$t("engine.title")}
    <OptionDropdown
      options={engineOptions}
      value={activeEngine ?? ""}
      disabled={serviceConfigLocked}
      onChange={(engine) => {
        if (engine) void onChooseEngine(engine as ZapretEngine);
      }}
    />
  </label>
  <label>
    {$t("settings.theme")}
    <OptionDropdown
      options={themeOptions}
      value={$settings.theme}
      onChange={(theme) => onUpdateSettings({ theme: theme as Settings["theme"] })}
    />
  </label>
  <label>
    {$t("settings.language")}
    <OptionDropdown
      options={languageOptions}
      value={$settings.language}
      onChange={(language) => onUpdateSettings({ language })}
    />
  </label>
  <label>
    {$t("settings.orientation")}
    <OptionDropdown
      options={orientationOptions}
      value={$settings.layoutOrientation}
      onChange={(layoutOrientation) => onUpdateWindowLayout(layoutOrientation as Settings["layoutOrientation"])}
    />
  </label>
  <div class="settings-subsection minimize-settings">
    <div>
      <strong>{$t("minimize.settingsTitle")}</strong>
      <span>{$t("minimize.settingsHint")}</span>
    </div>
    <label>
      {$t("minimize.defaultAction")}
      <OptionDropdown
        options={minimizeOptions}
        value={$settings.minimizeBehavior}
        onChange={(minimizeBehavior) => onUpdateSettings({
          minimizeBehavior: minimizeBehavior as Settings["minimizeBehavior"]
        })}
      />
    </label>
    <label class="check-row">
      <input
        type="checkbox"
        checked={$settings.minimizeDontAsk}
        onchange={(event) => onUpdateSettings({ minimizeDontAsk: event.currentTarget.checked })}
      />
      {$t("minimize.dontAsk")}
    </label>
  </div>
  <label>
    {$t("settings.accent")}
    <OptionDropdown
      options={accentOptions}
      value={$settings.accent}
      onChange={(accent) => onUpdateSettings({ accent })}
    />
  </label>
  <label class="check-row">
    <input
      type="checkbox"
      checked={$settings.startWithWindows}
      onchange={(event) => onUpdateSettings({ startWithWindows: event.currentTarget.checked })}
    />
    {$t("settings.startWithWindows")}
  </label>
  <label class="check-row startup-tray-option" class:disabled={!$settings.startWithWindows}>
    <input
      type="checkbox"
      checked={$settings.startWithWindowsInTray}
      disabled={!$settings.startWithWindows}
      onchange={(event) => onUpdateSettings({ startWithWindowsInTray: event.currentTarget.checked })}
    />
    <span>
      {$t("settings.startWithWindowsInTray")}
      <small>{$t("settings.startWithWindowsInTrayHint")}</small>
    </span>
  </label>
  <label class="check-row">
    <input
      type="checkbox"
      checked={$settings.autoStartActiveProfileOnLaunch}
      onchange={(event) => onUpdateSettings({ autoStartActiveProfileOnLaunch: event.currentTarget.checked })}
    />
    {$t("settings.autoStart")}
  </label>
  <label class="check-row">
    <input
      type="checkbox"
      checked={$settings.checkUpdatesOnLaunch}
      onchange={(event) => onUpdateSettings({ checkUpdatesOnLaunch: event.currentTarget.checked })}
    />
    {$t("settings.checkUpdatesOnLaunch")}
  </label>
  <div class="update-card">
    <div>
      <strong>{$t("update.title")}</strong>
      <span>
        {#if updateMessage}
          {updateMessage}
        {:else if updateInfo?.updateAvailable}
          {$t("update.availableShort", { version: updateInfo.latestVersion ?? "" })}
        {:else if updateInfo}
          {$t("update.current", { version: updateInfo.currentVersion })}
        {:else}
          {$t("update.startupHint")}
        {/if}
      </span>
    </div>
    <button class="secondary-button" type="button" disabled={updateChecking} onclick={onCheckUpdates}>
      <RotateCw size={16} /> {updateChecking ? $t("update.checking") : $t("update.check")}
    </button>
  </div>
</section>
