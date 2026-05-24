from __future__ import annotations

import sys
from pathlib import Path

from zapret_core import crash_report
from zapret_core.paths import AppPaths


def add_tg_ws_vendor_path(paths: AppPaths) -> None:
    vendor_path = paths.root / "vendor" / "tg-ws-proxy-src"
    if vendor_path.exists() and str(vendor_path) not in sys.path:
        sys.path.insert(0, str(vendor_path))


def run_tg_ws_proxy_helper(paths: AppPaths) -> int:
    logger = crash_report.install(paths, native_faults=False)
    logger.debug("Starting tg-ws proxy helper")
    add_tg_ws_vendor_path(paths)
    from proxy.tg_ws_proxy import main as tg_ws_main

    tg_ws_main()
    logger.debug("tg-ws proxy helper finished")
    return 0


def center_window_on_screen(window, app) -> None:
    from PySide6.QtGui import QCursor

    screen = app.screenAt(QCursor.pos()) or app.primaryScreen()
    if screen is None:
        return

    available = screen.availableGeometry()
    frame = window.frameGeometry()
    frame.moveCenter(available.center())
    window.move(frame.topLeft())


def main() -> int:
    paths = AppPaths.from_app_file(Path(__file__))
    if "--z2-tg-ws-proxy" in sys.argv:
        logger = crash_report.install(paths, native_faults=False)
        logger.debug("Helper startup: argv=%r cwd=%s root=%s", sys.argv, Path.cwd(), paths.root)
        sys.argv.remove("--z2-tg-ws-proxy")
        return run_tg_ws_proxy_helper(paths)

    logger = crash_report.install(paths)
    logger.debug("Application startup: argv=%r cwd=%s root=%s", sys.argv, Path.cwd(), paths.root)
    logger.debug("Importing Qt modules")
    from PySide6.QtGui import QIcon
    from PySide6.QtWidgets import QApplication
    from ui.main_window import MainWindow

    crash_report.install_qt_message_handler()
    logger.debug("Creating QApplication")
    app = QApplication(sys.argv)
    app.setApplicationName("Z2 GUI")
    app.setOrganizationName("Zapret2")

    icon_path = paths.root / "gui" / "ui" / "assets" / "Z2.png"
    if icon_path.exists():
        app.setWindowIcon(QIcon(str(icon_path)))
        logger.debug("Application icon loaded: %s", icon_path)
    else:
        logger.debug("Application icon not found: %s", icon_path)

    logger.debug("Creating main window")
    window = MainWindow(paths)
    window.resize(1180, 660)
    center_window_on_screen(window, app)
    window.show()
    logger.debug("Entering Qt event loop")
    exit_code = app.exec()
    logger.debug("Qt event loop finished: exit_code=%s", exit_code)
    crash_report.close_cleanly()
    return exit_code


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Exception as exc:
        crash_report.write_report(type(exc), exc, exc.__traceback__, source="__main__")
        raise
