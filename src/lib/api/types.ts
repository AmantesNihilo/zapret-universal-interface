export type ThemeMode = "dark" | "light" | "oled" | "system";
export type LayoutOrientation = "portrait" | "landscape";
export type MinimizeBehavior = "taskbar" | "tray";
export type AppStatus = "off" | "starting" | "on" | "stopping" | "error";
export type ServiceState = "stopped" | "starting" | "running" | "stopping" | "error";
export type ServiceName = "zapret" | "tg-ws";
export type PresetKind = "bat" | "cmd" | "config" | "unknown";
export type ZapretEngine = "classic" | "zapret2";
export type LogSource = "app" | "zapret" | "tgWs" | "tests";
export type TestMode = "selected" | "all";
export type TestServiceStatus = "passed" | "partial" | "failed";
export type TestRecommendationState = "recommended" | "partial" | "notRecommended";

export type TestTargetConfig = {
  service: string;
  name: string;
  value: string;
  enabled: boolean;
};

export type Settings = {
  theme: ThemeMode;
  accent: string;
  language: string;
  layoutOrientation: LayoutOrientation;
  launchMinimized: boolean;
  closeToTray: boolean;
  minimizeBehavior: MinimizeBehavior;
  minimizeDontAsk: boolean;
  startWithWindows: boolean;
  startWithWindowsInTray: boolean;
  autoStartActiveProfileOnLaunch: boolean;
  checkUpdatesOnLaunch: boolean;
  customPresetRoots: string[];
  testTargets: TestTargetConfig[];
};

export type Profile = {
  id: string;
  name: string;
  zapretEnabled: boolean;
  zapretEngine: ZapretEngine | null;
  zapretPresetId: string | null;
  tgWsEnabled: boolean;
  tgWsHost: string;
  tgWsPort: number;
  tgWsSecret: string;
  tgWsDcIps: string[];
  tgWsCfProxyEnabled: boolean;
  tgWsCfCustomEnabled: boolean;
  tgWsDefaultDomains: boolean;
  tgWsCfDomains: string[];
  tgWsCfWorkerEnabled: boolean;
  tgWsCfWorkerDomain?: string | null;
  tgWsFrontingDomain?: string | null;
  tgWsCfPriority: boolean;
  tgWsCfBalance: boolean;
  tgWsBufKb: number;
  tgWsPoolSize: number;
  tgWsVerbose: boolean;
  tgWsLogMaxMb: number;
  tgWsForceTestDc: boolean;
  autostartOnAppLaunch?: boolean;
  notes?: string | null;
};

export type TgWsConnectivityKind = "cfProxy" | "cfWorker";

export type TgWsConnectivityProbe = {
  domain: string;
  dc: number;
  target: string;
  ok: boolean;
  latencyMs?: number | null;
  detail: string;
};

export type TgWsConnectivityReport = {
  kind: TgWsConnectivityKind;
  allOk: boolean;
  probes: TgWsConnectivityProbe[];
};

export type ProfilesFile = {
  activeProfileId: string;
  profiles: Profile[];
};

export type Preset = {
  id: string;
  name: string;
  path: string;
  relativePath: string;
  engine: ZapretEngine;
  kind: PresetKind;
  favorite: boolean;
  hidden: boolean;
};

export type ServiceStatus = {
  service: ServiceName;
  state: ServiceState;
  pid?: number | null;
  message?: string | null;
  error?: string | null;
};

export type ConflictProcess = {
  image: string;
  pid: number;
  title?: string | null;
};

export type AppState = {
  status: AppStatus;
  activeProfileId: string;
  zapret: ServiceStatus;
  tgWs: ServiceStatus;
  lastError?: string | null;
};

export type LogLine = {
  source: LogSource;
  timestamp: string;
  message: string;
};

export type TestTargetResult = {
  service: string;
  label: string;
  url: string;
  ok: boolean;
  status?: number | null;
  latencyMs?: number | null;
  error?: string | null;
};

export type ServiceTestResult = {
  name: string;
  status: TestServiceStatus;
  ok: number;
  total: number;
  errors: string[];
  targets: TestTargetResult[];
};

export type TestResult = {
  id: string;
  presetId: string;
  presetName: string;
  engine: ZapretEngine;
  presetVersion: string;
  mode: TestMode;
  startedAt: string;
  finishedAt: string;
  cachedAt: string;
  recommendation: TestRecommendationState;
  score: number;
  ok: number;
  total: number;
  services: ServiceTestResult[];
};

export type TestPhase = "starting" | "warmup" | "checking" | "finishing";

export type TestProgress = {
  testId: string;
  presetId: string;
  presetName: string;
  engine: ZapretEngine;
  presetIndex: number;
  presetCount: number;
  totalChecks: number;
  completedChecks: number;
  passedChecks: number;
  failedChecks: number;
  phase: TestPhase;
  currentTarget?: string | null;
};

export type Diagnostics = {
  resourcesPath: string;
  dataPath: string;
  logsPath: string;
  presetCount: number;
  selectedPresetExists: boolean;
  winwsFound: boolean;
  winws2Found: boolean;
  tgWsFound: boolean;
  tgWsEngine: string;
  tgWsEngineVersion: string;
  winwsRunning: boolean;
  winws2Running: boolean;
  tgWsRunning: boolean;
  isAdmin: boolean;
  tgWsPortAvailable: boolean;
  warnings: string[];
};

export type UpdateAsset = {
  name: string;
  downloadUrl: string;
  size: number;
  kind: string;
};

export type UpdateCheck = {
  updateAvailable: boolean;
  currentVersion: string;
  latestVersion?: string | null;
  releaseName?: string | null;
  releaseNotes?: string | null;
  releaseUrl?: string | null;
  publishedAt?: string | null;
  distribution: "installed" | "portable" | "development" | string;
  canInstall: boolean;
  installerAsset?: UpdateAsset | null;
  portableAsset?: UpdateAsset | null;
};
