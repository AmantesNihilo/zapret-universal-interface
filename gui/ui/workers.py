from __future__ import annotations

import time

from PySide6.QtCore import QObject, Signal, Slot

from zapret_core import runner, scoring, targets, tester
from zapret_core.paths import AppPaths
from zapret_core.presets import Preset


def sleep_interruptible(seconds: int, should_stop) -> bool:
    deadline = time.monotonic() + seconds
    while time.monotonic() < deadline:
        if should_stop():
            return True
        time.sleep(0.1)
    return should_stop()


class TestAllWorker(QObject):
    preset_started = Signal(str, int, int)
    target_finished = Signal(str, str, int, int, str, str)
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
                    score = scoring.score_result(result.ok, result.fail, result.ping_ok)
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
