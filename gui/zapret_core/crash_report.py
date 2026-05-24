from __future__ import annotations

import logging
import os
import platform
import sys
import threading
import traceback
import atexit
import faulthandler
from collections import deque
from datetime import datetime
from pathlib import Path
from types import TracebackType
from typing import TextIO

from .paths import AppPaths


REPORT_PREFIX = "crash-report"
LOGGER_NAME = "z2"
MAX_BUFFERED_RECORDS = 300

_logger = logging.getLogger(LOGGER_NAME)
_records: deque[str] = deque(maxlen=MAX_BUFFERED_RECORDS)
_report_dir: Path | None = None
_installed = False
_previous_excepthook = sys.excepthook
_previous_threading_excepthook = getattr(threading, "excepthook", None)
_previous_unraisablehook = getattr(sys, "unraisablehook", None)
_fault_log_path: Path | None = None
_fault_log_file: TextIO | None = None


class _RecentLogHandler(logging.Handler):
    def emit(self, record: logging.LogRecord) -> None:
        try:
            formatted = self.format(record)
            _records.append(formatted)
            _write_fault_log_line(f"Log: {formatted}")
        except Exception:
            pass


def install(paths: AppPaths, *, native_faults: bool = True) -> logging.Logger:
    global _installed, _report_dir

    _report_dir = _writable_report_dir(paths)
    _logger.setLevel(logging.DEBUG)
    _logger.propagate = False

    if not _installed:
        _cleanup_empty_native_reports(_report_dir)
        if native_faults:
            _install_fault_handler()

        formatter = logging.Formatter("%(asctime)s.%(msecs)03d %(levelname)s %(name)s: %(message)s", "%H:%M:%S")

        recent_handler = _RecentLogHandler()
        recent_handler.setLevel(logging.DEBUG)
        recent_handler.setFormatter(formatter)
        _logger.addHandler(recent_handler)

        stream = _console_stream()
        if stream is not None:
            console_handler = logging.StreamHandler(stream)
            console_handler.setLevel(logging.DEBUG)
            console_handler.setFormatter(formatter)
            _logger.addHandler(console_handler)

        sys.excepthook = _handle_exception
        if hasattr(threading, "excepthook"):
            threading.excepthook = _handle_thread_exception  # type: ignore[assignment]
        if hasattr(sys, "unraisablehook"):
            sys.unraisablehook = _handle_unraisable  # type: ignore[assignment]

        _installed = True

    _logger.debug("Crash reporter initialized: report_dir=%s", _report_dir)
    return _logger


def install_qt_message_handler() -> None:
    try:
        from PySide6.QtCore import qInstallMessageHandler
    except Exception as exc:
        _logger.debug("Qt message handler was not installed: %s", exc)
        return

    def qt_message_handler(mode, context, message: str) -> None:
        level = _qt_level(mode)
        location = ""
        file = getattr(context, "file", None)
        line = getattr(context, "line", 0)
        function = getattr(context, "function", None)
        if file:
            location = f" ({file}:{line}"
            if function:
                location += f", {function}"
            location += ")"
        _logger.log(level, "Qt: %s%s", message, location)

    qInstallMessageHandler(qt_message_handler)
    _logger.debug("Qt message handler installed")


def close_cleanly() -> None:
    _delete_fault_log()


def write_report(
    exc_type: type[BaseException],
    exc_value: BaseException,
    exc_traceback: TracebackType | None,
    *,
    source: str = "sys.excepthook",
) -> Path | None:
    report_dir = _report_dir or Path.cwd()
    try:
        _delete_fault_log()
        report_dir.mkdir(parents=True, exist_ok=True)
        report_path = report_dir / f"{REPORT_PREFIX}-{datetime.now():%Y%m%d-%H%M%S-%f}.log"
        report_path.write_text(
            _format_report(exc_type, exc_value, exc_traceback, source),
            encoding="utf-8",
        )
        _logger.error("Crash report written: %s", report_path)
        return report_path
    except Exception as report_exc:
        _write_console_line(f"Failed to write crash report: {report_exc}")
        return None


def _handle_exception(
    exc_type: type[BaseException],
    exc_value: BaseException,
    exc_traceback: TracebackType | None,
) -> None:
    if issubclass(exc_type, KeyboardInterrupt):
        _previous_excepthook(exc_type, exc_value, exc_traceback)
        return

    write_report(exc_type, exc_value, exc_traceback)
    _previous_excepthook(exc_type, exc_value, exc_traceback)


def _handle_thread_exception(args: threading.ExceptHookArgs) -> None:
    write_report(args.exc_type, args.exc_value, args.exc_traceback, source=f"threading.excepthook:{args.thread.name}")
    if _previous_threading_excepthook is not None:
        _previous_threading_excepthook(args)


def _handle_unraisable(args) -> None:
    exc_type = type(args.exc_value)
    source = f"sys.unraisablehook:{args.object!r}"
    write_report(exc_type, args.exc_value, args.exc_traceback, source=source)
    if _previous_unraisablehook is not None:
        _previous_unraisablehook(args)


