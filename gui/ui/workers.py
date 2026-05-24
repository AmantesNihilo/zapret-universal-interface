from __future__ import annotations

import re
import time

from PySide6.QtCore import QObject, Signal, Slot

from zapret_core import runner, scoring, targets, tester, tg_ws, updater
from zapret_core.paths import AppPaths
from zapret_core.presets import Preset


class ServiceOperationWorker(QObject):
    finished = Signal(str, object)
    failed = Signal(str, str)

    def __init__(self, operation: str, params: dict[str, object]) -> None:
        super().__init__()
        self.operation = operation
        self.params = params

    @Slot()
    def run(self) -> None:
        try:
            if self.operation == "start_preset":
                result = self.start_preset()
            elif self.operation == "start_tg":
                result = self.start_tg()
            elif self.operation == "stop_tg":
                result = self.stop_tg()
            elif self.operation == "stop_services":
                result = self.stop_services()
            else:
                raise ValueError(f"Unknown service operation: {self.operation}")
            self.finished.emit(self.operation, result)
        except Exception as exc:
            self.failed.emit(self.operation, str(exc))

    def start_preset(self) -> dict[str, object]:
        paths = self.params["paths"]
        preset_path = self.params["preset_path"]
        preset_name = str(self.params["preset_name"])
        proc = runner.run_preset(paths, preset_path, preset_name, update_state=True)
        time.sleep(0.8)
        running = proc.poll() is None and runner.is_winws2_running()
        return {
            "running": running,
            "returncode": proc.returncode,
            "preset_name": preset_name,
        }

    def start_tg(self) -> dict[str, object]:
        paths = self.params["paths"]
        host = str(self.params["host"])
        port = int(self.params["port"])
        secret = str(self.params["secret"])
        proc = tg_ws.start(paths, host, port, secret)
        pid = int(proc.pid or 0)
        time.sleep(0.8)
        running = tg_ws.is_running(host, port, pid)
        return {
            "pid": pid,
            "running": running,
            "host": host,
            "port": port,
            "secret": secret,
        }

    def stop_tg(self) -> dict[str, object]:
        pid = int(self.params.get("pid") or 0)
        host = str(self.params["host"])
        port = int(self.params["port"])
        tg_ws.stop(pid, host, port)
        return {"stopped": True}

    def stop_services(self) -> dict[str, object]:
        stopped_tg = False
        stopped_winws = False
        if bool(self.params.get("stop_tg")):
            tg_ws.stop(
                int(self.params.get("tg_pid") or 0),
                str(self.params["tg_host"]),
                int(self.params["tg_port"]),
            )
            stopped_tg = True
        if bool(self.params.get("stop_winws")):
            runner.stop_winws2()
            stopped_winws = True
        return {
            "stopped_tg": stopped_tg,
            "stopped_winws": stopped_winws,
            "stopped_any": stopped_tg or stopped_winws,
        }


def sleep_interruptible(seconds: int, should_stop) -> bool:
    deadline = time.monotonic() + seconds
    while time.monotonic() < deadline:
        if should_stop():
            return True
        time.sleep(0.1)
    return should_stop()


def target_passed(target_result: tester.TargetResult) -> bool:
    if target_result.http_tokens:
        return any(re.search(r":(?:OK|HTTP)\d{3}$", token) for token in target_result.http_tokens)
    return target_result.ping_ok


def service_adjusted_score(result: tester.PresetTestResult) -> int:
    groups: dict[str, list[tester.TargetResult]] = {}
    for detail in result.details:
        groups.setdefault(detail.service or detail.category or "General", []).append(detail)

    passed = partial = failed = primary_failed = 0
    for service_details in groups.values():
        primary_details = [detail for detail in service_details if detail.primary]
        if primary_details:
            if all(target_passed(detail) for detail in primary_details):
                passed += 1
            else:
                failed += 1
                primary_failed += sum(1 for detail in primary_details if not target_passed(detail))
            continue

        passed_count = sum(1 for detail in service_details if target_passed(detail))
        if passed_count == len(service_details) and service_details:
            passed += 1
        elif passed_count:
            partial += 1
        else:
            failed += 1

    return (
        scoring.score_result(result.ok, result.fail, result.ping_ok)
        + passed * 100
        + partial * 25
        - failed * 80
        - primary_failed * 120
    )


