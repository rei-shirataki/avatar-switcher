//! アバター一覧・お気に入りの SWR ディスクキャッシュ。
//!
//! 起動時の体感速度を上げるために、前回取得した結果を JSON で永続化する。
//! フロントは起動時にまずキャッシュを読んで即時表示し、裏で VRChat API へ
//! 再フェッチして差分を反映する。
//!
//! 機微情報は含まない（公開メタデータのみ）ため、平文 JSON で保存する。
//! Cookie と同じ `app_data_dir` 配下に置く。
use std::path::{Path, PathBuf};
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

fn save_to(name: &str, avatars: &[VRCAvatar]) {
    let Some(p) = path(name) else { return; };
    let Some(lock) = SAVE_LOCK.get() else { return; };
    let _guard = lock.lock().unwrap_or_else(|e| e.into_inner());
    let cache = CacheFile {
        version: CACHE_VERSION,
        avatars: avatars.to_vec(),
    };
    let Ok(buf) = serde_json::to_vec(&cache) else { return; };
    if let Err(e) = atomic_write(&p, &buf) {
        log::warn!("{}: キャッシュの書き込みに失敗しました: {}", name, e);
    }
}

pub fn load_avatars() -> Vec<VRCAvatar> { load_from(AVATARS_FILE) }
pub fn save_avatars(avatars: &[VRCAvatar]) { save_to(AVATARS_FILE, avatars) }

pub fn load_favorites() -> Vec<VRCAvatar> { load_from(FAVORITES_FILE) }
pub fn save_favorites(avatars: &[VRCAvatar]) { save_to(FAVORITES_FILE, avatars) }

/// ログアウト時にキャッシュを破棄する。
pub fn clear() {
    for name in [AVATARS_FILE, FAVORITES_FILE] {
        if let Some(p) = path(name) {
            std::fs::remove_file(&p).ok();
        }
    }
}
