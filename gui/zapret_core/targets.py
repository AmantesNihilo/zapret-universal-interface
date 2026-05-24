from __future__ import annotations

import re
from dataclasses import dataclass, replace
from pathlib import Path
from urllib.parse import urlparse


@dataclass(frozen=True)
class Target:
    name: str
    url: str | None
    ping_target: str | None
    category: str = "General"
    service: str = "General"
    primary: bool = False


def _target_from_value(name: str, value: str, category: str, service: str) -> Target:
    if value.upper().startswith("PING:"):
        return Target(
            name=name,
            url=None,
            ping_target=value.split(":", 1)[1].strip(),
            category=category,
            service=service,
        )

    parsed = urlparse(value)
    host = parsed.hostname or re.sub(r"^https?://", "", value).split("/", 1)[0]
    return Target(name=name, url=value, ping_target=host, category=category, service=service)


def _parse_section(title: str) -> tuple[str, str]:
    title = title.strip()
    for separator in ("/", ":", "|"):
        if separator in title:
            category, service = [part.strip() for part in title.split(separator, 1)]
            return category or service or "General", service or category or "General"
    return title or "General", title or "General"


def load_targets(path: Path) -> list[Target]:
    raw: dict[str, tuple[str, str, str]] = {}
    category = "General"
    service = "General"
    if path.exists():
        for line in path.read_text(encoding="utf-8-sig", errors="replace").splitlines():
            category_match = re.match(r"^\s*###\s+(.+?)\s*$", line)
            if category_match:
                category, service = _parse_section(category_match.group(1))
                continue

            match = re.match(r'^\s*(\w+)\s*=\s*"(.+)"\s*$', line)
            if match and match.group(1) not in raw:
                raw[match.group(1)] = (match.group(2), category, service)

    if not raw:
        raw = {
            "Discord": ("https://discord.com", "Social", "Discord"),
            "YouTube": ("https://www.youtube.com", "Video", "YouTube"),
            "Google": ("https://www.google.com", "Core", "Google"),
            "Cloudflare": ("https://www.cloudflare.com", "Core", "Cloudflare"),
            "DNS_1111": ("PING:1.1.1.1", "Core", "DNS"),
        }

    loaded = [
        _target_from_value(name, value, category_, service_)
        for name, (value, category_, service_) in raw.items()
    ]
    seen_services: set[str] = set()
    marked: list[Target] = []
    for target in loaded:
        primary = target.service not in seen_services
        seen_services.add(target.service)
        marked.append(replace(target, primary=primary))
    return marked
