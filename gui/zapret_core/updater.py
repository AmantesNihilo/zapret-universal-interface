from __future__ import annotations

import json
import re
import urllib.error
import urllib.request
from dataclasses import dataclass


REPO = "AmantesNihilo/Z2-GUI-windows"
LATEST_RELEASE_URL = f"https://api.github.com/repos/{REPO}/releases/latest"

@dataclass(frozen=True)
class UpdateInfo:
    current_version: str
    latest_version: str
    latest_tag: str
    release_url: str
    asset_url: str
    asset_name: str
    changes: str
    is_newer: bool


def check_for_updates(current_version: str, timeout_sec: int = 10) -> UpdateInfo:
    release = _fetch_json(LATEST_RELEASE_URL, timeout_sec)
    latest_tag = str(release.get("tag_name", "")).strip()
    latest_version = _normalize_version(latest_tag)
    release_url = str(release.get("html_url", "")).strip()
    release_body = str(release.get("body", "")).strip()
    asset_name, asset_url = _portable_asset(release.get("assets", []))

    changes = _fetch_first_text(_changes_urls(latest_tag), timeout_sec).strip()
    if not changes:
        changes = release_body
    if not changes:
        changes = "No changelog is available for this release."

    return UpdateInfo(
        current_version=current_version,
        latest_version=latest_version,
        latest_tag=latest_tag or f"v{latest_version}",
        release_url=release_url,
        asset_url=asset_url,
        asset_name=asset_name,
        changes=changes,
        is_newer=_version_tuple(latest_version) > _version_tuple(current_version),
    )


def _fetch_json(url: str, timeout_sec: int) -> dict:
    text = _fetch_text(url, timeout_sec)
    payload = json.loads(text)
    if not isinstance(payload, dict):
        raise ValueError("Unexpected GitHub response.")
    return payload


def _fetch_first_text(urls: tuple[str, ...], timeout_sec: int) -> str:
    for url in urls:
        try:
            return _fetch_text(url, timeout_sec)
        except RuntimeError:
            continue
    return ""


def _changes_urls(latest_tag: str) -> tuple[str, ...]:
    refs = []
    tag = latest_tag.strip()
    if tag:
        refs.append(tag)
    refs.extend(["main", "master"])

    urls = []
    seen = set()
    for ref in refs:
        for path in ("docs/CHANGES.md", "CHANGES.md"):
            url = f"https://raw.githubusercontent.com/{REPO}/{ref}/{path}"
            if url not in seen:
                seen.add(url)
                urls.append(url)
    return tuple(urls)


def _fetch_text(url: str, timeout_sec: int) -> str:
    request = urllib.request.Request(
        url,
        headers={
            "Accept": "application/vnd.github+json, text/plain",
            "User-Agent": "Z2-GUI-update-checker",
        },
    )
    try:
        with urllib.request.urlopen(request, timeout=timeout_sec) as response:
            return response.read().decode("utf-8", errors="replace")
    except urllib.error.HTTPError as exc:
        if exc.code == 404 and "releases/latest" in url:
            raise RuntimeError("No published GitHub release was found yet.") from exc
        raise RuntimeError(f"GitHub returned HTTP {exc.code}.") from exc
    except urllib.error.URLError as exc:
        raise RuntimeError(f"Could not connect to GitHub: {exc.reason}") from exc


def _portable_asset(assets: object) -> tuple[str, str]:
    if not isinstance(assets, list):
        return "", ""

    fallback: tuple[str, str] | None = None
    for asset in assets:
        if not isinstance(asset, dict):
            continue
        name = str(asset.get("name", "")).strip()
        url = str(asset.get("browser_download_url", "")).strip()
        if not name or not url:
            continue
        if fallback is None:
            fallback = (name, url)
        lowered = name.casefold()
        if "portable" in lowered and lowered.endswith(".zip"):
            return name, url
    return fallback or ("", "")


def _normalize_version(version: str) -> str:
    return version.strip().lstrip("vV") or "0.0.0"


def _version_tuple(version: str) -> tuple[int, ...]:
    parts = re.findall(r"\d+", _normalize_version(version))
    numbers = tuple(int(part) for part in parts[:4])
    return numbers or (0,)
