// 調査専用: OSCQuery mDNS の発見までのレイテンシを計測する。
// `_oscjson._tcp.local.` を browse し、ServiceResolved/Removed を
// タイムスタンプ付きで出力するだけの使い捨てツール。
use mdns_sd::{ServiceDaemon, ServiceEvent};
use std::time::Instant;

fn main() {
    let timeout_secs: u64 = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(300);
    let start = Instant::now();
    let mdns = ServiceDaemon::new().expect("mDNS デーモン起動失敗");
    let receiver = mdns.browse("_oscjson._tcp.local.").expect("browse 失敗");

    println!("[{:>7.3}s] browse 開始 (timeout={}s)", start.elapsed().as_secs_f64(), timeout_secs);
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(timeout_secs);

    while let Ok(event) = receiver.recv_timeout(deadline.saturating_duration_since(std::time::Instant::now())) {
        let t = start.elapsed().as_secs_f64();
        match event {
            ServiceEvent::ServiceFound(ty, name) => {
                println!("[{:>7.3}s] ServiceFound   ty={} name={}", t, ty, name);
            }
            ServiceEvent::ServiceResolved(info) => {
                println!(
                    "[{:>7.3}s] ServiceResolved name={} addrs={:?} port={}",
                    t,
                    info.get_fullname(),
                    info.get_addresses(),
                    info.get_port()
                );
            }
            ServiceEvent::ServiceRemoved(ty, name) => {
                println!("[{:>7.3}s] ServiceRemoved ty={} name={}", t, ty, name);
            }
            ServiceEvent::SearchStarted(s) => {
                println!("[{:>7.3}s] SearchStarted {}", t, s);
            }
            ServiceEvent::SearchStopped(s) => {
                println!("[{:>7.3}s] SearchStopped {}", t, s);
            }
        }
    }
    println!("[{:>7.3}s] タイムアウトまたは終了", start.elapsed().as_secs_f64());
}
