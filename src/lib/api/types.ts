export type ThemeMode = "dark" | "light" | "oled" | "system";
export type LayoutOrientation = "portrait" | "landscape";
export type AppStatus = "off" | "starting" | "on" | "stopping" | "error";
export type ServiceState = "stopped" | "starting" | "running" | "stopping" | "error";
export type ServiceName = "zapret" | "tg-ws";
export type PresetKind = "bat" | "cmd" | "config" | "unknown";
export type LogSource = "app" | "zapret" | "tgWs" | "tests";
export type TestMode = "quick" | "full" | "advanced";
export type TestServiceStatus = "passed" | "partial" | "failed";

export type Settings = {
  theme: ThemeMode;
  accent: string;
  language: string;
  layoutOrientation: LayoutOrientation;
  launchMinimized: boolean;
  closeToTray: boolean;
  startWithWindows: boolean;
  autoStartActiveProfileOnLaunch: boolean;
  customPresetRoots: string[];
};

export type Profile = {
  id: string;
  name: string;
  zapretEnabled: boolean;
  zapretPresetId: string | null;
  tgWsEnabled: boolean;
  tgWsHost: string;
  tgWsPort: number;
  tgWsSecret: string;
  tgWsDefaultDomains: boolean;
  tgWsCfDomains: string[];
  tgWsCfWorkerDomain?: string | null;
  tgWsCfPriority: boolean;
  tgWsCfBalance: boolean;
  autostartOnAppLaunch?: boolean;
  notes?: string | null;
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
  mode: TestMode;
  startedAt: string;
  finishedAt: string;
  score: number;
  ok: number;
  total: number;
  services: ServiceTestResult[];
};

export type Diagnostics = {
  resourcesPath: string;
  dataPath: string;
  logsPath: string;
  presetCount: number;
  selectedPresetExists: boolean;
  winwsFound: boolean;
  tgWsFound: boolean;
  tgWsEngine: string;
  tgWsEngineVersion: string;
  winwsRunning: boolean;
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
