from __future__ import annotations

import csv
import io
import subprocess
from dataclasses import dataclass


CREATE_NO_WINDOW = 0x08000000


CONFLICT_PROCESS_NAMES = {
    "happ.exe",
    "hiddify.exe",
    "nekobox.exe",
    "nekoray.exe",
    "v2rayn.exe",
    "v2ray.exe",
    "xray.exe",
    "clash.exe",
    "clash-verge.exe",
    "clash verge.exe",
    "clash-meta.exe",
    "sing-box.exe",
    "singbox.exe",
    "mihomo.exe",
    "wireguard.exe",
    "openvpn.exe",
    "openvpn-gui.exe",
    "outline.exe",
    "shadowsocks.exe",
    "shadowsocksr.exe",
    "sstap.exe",
    "proxifier.exe",
}

CONFLICT_SERVICE_HINTS = {
    "happ",
    "hiddify",
    "nekobox",
    "nekoray",
    "v2ray",
    "xray",
    "clash",
    "mihomo",
    "sing-box",
    "wireguard",
    "openvpn",
    "outline",
    "shadowsocks",
}

CONFLICT_ADAPTER_HINTS = {
    "happ",
    "sing-tun",
    "wintun",
    "wireguard",
    "openvpn",
    "tap-windows",
    "tailscale",
    "outline",
    "clash",
    "mihomo",
    "nekobox",
    "nekoray",
    "v2ray",
    "tun",
}


@dataclass(frozen=True)
class ConflictProcess:
    image_name: str
    pid: int
    session_name: str = ""
    memory: str = ""
    kind: str = "process"
    service_name: str = ""
    adapter_name: str = ""

    @property
    def display_name(self) -> str:
        if self.kind == "service":
            return f"Service: {self.image_name} ({self.service_name or 'unknown'})"
        if self.kind == "adapter":
            return f"Adapter: {self.image_name}"
        details = f"PID {self.pid}"
        if self.memory:
            details += f", {self.memory}"
        return f"{self.image_name} ({details})"


def find_process_conflicts() -> list[ConflictProcess]:
    completed = subprocess.run(
        ["tasklist.exe", "/FO", "CSV", "/NH"],
        capture_output=True,
        text=True,
        check=False,
        creationflags=CREATE_NO_WINDOW,
    )
    output = completed.stdout or ""
    found: list[ConflictProcess] = []

    for row in csv.reader(io.StringIO(output)):
        if len(row) < 2:
            continue
        image_name = row[0].strip()
        image_key = image_name.casefold()
        if image_key not in CONFLICT_PROCESS_NAMES:
            continue
        try:
            pid = int(row[1])
        except ValueError:
            continue
        found.append(
            ConflictProcess(
                image_name=image_name,
                pid=pid,
                session_name=row[2].strip() if len(row) > 2 else "",
                memory=row[4].strip() if len(row) > 4 else "",
            )
        )

    return sorted(found, key=lambda process: (process.image_name.casefold(), process.pid))


def _run_powershell_csv(script: str) -> list[list[str]]:
    completed = subprocess.run(
        ["powershell.exe", "-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", script],
        capture_output=True,
        text=True,
        check=False,
        creationflags=CREATE_NO_WINDOW,
    )
    output = completed.stdout or ""
    return list(csv.reader(io.StringIO(output)))


def find_service_conflicts() -> list[ConflictProcess]:
    hints = ",".join(f"'{hint}'" for hint in sorted(CONFLICT_SERVICE_HINTS))
    rows = _run_powershell_csv(
        "$hints=@("
        + hints
        + "); "
        "Get-Service | Where-Object { "
        "  if ($_.Status -ne 'Running') { return $false }; "
        "  $text=($_.Name + ' ' + $_.DisplayName).ToLowerInvariant(); "
        "  foreach ($hint in $hints) { if ($text -like \"*$hint*\") { return $true } }; "
        "  return $false "
        "} | Select-Object Name,DisplayName,Status | ConvertTo-Csv -NoTypeInformation"
    )
    found: list[ConflictProcess] = []
    for row in rows[1:]:
        if len(row) < 2:
            continue
        found.append(
            ConflictProcess(
                image_name=row[1].strip() or row[0].strip(),
                pid=0,
                kind="service",
                service_name=row[0].strip(),
            )
        )
    return found


def find_adapter_conflicts() -> list[ConflictProcess]:
    hints = ",".join(f"'{hint}'" for hint in sorted(CONFLICT_ADAPTER_HINTS))
    rows = _run_powershell_csv(
        "$hints=@("
        + hints
        + "); "
        "Get-NetAdapter | Where-Object { "
        "  if ($_.Status -ne 'Up') { return $false }; "
        "  $text=($_.Name + ' ' + $_.InterfaceDescription).ToLowerInvariant(); "
        "  foreach ($hint in $hints) { if ($text -like \"*$hint*\") { return $true } }; "
        "  return $false "
        "} | Select-Object Name,InterfaceDescription,Status | ConvertTo-Csv -NoTypeInformation"
    )
    found: list[ConflictProcess] = []
    for row in rows[1:]:
        if len(row) < 2:
            continue
        name = row[0].strip()
        description = row[1].strip()
        found.append(
            ConflictProcess(
                image_name=f"{name} - {description}" if description else name,
                pid=0,
                kind="adapter",
                adapter_name=name,
            )
        )
    return found


def find_conflicts() -> list[ConflictProcess]:
    found = [
        *find_process_conflicts(),
        *find_service_conflicts(),
        *find_adapter_conflicts(),
    ]
    return sorted(found, key=lambda item: (item.kind, item.image_name.casefold(), item.pid))


def kill_processes(processes: list[ConflictProcess]) -> None:
    for process in processes:
        if process.kind == "service" and process.service_name:
            subprocess.run(
                [
                    "powershell.exe",
                    "-NoProfile",
                    "-ExecutionPolicy",
                    "Bypass",
                    "-Command",
                    f"Stop-Service -Name '{process.service_name.replace("'", "''")}' -Force -ErrorAction SilentlyContinue",
                ],
                stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
                check=False,
                creationflags=CREATE_NO_WINDOW,
            )
            continue
        if process.kind == "adapter" and process.adapter_name:
            subprocess.run(
                [
                    "powershell.exe",
                    "-NoProfile",
                    "-ExecutionPolicy",
                    "Bypass",
                    "-Command",
                    f"Disable-NetAdapter -Name '{process.adapter_name.replace("'", "''")}' -Confirm:$false -ErrorAction SilentlyContinue",
                ],
                stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
                check=False,
                creationflags=CREATE_NO_WINDOW,
            )
            continue
        if process.pid:
            subprocess.run(
                ["taskkill.exe", "/F", "/PID", str(process.pid)],
                stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
                check=False,
                creationflags=CREATE_NO_WINDOW,
            )


def flush_dns_cache() -> None:
    subprocess.run(
        ["ipconfig.exe", "/flushdns"],
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
        check=False,
        creationflags=CREATE_NO_WINDOW,
    )
