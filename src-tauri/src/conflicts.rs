use crate::models::ConflictProcess;
use crate::system_process;

const CONFLICT_IMAGES: &[&str] = &[
    "hiddify.exe",
    "hiddify-next.exe",
    "nekobox.exe",
    "nekoray.exe",
    "sing-box.exe",
    "sign-box.exe",
    "v2rayn.exe",
    "v2ray.exe",
    "xray.exe",
    "clash.exe",
    "clash-meta.exe",
    "clash-verge.exe",
    "clash-verge-rev.exe",
    "clash-win64.exe",
    "mihomo.exe",
    "mihomo-party.exe",
    "wireguard.exe",
    "openvpn.exe",
    "openvpn-gui.exe",
    "protonvpn.exe",
    "nordvpn.exe",
    "outline.exe",
    "shadowsocks.exe",
    "shadowsocksr-win.exe",
    "privoxy.exe",
    "hysteria.exe",
    "trojan.exe",
    "goodbyedpi.exe",
];

pub fn detect() -> Vec<ConflictProcess> {
    system_process::running_processes_by_names(CONFLICT_IMAGES)
        .into_iter()
        .map(|process| ConflictProcess {
            image: process.image,
            pid: process.pid,
            title: process.title,
        })
        .collect()
}

pub fn kill(pids: &[u32]) -> Vec<ConflictProcess> {
    for pid in pids {
        let _ = system_process::kill_pid(*pid);
    }

    detect()
        .into_iter()
        .filter(|process| pids.contains(&process.pid))
        .collect()
}
