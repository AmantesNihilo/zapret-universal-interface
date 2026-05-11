from __future__ import annotations

import re
import subprocess
from concurrent.futures import Future, ThreadPoolExecutor, as_completed
from collections.abc import Callable
from dataclasses import dataclass

from .targets import Target


CREATE_NO_WINDOW = 0x08000000


@dataclass(frozen=True)
class TargetResult:
    name: str
    http_tokens: tuple[str, ...]
    ping_ok: bool
    ping_text: str


@dataclass(frozen=True)
class PresetTestResult:
    ok: int
    fail: int
    unsup: int
    ping_ok: int
    ping_total: int
    details: tuple[TargetResult, ...]


URL_TESTS = (
    ("HTTP", ("--http1.1",)),
    ("TLS1.2", ("--tlsv1.2", "--tls-max", "1.2")),
    ("TLS1.3", ("--tlsv1.3", "--tls-max", "1.3")),
)


def test_url_protocol(url: str, timeout_sec: int, label: str, protocol_args: tuple[str, ...]) -> str:
    command = [
        "curl.exe",
        "-I",
        "-s",
        "-m",
        str(timeout_sec),
        "-o",
        "NUL",
        "-w",
        "%{http_code}",
        "--show-error",
        *protocol_args,
        url,
    ]
    try:
        completed = subprocess.run(
            command,
            capture_output=True,
            text=True,
            timeout=timeout_sec + 3,
            check=False,
            creationflags=CREATE_NO_WINDOW,
        )
    except (OSError, subprocess.TimeoutExpired):
        return f"{label}:ERR"

    stderr = completed.stderr or ""
    unsupported = completed.returncode == 35 or re.search(
        r"not supported|unsupported protocol|schannel",
        stderr,
        re.IGNORECASE,
    )
    if unsupported:
        return f"{label}:UNSUP"
    if re.search(r"certificate|SSL|self.?signed", stderr, re.IGNORECASE):
        return f"{label}:SSL"
    if completed.returncode == 0:
        return f"{label}:OK"
    return f"{label}:FAIL"


def test_url(url: str, timeout_sec: int) -> tuple[str, ...]:
    with ThreadPoolExecutor(max_workers=len(URL_TESTS)) as executor:
        futures = [
            executor.submit(test_url_protocol, url, timeout_sec, label, protocol_args)
            for label, protocol_args in URL_TESTS
        ]
        return tuple(future.result() for future in futures)


def test_ping(target: str | None, timeout_sec: int) -> tuple[bool, str]:
    if not target:
        return False, "n/a"

    try:
        completed = subprocess.run(
            ["ping.exe", "-n", "2", "-w", str(timeout_sec * 1000), target],
            capture_output=True,
            text=True,
            timeout=timeout_sec * 2 + 3,
            check=False,
            creationflags=CREATE_NO_WINDOW,
        )
    except (OSError, subprocess.TimeoutExpired):
        return False, "Timeout"

    output = (completed.stdout or "") + (completed.stderr or "")
    if completed.returncode != 0:
        return False, "Timeout"

    avg_match = re.search(r"Average\s*=\s*(\d+ms)", output, re.IGNORECASE)
    if not avg_match:
        avg_match = re.search(r"Среднее\s*=\s*(\d+мс)", output, re.IGNORECASE)
    return True, avg_match.group(1) if avg_match else "OK"


def test_target(target: Target, timeout_sec: int) -> TargetResult:
    with ThreadPoolExecutor(max_workers=2) as executor:
        url_future = executor.submit(test_url, target.url, timeout_sec) if target.url else None
        ping_future = executor.submit(test_ping, target.ping_target, timeout_sec)
        tokens = url_future.result() if url_future is not None else tuple()
        ping_success, ping_text = ping_future.result()
    return TargetResult(target.name, tokens, ping_success, ping_text)


ProgressCallback = Callable[[Target, TargetResult, int, int], None]
StopCallback = Callable[[], bool]


def test_targets(
    targets: list[Target],
    timeout_sec: int,
    progress_callback: ProgressCallback | None = None,
    stop_callback: StopCallback | None = None,
    max_workers: int = 4,
) -> PresetTestResult:
    details: list[TargetResult] = []
    ok = fail = unsup = ping_ok = ping_total = 0

    worker_count = max(1, min(max_workers, len(targets) or 1))
    future_targets: dict[Future[TargetResult], Target] = {}

    with ThreadPoolExecutor(max_workers=worker_count) as executor:
        for target in targets:
            if stop_callback is not None and stop_callback():
                break
            future_targets[executor.submit(test_target, target, timeout_sec)] = target

        completed_count = 0
        for future in as_completed(future_targets):
            target = future_targets[future]
            if stop_callback is not None and stop_callback():
                for pending in future_targets:
                    pending.cancel()
                break

            completed_count += 1
            try:
                target_result = future.result()
            except Exception:
                target_result = TargetResult(target.name, ("HTTP:ERR", "TLS1.2:ERR", "TLS1.3:ERR"), False, "ERR")

            for token in target_result.http_tokens:
                if token.endswith(":OK"):
                    ok += 1
                elif token.endswith(":UNSUP"):
                    unsup += 1
                else:
                    fail += 1

            if target.ping_target:
                ping_total += 1
                if target_result.ping_ok:
                    ping_ok += 1

            details.append(target_result)
            if progress_callback is not None:
                progress_callback(target, target_result, completed_count, len(targets))

    return PresetTestResult(ok, fail, unsup, ping_ok, ping_total, tuple(details))
