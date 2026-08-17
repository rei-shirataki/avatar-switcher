// 調査専用: register 側の構成違いで発見レイテンシがどう変わるか比較する。
// mode:
//   a = 現行実装を模倣 (COMPUTERNAME.local. + LAN IP + 127.0.0.1)
//   b = ユニークホスト名 (regtestb-<pid>.local.) + LAN IP + 127.0.0.1
//   c = loopback のみ (127.0.0.1 のみ、ホスト名もユニーク)
use mdns_sd::{ServiceDaemon, ServiceInfo};
use std::env;
use std::net::{IpAddr, Ipv4Addr, UdpSocket};

const SERVICE_TYPE: &str = "_oscjson._tcp.local.";

fn detect_local_ipv4() -> Option<Ipv4Addr> {
    let sock = UdpSocket::bind("0.0.0.0:0").ok()?;
    sock.connect("8.8.8.8:80").ok()?;
    match sock.local_addr().ok()?.ip() {
        IpAddr::V4(v4) if !v4.is_unspecified() && !v4.is_loopback() => Some(v4),
        _ => None,
    }
}

fn get_hostname() -> String {
    let raw = std::env::var("COMPUTERNAME")
        .ok()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "localhost".to_string());
    format!("{}.local.", raw)
}

fn main() {
    let mode = env::args().nth(1).unwrap_or_else(|| "a".to_string());
    let pid = std::process::id();
    let mdns = ServiceDaemon::new().expect("mDNS デーモン起動失敗");

    let (instance_name, host_name, ips): (String, String, Vec<String>) = match mode.as_str() {
        "a" => {
            let mut ips = vec![];
            if let Some(l) = detect_local_ipv4() {
                ips.push(l.to_string());
            }
            ips.push("127.0.0.1".to_string());
            (format!("RegTestA-{}", pid), get_hostname(), ips)
        }
        "b" => {
            let mut ips = vec![];
            if let Some(l) = detect_local_ipv4() {
                ips.push(l.to_string());
            }
            ips.push("127.0.0.1".to_string());
            (
                format!("RegTestB-{}", pid),
                format!("regtestb-{}.local.", pid),
                ips,
            )
        }
        "c" => (
            format!("RegTestC-{}", pid),
            format!("regtestc-{}.local.", pid),
            vec!["127.0.0.1".to_string()],
        ),
        // 実アプリの修正後ロジックをそのまま再現 (instance_name = host_name の由来、
        // 大文字混じり・実 instance_name と同一文字列であることを確認する用途)。
        "d" => {
            let instance_name = format!("AvatarSwitcher-{}", pid);
            let host_name = format!("{}.local.", instance_name);
            let mut ips = vec![];
            if let Some(l) = detect_local_ipv4() {
                ips.push(l.to_string());
            }
            ips.push("127.0.0.1".to_string());
            (instance_name, host_name, ips)
        }
        other => panic!("unknown mode: {}", other),
    };

    let ips_str = ips.join(",");
    let props: &[(String, String)] = &[("test".to_string(), mode.clone())];
    let info = ServiceInfo::new(SERVICE_TYPE, &instance_name, &host_name, ips_str.as_str(), 12345u16, props)
        .expect("ServiceInfo 作成失敗");
    println!(
        "[register_test mode={}] name={} host={} ips={}",
        mode, instance_name, host_name, ips_str
    );
    mdns.register(info).expect("register 失敗");
    println!("[register_test mode={}] registered. 90秒間待機します...", mode);
    std::thread::sleep(std::time::Duration::from_secs(90));
}
