//! アバター一覧・お気に入りの SWR ディスクキャッシュ。
//!
//! 起動時の体感速度を上げるために、前回取得した結果を JSON で永続化する。
//! フロントは起動時にまずキャッシュを読んで即時表示し、裏で VRChat API へ
//! 再フェッチして差分を反映する。
//!
//! 機微情報は含まない（公開メタデータのみ）ため、平文 JSON で保存する。
//! Cookie と同じ `app_data_dir` 配下に置く。
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};
use crate::vrchat::models::VRCAvatar;

const AVATARS_FILE: &str = "avatars_cache.json";
const FAVORITES_FILE: &str = "favorites_cache.json";
/// 将来のスキーマ変更時に互換性を切るためのバージョン。
/// インクリメントすると旧キャッシュは無視されて再フェッチが走る。
const CACHE_VERSION: u32 = 1;

static CACHE_DIR: OnceCell<PathBuf> = OnceCell::new();
/// 並列の save 呼び出しが file write を競合させないための排他ロック。
/// 書き込みしか使わないので RwLock ではなく Mutex を採用。
static SAVE_LOCK: OnceCell<Mutex<()>> = OnceCell::new();
/// `clear()` のたびに増える世代番号。取得開始時の世代を `save_*` に渡し、
/// 取得中にログアウト（clear）された場合は前のアカウントのデータを書き戻さない。
static EPOCH: AtomicU64 = AtomicU64::new(0);

#[derive(Serialize, Deserialize)]
struct CacheFile {
    version: u32,
    avatars: Vec<VRCAvatar>,
}

pub fn init(app_data_dir: PathBuf) {
    let _ = CACHE_DIR.set(app_data_dir);
    let _ = SAVE_LOCK.set(Mutex::new(()));
}

fn path(name: &str) -> Option<PathBuf> {
    CACHE_DIR.get().map(|d| d.join(name))
}

fn load_from(name: &str) -> Vec<VRCAvatar> {
    let Some(p) = path(name) else { return Vec::new(); };
    if !p.exists() {
        return Vec::new();
    }
    let Ok(bytes) = std::fs::read(&p) else { return Vec::new(); };
    match serde_json::from_slice::<CacheFile>(&bytes) {
        Ok(cache) if cache.version == CACHE_VERSION => cache.avatars,
        Ok(_) => {
            log::info!("{}: キャッシュバージョン不一致のため無視します。", name);
            Vec::new()
        }
        Err(e) => {
            log::warn!("{}: キャッシュの読み込みに失敗しました: {}", name, e);
            Vec::new()
        }
    }
}

/// `path` に対して `bytes` をアトミックに書き込む。
/// 同じディレクトリの一時ファイルに書いてから rename することで、
/// 書き込み途中のクラッシュでも本体ファイルが破損しない。
/// `vrchat::auth` の Cookie 永続化からも共有して使う（`pub(crate)`）。
///
/// 既存ファイルを事前に削除しない: std::fs::rename は Windows でも
/// MoveFileExW(REPLACE_EXISTING) で上書きでき、削除→rename の間でクラッシュすると
/// 本体ファイルが失われるため。一時ファイル名は呼び出しごとに一意にし、
/// 並行呼び出し同士で衝突しないようにする。
pub(crate) fn atomic_write(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    static SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let mut tmp = path.as_os_str().to_os_string();
    tmp.push(format!(
        ".{}.{}.tmp",
        std::process::id(),
        SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    let tmp = PathBuf::from(tmp);
    std::fs::write(&tmp, bytes)?;
    std::fs::rename(&tmp, path).map_err(|e| {
        std::fs::remove_file(&tmp).ok();
        e
    })
}

fn save_to(name: &str, epoch: u64, avatars: &[VRCAvatar]) {
    let Some(p) = path(name) else { return; };
    let Some(lock) = SAVE_LOCK.get() else { return; };
    let _guard = lock.lock().unwrap_or_else(|e| e.into_inner());
    // clear() と同じロックの中で世代を確認するので、clear() 後に書き戻すことはない。
    if EPOCH.load(Ordering::SeqCst) != epoch {
        log::info!("{}: 取得中にキャッシュが破棄されたため保存しません。", name);
        return;
    }
    let cache = CacheFile {
        version: CACHE_VERSION,
        avatars: avatars.to_vec(),
    };
    let Ok(buf) = serde_json::to_vec(&cache) else { return; };
    if let Err(e) = atomic_write(&p, &buf) {
        log::warn!("{}: キャッシュの書き込みに失敗しました: {}", name, e);
    }
}

/// 現在の世代。取得を始める前に読み、結果を `save_*` に渡す。
pub fn epoch() -> u64 { EPOCH.load(Ordering::SeqCst) }

pub fn load_avatars() -> Vec<VRCAvatar> { load_from(AVATARS_FILE) }
pub fn save_avatars(epoch: u64, avatars: &[VRCAvatar]) { save_to(AVATARS_FILE, epoch, avatars) }

pub fn load_favorites() -> Vec<VRCAvatar> { load_from(FAVORITES_FILE) }
pub fn save_favorites(epoch: u64, avatars: &[VRCAvatar]) { save_to(FAVORITES_FILE, epoch, avatars) }

/// ログアウト時にキャッシュを破棄する。世代を進めるので、実行中の取得が
/// 完了しても前のアカウントのデータは保存されない。
pub fn clear() {
    let _guard = SAVE_LOCK.get().map(|l| l.lock().unwrap_or_else(|e| e.into_inner()));
    EPOCH.fetch_add(1, Ordering::SeqCst);
    for name in [AVATARS_FILE, FAVORITES_FILE] {
        if let Some(p) = path(name) {
            std::fs::remove_file(&p).ok();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn save_started_before_clear_is_discarded() {
        let dir = std::env::temp_dir().join(format!("avsw-cache-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        init(dir.clone());
        let file = dir.join(AVATARS_FILE);

        let epoch_before = epoch();
        save_avatars(epoch_before, &[]);
        assert!(file.exists(), "同じ世代なら保存される");

        clear();
        assert!(!file.exists());
        // ログアウト前に始まった取得が後から完了しても書き戻さない。
        save_avatars(epoch_before, &[]);
        assert!(!file.exists(), "clear 前の世代の結果が保存された");

        save_avatars(epoch(), &[]);
        assert!(file.exists(), "clear 後に始めた取得は保存される");

        std::fs::remove_dir_all(&dir).ok();
    }
}
