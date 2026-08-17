//! overlay-sidecar (.NET, 別プロセス) の起動・監視・終了を管理する。
//!
//! SteamVR / .NET ランタイム未導入環境でも本体アプリの起動を妨げないよう、
//! 実行ファイルが見つからない場合は warn ログのみで正常終了する
//! （`osc::oscquery::start` 失敗時の既存フォールバックと同じ思想）。

use std::path::PathBuf;
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::time::Duration;
use tauri::{AppHandle, Manager};
use tokio::process::Command;

/// 現在起動中の overlay-sidecar の OS プロセスID。0 なら未起動。
static SIDECAR_PID: AtomicU32 = AtomicU32::new(0);

/// shutdown() が呼ばれたら true。監視ループはこれを見て再起動を止める。
static SHUTTING_DOWN: AtomicBool = AtomicBool::new(false);

const RESTART_BACKOFF_SECS: [u64; 4] = [1, 3, 10, 30];

/// サイドカー実行ファイルのパスを解決する。
/// 1. 環境変数 `AVATAR_SWITCHER_OVERLAY_SIDECAR_PATH`（開発用オーバーライド）
/// 2. Tauri のリソースディレクトリ配下（本番、tauri.conf.json の bundle.resources 経由で配置）
fn resolve_sidecar_path(app: &AppHandle) -> Option<PathBuf> {
    if let Ok(p) = std::env::var("AVATAR_SWITCHER_OVERLAY_SIDECAR_PATH") {
        let path = PathBuf::from(p);
        if path.exists() {
            return Some(path);
        }
        log::warn!(
            "[steamvr] AVATAR_SWITCHER_OVERLAY_SIDECAR_PATH が指す先が存在しません: {}",
            path.display()
        );
    }
    let resource_path = app
        .path()
        .resource_dir()
        .ok()?
        .join("overlay-sidecar")
        .join("overlay-sidecar.exe");
    resource_path.exists().then_some(resource_path)
}

/// overlay-sidecar の stdout/stderr を書き出すログファイルのパスを返す。
/// サイドカーはGUIアプリでコンソールを持たないため、ここに書き出さないと
/// ユーザーが動作確認する手段が無くなる。
fn log_file_path(app: &AppHandle) -> Option<PathBuf> {
    let dir = app.path().app_data_dir().ok()?;
    std::fs::create_dir_all(&dir).ok()?;
    Some(dir.join("overlay-sidecar.log"))
}

/// サイドカーを起動し、異常終了時はバックオフしつつ再起動を繰り返す監視タスクを spawn する。
///
/// 実行ファイルパスの解決はループの毎回試みる（起動時1回だけではない）。
/// 開発時は「アプリを起動 → `dotnet publish` → `AVATAR_SWITCHER_OVERLAY_SIDECAR_PATH`
/// を設定」という順序になりがちで、起動時点ではまだ publish 成果物が存在しないことが
/// 多い。ここで恒久的に諦めてしまうと、後から exe が現れても一生ピックアップされない。
pub(crate) async fn spawn(app: AppHandle, ws_port: u16, token: String) {
    let log_path = log_file_path(&app);
    if let Some(ref p) = log_path {
        log::info!("[steamvr] overlay-sidecar のログ出力先: {}", p.display());
    }
    tokio::spawn(async move {
        let mut attempt = 0usize;
        let mut warned_missing = false;
        loop {
            if SHUTTING_DOWN.load(Ordering::SeqCst) {
                break;
            }

            let Some(path) = resolve_sidecar_path(&app) else {
                if !warned_missing {
                    log::warn!(
                        "[steamvr] overlay-sidecar 実行ファイルが見つかりません。見つかり次第自動的に起動します。"
                    );
                    warned_missing = true;
                }
                tokio::time::sleep(Duration::from_secs(3)).await;
                continue;
            };
            warned_missing = false;

            match launch(&path, ws_port, &token, log_path.as_deref()).await {
                Ok(mut child) => {
                    attempt = 0;
                    SIDECAR_PID.store(child.id().unwrap_or(0), Ordering::SeqCst);
                    let status = child.wait().await;
                    SIDECAR_PID.store(0, Ordering::SeqCst);
                    match status {
                        Ok(s) => log::warn!("[steamvr] overlay-sidecar が終了しました: {:?}", s),
                        Err(e) => log::warn!("[steamvr] overlay-sidecar の監視に失敗: {}", e),
                    }
                }
                Err(e) => {
                    log::warn!("[steamvr] overlay-sidecar の起動に失敗: {}", e);
                }
            }
            if SHUTTING_DOWN.load(Ordering::SeqCst) {
                break;
            }
            let backoff = RESTART_BACKOFF_SECS[attempt.min(RESTART_BACKOFF_SECS.len() - 1)];
            attempt += 1;
            tokio::time::sleep(Duration::from_secs(backoff)).await;
        }
    });
}

async fn launch(
    path: &PathBuf,
    ws_port: u16,
    token: &str,
    log_path: Option<&std::path::Path>,
) -> std::io::Result<tokio::process::Child> {
    let mut cmd = Command::new(path);
    cmd.arg("--ws-port")
        .arg(ws_port.to_string())
        .arg("--token")
        .arg(token)
        .kill_on_drop(true);

    // stdout/stderr は同じログファイルに追記する。起動のたびに個別ハンドルを
    // 開き直す（tokio::process::Command は Stdio を Clone できないため）。
    match log_path.map(open_log_append) {
        Some(Ok((out, err))) => {
            cmd.stdout(Stdio::from(out)).stderr(Stdio::from(err));
        }
        Some(Err(e)) => {
            log::warn!("[steamvr] overlay-sidecar ログファイルを開けませんでした: {}", e);
            cmd.stdout(Stdio::null()).stderr(Stdio::null());
        }
        None => {
            cmd.stdout(Stdio::null()).stderr(Stdio::null());
        }
    }

    cmd.spawn()
}

fn open_log_append(path: &std::path::Path) -> std::io::Result<(std::fs::File, std::fs::File)> {
    let out = std::fs::OpenOptions::new().create(true).append(true).open(path)?;
    let err = out.try_clone()?;
    Ok((out, err))
}

/// アプリ終了時にサイドカープロセスを確実に終了させる。
///
/// `app.exit()` 経由のプロセス終了は Rust の Drop（`kill_on_drop`）実行を
/// 保証しないため、PID を直接指定して同期的に kill する。
pub(crate) fn shutdown() {
    SHUTTING_DOWN.store(true, Ordering::SeqCst);
    let pid = SIDECAR_PID.swap(0, Ordering::SeqCst);
    if pid == 0 {
        return;
    }
    log::info!("[steamvr] overlay-sidecar (pid={}) を終了します", pid);
    #[cfg(target_os = "windows")]
    {
        let _ = std::process::Command::new("taskkill")
            .args(["/PID", &pid.to_string(), "/T", "/F"])
            .output();
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = std::process::Command::new("kill").arg(pid.to_string()).output();
    }
}
