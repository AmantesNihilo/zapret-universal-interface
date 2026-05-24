from __future__ import annotations

import json
from pathlib import Path


ResultTuple = tuple[int, int, int, int, int, int]
TargetDetails = list[dict[str, object]]


def load_results(path: Path) -> dict[str, ResultTuple]:
    try:
        raw = json.loads(path.read_text(encoding="utf-8"))
    except (FileNotFoundError, json.JSONDecodeError, OSError):
        return {}

    loaded: dict[str, ResultTuple] = {}
    if not isinstance(raw, dict):
        return loaded

    for name, value in raw.items():
        if not isinstance(value, dict):
            continue
        try:
            loaded[str(name)] = (
                int(value.get("ok", 0)),
                int(value.get("fail", 0)),
                int(value.get("unsup", 0)),
                int(value.get("ping_ok", 0)),
                int(value.get("ping_total", 0)),
                int(value.get("score", 0)),
            )
        except (TypeError, ValueError):
            continue
    return loaded


def save_results(path: Path, results: dict[str, ResultTuple]) -> None:
    save_results_with_details(path, results, {})


def load_result_details(path: Path) -> dict[str, TargetDetails]:
    try:
        raw = json.loads(path.read_text(encoding="utf-8"))
    except (FileNotFoundError, json.JSONDecodeError, OSError):
        return {}

    loaded: dict[str, TargetDetails] = {}
    if not isinstance(raw, dict):
        return loaded

    for name, value in raw.items():
        if not isinstance(value, dict):
            continue
        details = value.get("details", [])
        if not isinstance(details, list):
            continue
        normalized: TargetDetails = []
        for detail in details:
            if not isinstance(detail, dict):
                continue
            normalized.append(
                {
                    "name": str(detail.get("name", "")),
                    "category": str(detail.get("category", "")),
                    "service": str(detail.get("service", "")),
                    "primary": bool(detail.get("primary", False)),
                    "url": str(detail.get("url", "")),
                    "ping_target": str(detail.get("ping_target", "")),
                    "http_tokens": [str(token) for token in detail.get("http_tokens", [])]
                    if isinstance(detail.get("http_tokens", []), list)
                    else [],
                    "ping_ok": bool(detail.get("ping_ok", False)),
                    "ping_text": str(detail.get("ping_text", "")),
                }
            )
        loaded[str(name)] = normalized
    return loaded


def save_results_with_details(
    path: Path,
    results: dict[str, ResultTuple],
    details: dict[str, TargetDetails] | None = None,
) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    details = details or {}
    payload = {
        name: {
            "ok": ok,
            "fail": fail,
            "unsup": unsup,
            "ping_ok": ping_ok,
            "ping_total": ping_total,
            "score": score,
            "details": details.get(name, []),
        }
        for name, (ok, fail, unsup, ping_ok, ping_total, score) in sorted(
            results.items(), key=lambda item: item[0].casefold()
        )
    }
    path.write_text(json.dumps(payload, ensure_ascii=False, indent=2), encoding="utf-8")
