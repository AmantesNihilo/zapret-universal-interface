#!/usr/bin/env python3
"""
Build an experimental personal preset from standalone preset-test logs.

The composer is intentionally conservative. It does not try to understand every
winws2 option semantically; it chooses winning presets from CSV logs, then
combines their reusable preset blocks with light de-duplication and a report.
"""

from __future__ import annotations

import argparse
import csv
import re
import sys
from collections import Counter, defaultdict
from dataclasses import dataclass, field
from datetime import datetime
from pathlib import Path
from typing import Iterable


ROOT = Path(__file__).resolve().parents[1]
PRESETS_DIR = ROOT / "presets"
DEFAULT_LOGS_DIR = ROOT / "utils" / "preset-test-logs"
DEFAULT_OUT = PRESETS_DIR / "User Auto Best.txt"

BAD_PRESET_HINTS = (
    "nofake",
    "h2 downgrade",
    "fixed",
)

SCORE_COLUMNS = ("score", "ok", "slow", "block", "fail", "unsup", "dns_loopback")
PASSTHROUGH_PREFIXES = (
    "--lua-init=",
    "--ctrack-",
    "--ipcache-",
    "--blob=",
    "--wf-",
)
FILTER_PREFIXES = (
    "--filter-",
    "--hostlist=",
    "--ipset=",
    "--ipset-exclude=",
    "--out-range=",
    "--payload=",
    "--lua-desync=",
)


@dataclass
class SummaryStat:
    runs: int = 0
    score: float = 0
    ok: int = 0
    slow: int = 0
    block: int = 0
    fail: int = 0
    unsup: int = 0
    dns_loopback: int = 0

    def add(self, row: dict[str, str]) -> None:
        self.runs += 1
        self.score += number(row.get("score"))
        self.ok += int(number(row.get("ok")))
        self.slow += int(number(row.get("slow")))
        self.block += int(number(row.get("block")))
        self.fail += int(number(row.get("fail")))
        self.unsup += int(number(row.get("unsup")))
        self.dns_loopback += int(number(row.get("dns_loopback")))

    @property
    def avg_score(self) -> float:
        return self.score / self.runs if self.runs else 0


@dataclass
class TargetStat:
    score: float = 0
    ok: int = 0
    slow: int = 0
    block: int = 0
    fail: int = 0
    rows: int = 0

    def add_issue_row(self, row: dict[str, str]) -> None:
        checks = parse_checks(row.get("checks", ""))
        if checks:
            self.ok += checks["ok"]
            self.slow += checks["slow"]
            self.block += checks["block"]
            self.fail += checks["fail"]
        else:
            cls = (row.get("class") or "").upper()
            if "FAIL" in cls:
                self.fail += 1
            elif "BLOCK" in cls:
                self.block += 1
            elif "SLOW" in cls or "THROTTLED" in cls:
                self.slow += 1
            elif "PARTIAL" in cls or "OK" in cls:
                self.ok += 1
        self.score += target_row_score(row)
        self.rows += 1


@dataclass
class PresetFile:
    name: str
    path: Path
    globals: list[str]
    blocks: list[list[str]]


@dataclass
class ComposeModel:
    summary: dict[str, SummaryStat] = field(default_factory=lambda: defaultdict(SummaryStat))
    target_stats: dict[tuple[str, str, str], dict[str, TargetStat]] = field(
        default_factory=lambda: defaultdict(lambda: defaultdict(TargetStat))
    )
    source_files: set[Path] = field(default_factory=set)


def number(value: object, default: float = 0) -> float:
    if value in (None, ""):
        return default
    try:
        return float(str(value).strip())
    except ValueError:
        return default


def read_csv(path: Path) -> list[dict[str, str]]:
    with path.open("r", encoding="utf-8-sig", newline="") as fh:
        return list(csv.DictReader(fh))


def find_logs(paths: Iterable[Path]) -> tuple[list[Path], list[Path]]:
    summary_paths: list[Path] = []
    issue_paths: list[Path] = []
    for path in paths:
        if path.is_file():
            if path.name.lower() == "summary.csv":
                summary_paths.append(path)
            elif path.name.lower() == "issues.csv":
                issue_paths.append(path)
            continue
        if path.exists():
            summary_paths.extend(path.rglob("summary.csv"))
            issue_paths.extend(path.rglob("issues.csv"))
    return sorted(set(summary_paths)), sorted(set(issue_paths))


def parse_checks(checks: str) -> Counter:
    result: Counter = Counter()
    for part in checks.split():
        if ":" not in part:
            continue
        _, status = part.split(":", 1)
        status = status.split("(", 1)[0].upper()
        if status.startswith(("OK", "HTTP400", "HTTP404")):
            result["ok"] += 1
        elif status.startswith("SLOW"):
            result["slow"] += 1
        elif "BLOCK" in status:
            result["block"] += 1
        elif status.startswith("UNSUP"):
            result["unsup"] += 1
        elif status:
            result["fail"] += 1
    return result


