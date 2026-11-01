import type { TgWsConnectivityReport, UpdateAsset, UpdateCheck } from "./types";
import {
  readArray, readBoolean, readInteger, readNonnegativeNumber,
  readObject, readOptional, readString, ResponseValidationError
} from "./validation";

function readUpdateAsset(value: unknown, path: string): UpdateAsset {
  const asset = readObject(value, path);
  return {
    name: readString(asset.name, path + ".name"),
    downloadUrl: readString(asset.downloadUrl, path + ".downloadUrl"),
    size: readInteger(asset.size, path + ".size"),
    kind: readString(asset.kind, path + ".kind")
  };
}

export function parseUpdateCheck(value: unknown): UpdateCheck {
  const data = readObject(value, "update");
  return {
    updateAvailable: readBoolean(data.updateAvailable, "update.updateAvailable"),
    currentVersion: readString(data.currentVersion, "update.currentVersion"),
    latestVersion: readOptional(data.latestVersion, "update.latestVersion", readString),
    releaseName: readOptional(data.releaseName, "update.releaseName", readString),
    releaseNotes: readOptional(data.releaseNotes, "update.releaseNotes", readString),
    releaseUrl: readOptional(data.releaseUrl, "update.releaseUrl", readString),
    publishedAt: readOptional(data.publishedAt, "update.publishedAt", readString),
    // Keep compatibility with native distribution names added in future versions.
    distribution: readString(data.distribution, "update.distribution"),
    canInstall: readBoolean(data.canInstall, "update.canInstall"),
    installerAsset: readOptional(data.installerAsset, "update.installerAsset", readUpdateAsset),
    portableAsset: readOptional(data.portableAsset, "update.portableAsset", readUpdateAsset)
  };
}

export function parseConnectivityReport(value: unknown): TgWsConnectivityReport {
  const data = readObject(value, "connectivity");
  const kind = readString(data.kind, "connectivity.kind");
  if (kind !== "cfProxy" && kind !== "cfWorker") {
    throw new ResponseValidationError({ path: "connectivity.kind", expected: "cfProxy или cfWorker" });
  }
  return {
    kind,
    allOk: readBoolean(data.allOk, "connectivity.allOk"),
    probes: readArray(data.probes, "connectivity.probes").map((value, index) => {
      const path = "connectivity.probes[" + index + "]";
      const probe = readObject(value, path);
      return {
        domain: readString(probe.domain, path + ".domain"),
        dc: readInteger(probe.dc, path + ".dc"),
        target: readString(probe.target, path + ".target"),
        ok: readBoolean(probe.ok, path + ".ok"),
        latencyMs: readOptional(probe.latencyMs, path + ".latencyMs", readNonnegativeNumber),
        detail: readString(probe.detail, path + ".detail")
      };
    })
  };
}