class TestAllWorker(QObject):
    preset_started = Signal(str, int, int)
    target_finished = Signal(str, str, int, int, str, str)
    preset_details_finished = Signal(str, object)
    preset_finished = Signal(str, int, int, int, int, int, int)
    log = Signal(str)
    finished = Signal()

    def __init__(
        self,
        paths: AppPaths,
        preset_list: list[Preset],
        target_names: set[str] | None = None,
        timeout_sec: int = 5,
        warmup_sec: int = 4,
        target_workers: int = 4,
    ) -> None:
        super().__init__()
        self.paths = paths
        self.preset_list = preset_list
        self.target_names = target_names or set()
        self.timeout_sec = timeout_sec
        self.warmup_sec = warmup_sec
        self.target_workers = target_workers
        self._stop_requested = False

    def request_stop(self) -> None:
        self._stop_requested = True

    @Slot()
    def run(self) -> None:
        target_list = targets.load_targets(self.paths.targets_file)
        if self.target_names:
            target_list = [target for target in target_list if target.name in self.target_names]
        total = len(self.preset_list)
        self.log.emit(f"Targets loaded: {len(target_list)}")
        self.log.emit(f"Parallel target workers: {self.target_workers}")

        try:
            for index, preset in enumerate(self.preset_list, start=1):
                if self._stop_requested:
                    self.log.emit("Test stopped by user.")
                    break

                self.preset_started.emit(preset.name, index, total)
                try:
                    self.log.emit(f"[{index}/{total}] Starting winws2: {preset.name}")
                    runner.run_preset(self.paths, preset.path, preset.name, update_state=False)
                    self.log.emit(f"[{index}/{total}] Warmup {self.warmup_sec}s: {preset.name}")
                    if sleep_interruptible(self.warmup_sec, lambda: self._stop_requested):
                        self.log.emit("Test stopped by user.")
                        break

                    def on_target_done(target, target_result, target_index, target_total):
                        tokens = " ".join(target_result.http_tokens) if target_result.http_tokens else "PING"
                        ping = target_result.ping_text
                        self.target_finished.emit(
                            preset.name,
                            target.name,
                            target_index,
                            target_total,
                            tokens,
                            ping,
                        )

                    result = tester.test_targets(
                        target_list,
                        self.timeout_sec,
                        on_target_done,
                        lambda: self._stop_requested,
                        self.target_workers,
                    )
                    if self._stop_requested:
                        self.log.emit("Test stopped by user.")
                        break
                    score = service_adjusted_score(result)
                    details = [
                        {
                            "name": detail.name,
                            "category": detail.category,
                            "service": detail.service,
                            "primary": detail.primary,
                            "url": detail.url or "",
                            "ping_target": detail.ping_target or "",
                            "http_tokens": list(detail.http_tokens),
                            "ping_ok": detail.ping_ok,
                            "ping_text": detail.ping_text,
                        }
                        for detail in result.details
                    ]
                    self.preset_details_finished.emit(preset.name, details)
                    self.preset_finished.emit(
                        preset.name,
                        result.ok,
                        result.fail,
                        result.unsup,
                        result.ping_ok,
                        result.ping_total,
                        score,
                    )
                except Exception as exc:
                    self.log.emit(f"{preset.name}: {exc}")
                    self.preset_finished.emit(preset.name, 0, 1, 0, 0, len(target_list), -1)
                finally:
                    runner.stop_winws2()
        finally:
            self.finished.emit()


class UpdateCheckWorker(QObject):
    finished = Signal(object)
    failed = Signal(str)

    def __init__(self, current_version: str) -> None:
        super().__init__()
        self.current_version = current_version

    @Slot()
    def run(self) -> None:
        try:
            self.finished.emit(updater.check_for_updates(self.current_version))
        except Exception as exc:
            self.failed.emit(str(exc))
