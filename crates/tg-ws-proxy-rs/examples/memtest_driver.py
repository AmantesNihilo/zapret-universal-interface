"""Run examples/memtest against one proxy binary and report its memory.

Windows usage:
    python examples/memtest_driver.py path/to/tg-ws-proxy.exe 19080 60 0 1024
"""

from __future__ import annotations

import argparse
import ctypes
import json
import os
import subprocess
import sys
import threading
import time
from ctypes import wintypes
from pathlib import Path


class ProcessMemoryCountersEx(ctypes.Structure):
    _fields_ = [
        ("cb", wintypes.DWORD),
        ("PageFaultCount", wintypes.DWORD),
        ("PeakWorkingSetSize", ctypes.c_size_t),
        ("WorkingSetSize", ctypes.c_size_t),
        ("QuotaPeakPagedPoolUsage", ctypes.c_size_t),
        ("QuotaPagedPoolUsage", ctypes.c_size_t),
        ("QuotaPeakNonPagedPoolUsage", ctypes.c_size_t),
        ("QuotaNonPagedPoolUsage", ctypes.c_size_t),
        ("PagefileUsage", ctypes.c_size_t),
        ("PeakPagefileUsage", ctypes.c_size_t),
        ("PrivateUsage", ctypes.c_size_t),
    ]


def memory(pid: int) -> tuple[int, int]:
    if sys.platform != "win32":
        values = {}
        with open(f"/proc/{pid}/smaps_rollup", encoding="ascii") as stats:
            for line in stats:
                name, _, value = line.partition(":")
                if name in {"Rss", "Private_Clean", "Private_Dirty"}:
                    values[name] = int(value.split()[0]) * 1024
        return values["Rss"], values["Private_Clean"] + values["Private_Dirty"]

    query_information = 0x0400
    process_vm_read = 0x0010
    handle = ctypes.windll.kernel32.OpenProcess(
        query_information | process_vm_read, False, pid
    )
    if not handle:
        raise ctypes.WinError()
    try:
        counters = ProcessMemoryCountersEx()
        counters.cb = ctypes.sizeof(counters)
        ok = ctypes.windll.psapi.GetProcessMemoryInfo(
            handle, ctypes.byref(counters), counters.cb
        )
        if not ok:
            raise ctypes.WinError()
        return counters.WorkingSetSize, counters.PrivateUsage
    finally:
        ctypes.windll.kernel32.CloseHandle(handle)


def read_value(process: subprocess.Popen[str], key: str) -> str:
    assert process.stdout is not None
    prefix = f"{key}="
    while line := process.stdout.readline():
        print(line, end="")
        if line.startswith(prefix):
            return line.removeprefix(prefix).strip()
    raise RuntimeError(f"memtest ended before emitting {key}")


def stop(process: subprocess.Popen[str]) -> None:
    if process.poll() is None:
        process.terminate()
        try:
            process.wait(timeout=5)
        except subprocess.TimeoutExpired:
            process.kill()
            process.wait()


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("proxy", type=Path)
    parser.add_argument("port", type=int)
    parser.add_argument("clients", type=int)
    parser.add_argument("frames", type=int)
    parser.add_argument("frame_kib", type=int)
    parser.add_argument("--harness", type=Path)
    parser.add_argument("--settle", type=float, default=2.0)
    parser.add_argument("--pool-size", type=int, default=0)
    parser.add_argument("--direct", action="store_true")
    args = parser.parse_args()

    root = Path(__file__).resolve().parents[1]
    executable_suffix = ".exe" if sys.platform == "win32" else ""
    harness_exe = args.harness or (
        root / "target" / "release" / "examples" / f"memtest{executable_suffix}"
    )
    hidden = getattr(subprocess, "CREATE_NO_WINDOW", 0)
    harness = subprocess.Popen(
        [
            str(harness_exe),
            str(args.port),
            str(args.clients),
            str(args.frames),
            str(args.frame_kib),
            str(4 * args.pool_size if args.direct else 0),
        ],
        cwd=root,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        creationflags=hidden,
    )

    proxy: subprocess.Popen[str] | None = None
    sample_stop = threading.Event()
    sampler: threading.Thread | None = None
    try:
        connect_port = read_value(harness, "CONNECT_PORT")
        read_value(harness, "START_PROXY")
        proxy_args = [
                str(args.proxy.resolve()),
                "--port",
                str(args.port),
                "--secret",
                "0ea7201141bf2763a7dee49ba68eeb4c",
                "--outbound-proxy",
                f"http://127.0.0.1:{connect_port}",
                "--no-proxy",
                "",
                "--danger-accept-invalid-certs",
                "--pool-size",
                str(args.pool_size),
                "--quiet",
            ]
        if not args.direct:
            proxy_args.extend(["--cf-domain", "fake.local"])

        proxy = subprocess.Popen(
            proxy_args,
            cwd=root,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.PIPE,
            text=True,
            creationflags=hidden,
        )
        peak_samples: list[tuple[int, int]] = []

        def sample_until_stopped() -> None:
            assert proxy is not None
            while not sample_stop.wait(0.02):
                try:
                    peak_samples.append(memory(proxy.pid))
                except (OSError, ProcessLookupError):
                    return

        sampler = threading.Thread(target=sample_until_stopped, daemon=True)
        sampler.start()
        connected = int(read_value(harness, "CLIENTS_CONNECTED"))
        upstream_connected = int(read_value(harness, "UPSTREAM_CONNECTED"))
        expected_upstreams = connected + (4 * args.pool_size if args.direct else 0)
        minimum_upstreams = expected_upstreams if connected == 0 else connected
        if upstream_connected < minimum_upstreams:
            raise RuntimeError(
                f"only {upstream_connected} of at least {minimum_upstreams} upstream sessions connected"
            )
        pool_spares_ready = upstream_connected - connected
        transfers_done = int(read_value(harness, "TRANSFER_DONE"))
        expected_transfers = connected if args.frames > 0 else 0
        if transfers_done != expected_transfers:
            raise RuntimeError(
                f"only {transfers_done} of {expected_transfers} transfers completed"
            )
        time.sleep(args.settle)
        samples = []
        for _ in range(10):
            time.sleep(0.25)
            samples.append(memory(proxy.pid))

        working_sets, private_sizes = zip(*samples)
        peak_working_sets, peak_private_sizes = zip(*(peak_samples or samples))
        open_fds = (
            len(os.listdir(f"/proc/{proxy.pid}/fd"))
            if sys.platform != "win32"
            else None
        )
        print(
            json.dumps(
                {
                    "clients": connected,
                    "upstreams": upstream_connected,
                    "pool_spares": pool_spares_ready,
                    "transfers": transfers_done,
                    "working_set_min": min(working_sets),
                    "working_set_avg": round(sum(working_sets) / len(working_sets)),
                    "working_set_max": max(working_sets),
                    "working_set_peak": max(peak_working_sets),
                    "private_min": min(private_sizes),
                    "private_avg": round(sum(private_sizes) / len(private_sizes)),
                    "private_max": max(private_sizes),
                    "private_peak": max(peak_private_sizes),
                    "open_fds": open_fds,
                }
            )
        )
    finally:
        sample_stop.set()
        if sampler is not None:
            sampler.join(timeout=1)
        if proxy is not None:
            stop(proxy)
        stop(harness)


if __name__ == "__main__":
    main()