def target_row_score(row: dict[str, str]) -> float:
    checks = parse_checks(row.get("checks", ""))
    if checks:
        return checks["ok"] * 10 + checks["slow"] * 3 - checks["block"] * 20 - checks["fail"] * 7
    cls = (row.get("class") or "").upper()
    if "DNS_LOOPBACK" in cls:
        return -40
    if "FAIL" in cls:
        return -20
    if "BLOCK" in cls:
        return -30
    if "THROTTLED" in cls or "SLOW" in cls:
        return 5
    if "PARTIAL" in cls or "OK" in cls:
        return 20
    return 0


def load_model(log_paths: list[Path]) -> ComposeModel:
    summaries, issues = find_logs(log_paths)
    model = ComposeModel()
    for path in summaries:
        model.source_files.add(path)
        for row in read_csv(path):
            preset = (row.get("preset") or "").strip()
            if not preset or preset.upper() == "BASELINE":
                continue
            model.summary[preset].add(row)
    for path in issues:
        model.source_files.add(path)
        for row in read_csv(path):
            preset = (row.get("preset") or "").strip()
            if not preset or preset.upper() == "BASELINE":
                continue
            category = (row.get("category") or "?").strip()
            service = (row.get("service") or "?").strip()
            target = (row.get("target") or "?").strip()
            model.target_stats[(category, service, target)][preset].add_issue_row(row)
    return model


def preset_path(name: str) -> Path | None:
    direct = PRESETS_DIR / f"{name}.txt"
    if direct.exists():
        return direct
    lowered = name.casefold()
    for path in PRESETS_DIR.glob("*.txt"):
        if path.stem.casefold() == lowered:
            return path
    return None


def is_bad_candidate(name: str) -> bool:
    lowered = name.casefold()
    return any(hint in lowered for hint in BAD_PRESET_HINTS)


def choose_presets(
    model: ComposeModel,
    max_presets: int,
    include: list[str],
    allow_experimental_bad: bool,
) -> list[str]:
    votes: Counter = Counter()

    for preset, stat in model.summary.items():
        if not allow_experimental_bad and is_bad_candidate(preset):
            continue
        votes[preset] += stat.avg_score
        votes[preset] += stat.ok * 2
        votes[preset] -= stat.fail * 3

    for _, by_preset in model.target_stats.items():
        ranked = sorted(
            by_preset.items(),
            key=lambda item: (
                item[1].score,
                item[1].ok,
                -item[1].fail,
                -item[1].block,
            ),
            reverse=True,
        )
        if not ranked:
            continue
        winner, stat = ranked[0]
        if not allow_experimental_bad and is_bad_candidate(winner):
            continue
        votes[winner] += 50 + stat.score

    selected: list[str] = []
    for name in include:
        if name not in selected:
            selected.append(name)
    for name, _ in votes.most_common():
        if len(selected) >= max_presets:
            break
        if name not in selected:
            selected.append(name)
    return selected


def split_preset(path: Path) -> PresetFile:
    lines = path.read_text(encoding="utf-8-sig").splitlines()
    globals_: list[str] = []
    blocks: list[list[str]] = []
    current: list[str] = []
    in_blocks = False
    for raw in lines:
        line = raw.strip()
        if not line or line.startswith("#"):
            continue
        if line == "--new":
            if current:
                blocks.append(current)
            current = []
            in_blocks = True
            continue
        if not in_blocks and line.startswith(FILTER_PREFIXES):
            in_blocks = True
        if in_blocks:
            current.append(line)
        elif line.startswith(PASSTHROUGH_PREFIXES):
            globals_.append(line)
    if current:
        blocks.append(current)
    return PresetFile(name=path.stem, path=path, globals=globals_, blocks=blocks)


def directive_key(line: str) -> str:
    if line.startswith("--blob="):
        return line.split(":", 1)[0]
    if line.startswith("--wf-tcp-out="):
        return "--wf-tcp-out"
    if line.startswith("--wf-udp-out="):
        return "--wf-udp-out"
    if line.startswith("--wf-raw-part="):
        return line
    if line.startswith("--lua-init="):
        return line
    if "=" in line:
        return line.split("=", 1)[0]
    return line


def merge_port_specs(lines: list[str], key: str) -> str | None:
    values: list[str] = []
    prefix = f"{key}="
    for line in lines:
        if line.startswith(prefix):
            values.extend(part.strip() for part in line[len(prefix) :].split(",") if part.strip())
    if not values:
        return None
    unique = sorted(set(values), key=port_sort_key)
    return prefix + ",".join(unique)


def port_sort_key(value: str) -> tuple[int, str]:
    first = value.split("-", 1)[0]
    try:
        return int(first), value
    except ValueError:
        return 999999, value


