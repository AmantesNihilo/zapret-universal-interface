from __future__ import annotations

import re
import ipaddress
import socket
import subprocess
from concurrent.futures import Future, ThreadPoolExecutor, as_completed
from collections.abc import Callable
from dataclasses import dataclass
from urllib.parse import urlparse

from .targets import Target


CREATE_NO_WINDOW = 0x08000000
BROWSER_USER_AGENT = (
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) "
    "AppleWebKit/537.36 (KHTML, like Gecko) "
    "Chrome/124.0.0.0 Safari/537.36"
)


@dataclass(frozen=True)
class TargetResult:
    name: str
    category: str
    service: str
    primary: bool
    url: str | None
    ping_target: str | None
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


def host_resolves_to_loopback(host: str) -> bool:
    try:
        infos = socket.getaddrinfo(host, 443, type=socket.SOCK_STREAM)
    except OSError:
        return False
    for info in infos:
        try:
            if ipaddress.ip_address(info[4][0]).is_loopback:
                return True
        except ValueError:
            continue
    return False


def url_resolves_to_loopback(url: str) -> bool:
    host = urlparse(url).hostname
    return bool(host and host_resolves_to_loopback(host))


def test_url_protocol(url: str, timeout_sec: int, label: str, protocol_args: tuple[str, ...]) -> str:
    command = [
        "curl.exe",
        "--noproxy",
        "*",
        "-s",
        "-L",
        "-m",
        str(timeout_sec),
        "--connect-timeout",
        str(timeout_sec),
        "--max-redirs",
        "4",
        "-A",
        BROWSER_USER_AGENT,
        "-H",
        "Accept: text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
        "-H",
        "Accept-Language: en-US,en;q=0.9",
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
    except subprocess.TimeoutExpired:
        return f"{label}:ERR_TIMEOUT"
    except OSError:
        return f"{label}:ERR_RUN"

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
    http_code_text = (completed.stdout or "").strip()
    try:
        http_code = int(http_code_text[-3:])
    except ValueError:
        http_code = 0

    if completed.returncode == 0 and 200 <= http_code < 400:
        return f"{label}:OK{http_code}"
    if completed.returncode == 0 and http_code:
        return f"{label}:HTTP{http_code}"
    error_hint = curl_error_hint(completed.returncode, stderr)
    return f"{label}:FAIL{completed.returncode}_{error_hint}"


def curl_error_hint(returncode: int, stderr: str) -> str:
    text = (stderr or "").lower()
    if returncode == 28 or "timed out" in text or "timeout" in text:
        return "TIMEOUT"
    if returncode == 35 or "ssl connect" in text or "schannel" in text:
        return "TLS"
    if returncode == 52 or "empty reply" in text:
        return "EMPTY"
    if returncode == 56 or "recv failure" in text or "connection was reset" in text:
        return "RESET"
    if returncode == 7 or "could not connect" in text:
        return "CONNECT"
    if returncode == 6 or "could not resolve" in text:
        return "DNS"
    return "ERR"


def test_url(url: str, timeout_sec: int) -> tuple[str, ...]:
    if url_resolves_to_loopback(url):
        return tuple(f"{label}:DNS_LOOPBACK" for label, _protocol_args in URL_TESTS)
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
    if target.url and url_resolves_to_loopback(target.url):
        return TargetResult(
            target.name,
            target.category,
            target.service,
            target.primary,
            target.url,
            target.ping_target,
            tuple(f"{label}:DNS_LOOPBACK" for label, _protocol_args in URL_TESTS),
            False,
            "DNS loopback",
        )

    with ThreadPoolExecutor(max_workers=2) as executor:
        url_future = executor.submit(test_url, target.url, timeout_sec) if target.url else None
        ping_future = executor.submit(test_ping, target.ping_target, timeout_sec)
        tokens = url_future.result() if url_future is not None else tuple()
        ping_success, ping_text = ping_future.result()
    return TargetResult(
        target.name,
        target.category,
        target.service,
        target.primary,
        target.url,
        target.ping_target,
        tokens,
        ping_success,
        ping_text,
    )


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
                target_result = TargetResult(
                    target.name,
                    target.category,
                    target.service,
                    target.primary,
                    target.url,
                    target.ping_target,
                    ("HTTP:ERR", "TLS1.2:ERR", "TLS1.3:ERR"),
                    False,
                    "ERR",
                )

            for token in target_result.http_tokens:
                if re.search(r":(?:OK|HTTP)\d{3}$", token):
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
