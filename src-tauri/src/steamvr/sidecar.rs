//! overlay-sidecar (.NET, 別プロセス) の起動・監視・終了を管理する。
//!
//! SteamVR / .NET ランタイム未導入環境でも本体アプリの起動を妨げないよう、
//! 実行ファイルが見つからない場合は warn ログのみで正常終了する
//! （`osc::oscquery::start` 失敗時の既存フォールバックと同じ思想）。

use std::path::PathBuf;
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Mutex;
use std::time::Duration;
use tauri::{AppHandle, Manager};
use tokio::io::AsyncWriteExt;
use tokio::process::{ChildStdin, Command};

/// 現在起動中の overlay-sidecar の OS プロセスID。0 なら未起動。
static SIDECAR_PID: AtomicU32 = AtomicU32::new(0);

/// shutdown() が呼ばれたら true。監視ループはこれを見て再起動を止める。
static SHUTTING_DOWN: AtomicBool = AtomicBool::new(false);

/// 起動中のサイドカーの標準入力。サイドカーはこれが閉じられる（EOF）と正常終了する。
/// アプリ自体がクラッシュした場合も OS が閉じるため、サイドカーが孤児にならない。
static SIDECAR_STDIN: Mutex<Option<ChildStdin>> = Mutex::new(None);

/// shutdown() で標準入力を閉じてから、サイドカーが自分で終了するのを待つ上限。
/// 超えたら強制終了する。
const GRACEFUL_EXIT_TIMEOUT: Duration = Duration::from_secs(3);

const RESTART_BACKOFF_SECS: [u64; 4] = [1, 3, 10, 30];
/// この時間以上動き続けたら正常に起動できたとみなし、バックオフを最初に戻す。
/// 起動直後に落ちるたびに戻していると、クラッシュループが1秒間隔で続いてしまう。
const HEALTHY_RUN: Duration = Duration::from_secs(60);