def merged_globals(presets: list[PresetFile]) -> list[str]:
    raw = [line for preset in presets for line in preset.globals]
    output: list[str] = []
    seen: set[str] = set()

    tcp = merge_port_specs(raw, "--wf-tcp-out")
    udp = merge_port_specs(raw, "--wf-udp-out")

    for line in raw:
        if line.startswith(("--wf-tcp-out=", "--wf-udp-out=")):
            continue
        key = directive_key(line)
        if key in seen:
            continue
        seen.add(key)
        output.append(line)

    if tcp:
        output.append(tcp)
    if udp:
        output.append(udp)
    return output


def block_signature(block: list[str]) -> tuple[str, ...]:
    return tuple(line for line in block if line)


def compose_preset(name: str, preset_files: list[PresetFile], model: ComposeModel) -> str:
    created = datetime.now().isoformat(timespec="seconds")
    lines: list[str] = [
        f"# Preset: {name}",
        f"# ActivePreset: {name}",
        "# GeneratedBy: tools/compose.py",
        f"# Created: {created}",
        "# Experimental: personal composed preset; verify with standalone tests before app use",
        "# Sources: " + ", ".join(preset.name for preset in preset_files),
        "# IconColor: #20c997ff",
        "",
    ]

    lines.extend(merged_globals(preset_files))
    lines.append("")

    seen_blocks: set[tuple[str, ...]] = set()
    first = True
    for preset in preset_files:
        for block in preset.blocks:
            sig = block_signature(block)
            if not sig or sig in seen_blocks:
                continue
            seen_blocks.add(sig)
            if not first:
                lines.extend(["", "--new", ""])
            first = False
            lines.append(f"# From: {preset.name}")
            lines.extend(block)

    lines.append("")
    return "\n".join(lines)


def write_report(path: Path, selected: list[str], model: ComposeModel) -> None:
    report = path.with_suffix(path.suffix + ".report.csv")
    with report.open("w", encoding="utf-8", newline="") as fh:
        writer = csv.writer(fh)
        writer.writerow(["kind", "name", "runs", "avg_score", "ok", "slow", "fail", "note"])
        for name in selected:
            stat = model.summary.get(name)
            if stat:
                writer.writerow(
                    [
                        "summary",
                        name,
                        stat.runs,
                        f"{stat.avg_score:.2f}",
                        stat.ok,
                        stat.slow,
                        stat.fail,
                        "",
                    ]
                )
            else:
                writer.writerow(["manual", name, "", "", "", "", "", "included manually or no summary rows"])
        for (category, service, target), by_preset in sorted(model.target_stats.items()):
            if not by_preset:
                continue
            winner, stat = max(by_preset.items(), key=lambda item: (item[1].score, item[1].ok, -item[1].fail))
            writer.writerow(
                [
                    "target_winner",
                    winner,
                    "",
                    f"{stat.score:.2f}",
                    stat.ok,
                    stat.slow,
                    stat.fail,
                    f"{category}/{service}/{target}",
                ]
            )


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Compose a personal preset from preset-test logs.")
    parser.add_argument(
        "--logs",
        action="append",
        type=Path,
        default=[],
        help="Log directory or CSV file. Can be repeated. Defaults to utils/preset-test-logs.",
    )
    parser.add_argument("--out", type=Path, default=DEFAULT_OUT, help="Output preset path.")
    parser.add_argument("--name", default="User Auto Best", help="Generated preset name.")
    parser.add_argument("--max-presets", type=int, default=3, help="Maximum winning source presets to merge.")
    parser.add_argument("--include-preset", action="append", default=[], help="Force include preset by name.")
    parser.add_argument("--allow-bad", action="store_true", help="Allow known weak experimental candidates.")
    parser.add_argument("--dry-run", action="store_true", help="Print planned source presets only.")
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    args = parse_args(argv or sys.argv[1:])
    log_paths = args.logs or [DEFAULT_LOGS_DIR]
    model = load_model(log_paths)

    if not model.summary and not model.target_stats and not args.include_preset:
        print("No logs found. Run preset tests first or pass --include-preset.", file=sys.stderr)
        return 2

    selected = choose_presets(
        model=model,
        max_presets=max(1, args.max_presets),
        include=args.include_preset,
        allow_experimental_bad=args.allow_bad,
    )
    if not selected:
        print("No candidate presets selected.", file=sys.stderr)
        return 2

    missing = [name for name in selected if preset_path(name) is None]
    if missing:
        print("Missing preset files: " + ", ".join(missing), file=sys.stderr)
        return 1

    files = [split_preset(preset_path(name)) for name in selected if preset_path(name)]
    print("Selected presets:")
    for preset in files:
        stat = model.summary.get(preset.name)
        suffix = f" avg_score={stat.avg_score:.1f} runs={stat.runs}" if stat else ""
        print(f"  - {preset.name}{suffix}")

    if args.dry_run:
        return 0

    out = args.out
    if not out.is_absolute():
        out = ROOT / out
    out.parent.mkdir(parents=True, exist_ok=True)
    text = compose_preset(args.name, files, model)
    out.write_text(text, encoding="utf-8", newline="\n")
    write_report(out, selected, model)
    print(f"Saved: {out}")
    print(f"Report: {out.with_suffix(out.suffix + '.report.csv')}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
