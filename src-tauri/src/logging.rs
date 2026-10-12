//! Rust 側のログ出力。
//!
//! リリースビルドは `windows_subsystem = "windows"` でコンソールを持たないため、
//! stderr だけに出すとログがすべて失われる。起動直後は stderr にのみ出し、
//! `attach_file` 以降は `app_data_dir/avatar-switcher.log` にも追記する。

use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::sync::{Mutex, OnceLock};

static LOG_FILE: OnceLock<Mutex<File>> = OnceLock::new();

/// stderr と（あれば）ログファイルの両方へ書く。
struct TeeWriter;

impl Write for TeeWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let _ = std::io::stderr().write_all(buf);
        if let Some(file) = LOG_FILE.get() {
            let mut file = file.lock().unwrap_or_else(|e| e.into_inner());
            let _ = file.write_all(buf);
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        let _ = std::io::stderr().flush();
        if let Some(file) = LOG_FILE.get() {
            let _ = file.lock().unwrap_or_else(|e| e.into_inner()).flush();
        }
        Ok(())
    }
}

/// ロガーを初期化する。RUST_LOG 未設定時は info レベルで出す。
pub(crate) fn init() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .target(env_logger::Target::Pipe(Box::new(TeeWriter)))
        .init();
}

/// 以降のログを `dir/avatar-switcher.log` にも追記する。失敗してもアプリは続行する。
pub(crate) fn attach_file(dir: &Path) {
    let path = dir.join("avatar-switcher.log");
    rotate_log_if_large(&path, LOG_ROTATE_BYTES);
    match std::fs::OpenOptions::new().create(true).append(true).open(&path) {
        Ok(file) => {
            let _ = LOG_FILE.set(Mutex::new(file));
            log::info!("ログ出力先: {}", path.display());
        }
        Err(e) => log::warn!("ログファイルを開けませんでした ({}): {}", path.display(), e),
    }
}

/// ログがこのサイズを超えていたら起動時に退避する。
pub(crate) const LOG_ROTATE_BYTES: u64 = 5 * 1024 * 1024;

/// ログファイル（`overlay-sidecar.log` / `avatar-switcher.log`）が肥大していれば `.log.1` へ退避する（1 世代のみ）。
/// 追記のみだと再起動のたびに際限なく増えるため。失敗しても起動は妨げない。
pub(crate) fn rotate_log_if_large(path: &std::path::Path, max_bytes: u64) {
    let Ok(meta) = std::fs::metadata(path) else { return };
    if meta.len() <= max_bytes {
        return;
    }
    let mut backup = path.as_os_str().to_os_string();
    backup.push(".1");
    let _ = std::fs::remove_file(&backup);
    if let Err(e) = std::fs::rename(path, &backup) {
        eprintln!("ログのローテーションに失敗: {e}");
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