def _install_fault_handler() -> None:
    global _fault_log_path, _fault_log_file

    if _report_dir is None:
        return

    _recover_previous_fault_log(_report_dir)
    try:
        _report_dir.mkdir(parents=True, exist_ok=True)
        _fault_log_path = _report_dir / f"{REPORT_PREFIX}-{datetime.now():%Y%m%d-%H%M%S-%f}.log"
        _fault_log_file = _fault_log_path.open("w", encoding="utf-8")
        _fault_log_file.write(_format_fault_header())
        _fault_log_file.flush()
        faulthandler.enable(file=_fault_log_file, all_threads=True)
        atexit.register(_delete_fault_log)
    except Exception as exc:
        _write_console_line(f"Failed to install native crash handler: {exc}")


def _recover_previous_fault_log(report_dir: Path) -> None:
    active_log = report_dir / f"{REPORT_PREFIX}-active.log"
    if not active_log.exists():
        return
    try:
        if active_log.stat().st_size <= 0:
            active_log.unlink()
            return
        recovered_log = report_dir / f"{REPORT_PREFIX}-{datetime.now():%Y%m%d-%H%M%S-%f}.log"
        active_log.replace(recovered_log)
    except OSError:
        pass


def _cleanup_empty_native_reports(report_dir: Path | None) -> None:
    if report_dir is None:
        return
    for report_path in report_dir.glob(f"{REPORT_PREFIX}-*.log"):
        try:
            text = report_path.read_text(encoding="utf-8", errors="replace")
        except OSError:
            continue
        if text.startswith("Z2 GUI native crash report") and "--z2-tg-ws-proxy" in text:
            try:
                report_path.unlink()
            except OSError:
                pass


def _delete_fault_log() -> None:
    global _fault_log_file

    try:
        if faulthandler.is_enabled():
            faulthandler.disable()
    except Exception:
        pass

    if _fault_log_file is not None:
        try:
            _fault_log_file.close()
        except Exception:
            pass
        _fault_log_file = None

    if _fault_log_path is not None:
        try:
            _fault_log_path.unlink(missing_ok=True)
        except OSError:
            pass


def _write_fault_log_line(message: str) -> None:
    if _fault_log_file is None:
        return
    try:
        _fault_log_file.write(message + "\n")
        _fault_log_file.flush()
    except Exception:
        pass


def _format_report(
    exc_type: type[BaseException],
    exc_value: BaseException,
    exc_traceback: TracebackType | None,
    source: str,
) -> str:
    lines = [
        "Z2 GUI crash report",
        f"Created: {datetime.now().isoformat(timespec='seconds')}",
        f"Source: {source}",
        "",
        "Environment",
        f"  Python: {sys.version.replace(os.linesep, ' ')}",
        f"  Executable: {sys.executable}",
        f"  Frozen: {getattr(sys, 'frozen', False)}",
        f"  Platform: {platform.platform()}",
        f"  CWD: {Path.cwd()}",
        f"  PID: {os.getpid()}",
        f"  argv: {sys.argv!r}",
        "",
        "Recent log records",
    ]
    if _records:
        lines.extend(f"  {record}" for record in _records)
    else:
        lines.append("  <empty>")
    lines.extend(
        [
            "",
            "Traceback",
            "".join(traceback.format_exception(exc_type, exc_value, exc_traceback)).rstrip(),
            "",
        ]
    )
    return "\n".join(lines)


def _format_fault_header() -> str:
    return "\n".join(
        [
            "Z2 GUI native crash report",
            f"Started: {datetime.now().isoformat(timespec='seconds')}",
            "",
            "This file is removed on normal exit. If it remains, the process likely crashed or was terminated.",
            "",
            "Environment",
            f"  Python: {sys.version.replace(os.linesep, ' ')}",
            f"  Executable: {sys.executable}",
            f"  Frozen: {getattr(sys, 'frozen', False)}",
            f"  Platform: {platform.platform()}",
            f"  CWD: {Path.cwd()}",
            f"  PID: {os.getpid()}",
            f"  argv: {sys.argv!r}",
            "",
            "Fatal traceback",
            "",
        ]
    )


def _writable_report_dir(paths: AppPaths) -> Path:
    if getattr(sys, "frozen", False):
        return Path(sys.executable).resolve().parent / "utils"
    return paths.utils_dir


def _console_stream() -> TextIO | None:
    for stream in (sys.stderr, sys.stdout):
        if stream is not None and not getattr(stream, "closed", False):
            try:
                stream.write("")
                stream.flush()
                return stream
            except Exception:
                continue
    return None


def _write_console_line(message: str) -> None:
    stream = _console_stream()
    if stream is not None:
        try:
            stream.write(message + "\n")
            stream.flush()
        except Exception:
            pass


def _qt_level(mode) -> int:
    name = str(mode).lower()
    if "fatal" in name or "critical" in name:
        return logging.CRITICAL
    if "warning" in name:
        return logging.WARNING
    if "info" in name:
        return logging.INFO
    return logging.DEBUG
