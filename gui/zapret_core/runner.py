from __future__ import annotations

import shutil
import subprocess
from pathlib import Path

from .paths import AppPaths
from .state import write_current_preset


CREATE_NO_WINDOW = 0x08000000


def stop_winws2() -> None:
    subprocess.run(
        ["taskkill.exe", "/F", "/IM", "winws2.exe"],
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
        check=False,
        creationflags=CREATE_NO_WINDOW,
    )


def is_winws2_running() -> bool:
    completed = subprocess.run(
        ["tasklist.exe", "/FI", "IMAGENAME eq winws2.exe", "/NH"],
        capture_output=True,
        text=True,
        check=False,
        creationflags=CREATE_NO_WINDOW,
    )
    return "winws2.exe" in ((completed.stdout or "") + (completed.stderr or "")).lower()


def run_preset(
    paths: AppPaths,
    preset_path: Path,
    preset_name: str,
    update_state: bool = True,
) -> subprocess.Popen:
    if not paths.winws2_exe.exists():
        raise FileNotFoundError(f"winws2.exe not found: {paths.winws2_exe}")

    stop_winws2()
    paths.utils_dir.mkdir(parents=True, exist_ok=True)
    shutil.copy2(preset_path, paths.active_preset)
    if update_state:
        write_current_preset(paths.current_preset, preset_name)

    return subprocess.Popen(
        [str(paths.winws2_exe), r"@utils\preset-active.txt"],
        cwd=str(paths.root),
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
        creationflags=CREATE_NO_WINDOW,
    )