/// サイドカー実行ファイルのパスを解決する。
/// 1. 環境変数 `AVATAR_SWITCHER_OVERLAY_SIDECAR_PATH`（開発用オーバーライド。デバッグビルドのみ）
/// 2. Tauri のリソースディレクトリ配下（本番、tauri.conf.json の bundle.resources 経由で配置）
fn resolve_sidecar_path(app: &AppHandle) -> Option<PathBuf> {
    if let Some(p) = super::dev_override_env("AVATAR_SWITCHER_OVERLAY_SIDECAR_PATH") {
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
pub(crate) async fn spawn(
    app: AppHandle,
    ws_port: u16,
    ui_port: Option<u16>,
    token: String,
    placement_mode: crate::storage::OverlayPlacementMode,
) {
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

            match launch(&path, ws_port, ui_port, &token, placement_mode, log_path.as_deref()).await {
                Ok(mut child) => {
                    let started = std::time::Instant::now();
                    SIDECAR_PID.store(child.id().unwrap_or(0), Ordering::SeqCst);
                    // tokio の Child::wait() は待機前に stdin を閉じるため、先に取り出して
                    // サイドカーの生存中は開いたままにする（SIDECAR_STDIN 参照）。
                    *lock_stdin() = child.stdin.take();
                    if SHUTTING_DOWN.load(Ordering::SeqCst) {
                        // shutdown() が上の登録より先に走っていた場合は、ここで終了させる。
                        lock_stdin().take();
                        let _ = child.start_kill();
                    }
                    let status = child.wait().await;
                    lock_stdin().take();
                    SIDECAR_PID.store(0, Ordering::SeqCst);
                    if started.elapsed() >= HEALTHY_RUN {
                        attempt = 0;
                    }
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
    ui_port: Option<u16>,
    token: &str,
    placement_mode: crate::storage::OverlayPlacementMode,
    log_path: Option<&std::path::Path>,
) -> std::io::Result<tokio::process::Child> {
    let mut cmd = Command::new(path);
    cmd.arg("--ws-port")
        .arg(ws_port.to_string())
        .arg("--placement-mode")
        .arg(placement_mode.as_cli_arg())
        .stdin(Stdio::piped())
        .kill_on_drop(true);
    if let Some(ui_port) = ui_port {
        cmd.arg("--ui-port").arg(ui_port.to_string());
    }

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

    let mut child = cmd.spawn()?;
    // トークンはコマンドラインに載せると同一ユーザーの他プロセスからプロセス一覧
    // （WMI 等）で読めるため、標準入力の1行目で渡す。
    if let Some(stdin) = child.stdin.as_mut() {
        stdin.write_all(format!("{token}\n").as_bytes()).await?;
        stdin.flush().await?;
    }
    Ok(child)
}

/// ログがこのサイズを超えていたら起動時に退避する。
const LOG_ROTATE_BYTES: u64 = 5 * 1024 * 1024;

/// `overlay-sidecar.log` が肥大していれば `.log.1` へ退避する（1 世代のみ）。
/// 追記のみだと再起動のたびに際限なく増えるため。失敗しても起動は妨げない。
fn rotate_log_if_large(path: &std::path::Path, max_bytes: u64) {
    let Ok(meta) = std::fs::metadata(path) else { return };
    if meta.len() <= max_bytes {
        return;
    }
    let mut backup = path.as_os_str().to_os_string();
    backup.push(".1");
    let _ = std::fs::remove_file(&backup);
    if let Err(e) = std::fs::rename(path, &backup) {
        log::warn!("[steamvr] ログのローテーションに失敗: {}", e);
    }
}

fn open_log_append(path: &std::path::Path) -> std::io::Result<(std::fs::File, std::fs::File)> {
    rotate_log_if_large(path, LOG_ROTATE_BYTES);
    let out = std::fs::OpenOptions::new().create(true).append(true).open(path)?;
    let err = out.try_clone()?;
    Ok((out, err))
}

fn lock_stdin() -> std::sync::MutexGuard<'static, Option<ChildStdin>> {
    SIDECAR_STDIN.lock().unwrap_or_else(|e| e.into_inner())
}

/// アプリ終了時にサイドカープロセスを確実に終了させる。
///
/// まず標準入力を閉じてサイドカー自身に後片付け（オーバーレイ破棄・CEF 終了）を
/// させ、`GRACEFUL_EXIT_TIMEOUT` 以内に終わらなければ強制終了する。
/// `app.exit()` 経由のプロセス終了は Rust の Drop（`kill_on_drop`）実行を
/// 保証しないため、強制終了は PID を直接指定して同期的に行う。
pub(crate) fn shutdown() {
    SHUTTING_DOWN.store(true, Ordering::SeqCst);
    lock_stdin().take();
    let pid = SIDECAR_PID.load(Ordering::SeqCst);
    if pid == 0 {
        return;
    }
    log::info!("[steamvr] overlay-sidecar (pid={}) を終了します", pid);
    // 監視タスク（tokio ワーカー）が終了を検知すると SIDECAR_PID を 0 に戻す。
    let deadline = std::time::Instant::now() + GRACEFUL_EXIT_TIMEOUT;
    while std::time::Instant::now() < deadline {
        if SIDECAR_PID.load(Ordering::SeqCst) != pid {
            return;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    log::warn!("[steamvr] overlay-sidecar が終了しないため強制終了します (pid={})", pid);
    force_kill(pid);
}

fn force_kill(pid: u32) {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        // PATH 解決だとインストール先などに置かれた同名ファイルを実行し得るため、
        // System32 の taskkill を絶対パスで呼ぶ。CREATE_NO_WINDOW でコンソールを出さない。
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        let system_root = std::env::var_os("SystemRoot").unwrap_or_else(|| "C:\\Windows".into());
        let taskkill = PathBuf::from(system_root).join("System32").join("taskkill.exe");
        let _ = std::process::Command::new(taskkill)
            .args(["/PID", &pid.to_string(), "/T", "/F"])
            .creation_flags(CREATE_NO_WINDOW)
            .output();
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = std::process::Command::new("/bin/kill").arg(pid.to_string()).output();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn large_log_is_rotated_and_small_log_is_kept() {
        let dir = std::env::temp_dir().join(format!("avsw-log-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let log = dir.join("overlay-sidecar.log");

        std::fs::write(&log, b"0123456789").unwrap();
        rotate_log_if_large(&log, 100);
        assert!(log.exists(), "上限以下ならそのまま");

        rotate_log_if_large(&log, 5);
        assert!(!log.exists());
        assert!(dir.join("overlay-sidecar.log.1").exists());

        std::fs::remove_dir_all(&dir).ok();
    }
}
