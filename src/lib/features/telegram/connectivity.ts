import type { TgWsConnectivityKind, TgWsConnectivityReport } from "$lib/api/types";

export type ConnectivityTestState =
  | { status: "idle" }
  | { status: "loading"; kind: TgWsConnectivityKind }
  | { status: "success"; report: TgWsConnectivityReport }
  | { status: "error"; message: string };
