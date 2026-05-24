# -*- mode: python ; coding: utf-8 -*-

from pathlib import Path


ROOT = Path.cwd()
TG_WS_VENDOR = ROOT / "vendor" / "tg-ws-proxy-src"


datas = [
    ("gui/ui/assets", "gui/ui/assets"),
    ("presets", "presets"),
    ("utils/targets.txt", "utils"),
    ("exe", "exe"),
    ("lists", "lists"),
    ("bin", "bin"),
    ("lua", "lua"),
    ("windivert.filter", "windivert.filter"),
    ("docs", "docs"),
    ("vendor/tg-ws-proxy-src/LICENSE", "licenses/tg-ws-proxy"),
]


a = Analysis(
    ["gui/app.py"],
    pathex=[str(ROOT), str(ROOT / "gui"), str(TG_WS_VENDOR)],
    binaries=[],
    datas=datas,
    hiddenimports=[
        "proxy.tg_ws_proxy",
        "proxy.balancer",
        "proxy.bridge",
        "proxy.config",
        "proxy.fake_tls",
        "proxy.raw_websocket",
        "proxy.stats",
        "proxy.utils",
    ],
    hookspath=[],
    hooksconfig={},
    runtime_hooks=[],
    excludes=[],
    noarchive=False,
    optimize=0,
)
pyz = PYZ(a.pure)

exe = EXE(
    pyz,
    a.scripts,
    [],
    exclude_binaries=True,
    name="Zapret2GUI",
    debug=False,
    bootloader_ignore_signals=False,
    strip=False,
    upx=True,
    console=False,
    disable_windowed_traceback=False,
    argv_emulation=False,
    target_arch=None,
    codesign_identity=None,
    entitlements_file=None,
    manifest="Zapret2GUI.manifest",
    uac_admin=True,
    icon="gui/ui/assets/Z2.ico",
)
coll = COLLECT(
    exe,
    a.binaries,
    a.datas,
    strip=False,
    upx=True,
    upx_exclude=[],
    name="Zapret2GUI",
)
