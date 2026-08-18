use anyhow::{anyhow, Result};
use crate::vrchat::auth::{get_client, send_with_retry, truncate_for_log, VRCHAT_API};
use crate::vrchat::cache;
use crate::vrchat::models::VRCAvatar;

const PAGE_SIZE: u32 = 100;
/// VRChat API rate limit safety margin (ms) between sequential requests
/// *within the same favorite group*. Cross-group calls run in parallel and
/// do not need this delay.
const API_DELAY: u64 = 150;
/// 自前アバター取得時にバックエンドで投機的に並列フェッチするページ数。
/// 通常ユーザーのアップロードは数十〜数百件なので 1〜2 バッチで終わる。
const MY_AVATARS_PARALLEL: u32 = 5;
/// アバター取得時の安全上限。1 ページ 100 件 × 100 ページ = 10,000 件まで。
/// 通常ユーザーは数十〜数百件なのでこの上限は実質的に到達しない。
/// API バグや想定外の挙動で無限ループに陥らないための保険。
const MAX_AVATAR_PAGES: u32 = 100;

/// HTTP ステータスが失敗のとき、ボディ本文を含めた anyhow エラーを返す。
/// `forbidden_msg` を渡すと 403 専用の文言を使う（編集系エンドポイント向け）。
/// select 等の閲覧系では `None` を渡し、誤誘導するメッセージを出さない。
async fn ensure_success(
    resp: reqwest::Response,
    context: &str,
    forbidden_msg: Option<&str>,
) -> Result<reqwest::Response> {
    let status = resp.status();
    if status.is_success() {
        return Ok(resp);
    }
    let text = resp.text().await.unwrap_or_default();
    if status == reqwest::StatusCode::FORBIDDEN {
        if let Some(msg) = forbidden_msg {
            return Err(anyhow!("{}: {} - {}", msg, status, truncate_for_log(&text, 500)));
        }
    }
    Err(anyhow!(
        "{}: {} - {}",
        context,
        status,
        truncate_for_log(&text, 500)
    ))
}

async fn fetch_my_avatars_page(offset: u32) -> Result<Vec<VRCAvatar>> {
    let client = get_client();
    let n_str = PAGE_SIZE.to_string();
    let offset_str = offset.to_string();
    let resp = send_with_retry(
        || {
            client
                .get(format!("{}/avatars", VRCHAT_API))
                .query(&[
                    ("user", "me"),
                    ("releaseStatus", "all"),
                    ("n", n_str.as_str()),
                    ("offset", offset_str.as_str()),
                    ("sort", "updated"),
                    ("order", "descending"),
                ])
                .send()
        },
        "GET /avatars",
    )
    .await?;
    let resp = ensure_success(resp, "アバター一覧の取得に失敗しました", None).await?;
    Ok(resp.json::<Vec<VRCAvatar>>().await?)
}

/// 自前アバターを全件取得する。最大 `MY_AVATARS_PARALLEL` ページを並列でフェッチし、
/// 100 件未満が返ったページが含まれるバッチでループを終了する。
/// 成功時にディスクキャッシュに保存し、次回起動時の SWR 即時表示に使う。
///
/// 個別ページのフェッチ失敗（JoinError / API エラー）は warn ログを残して継続する。
/// お気に入り側と挙動を統一し、1 ページの失敗で全件破棄しないことで、ユーザー体験を守る。
/// ただし「途中ページが失敗」した場合に reached_end 判定が後ろにずれる可能性があるため、
/// 失敗があった場合は安全側に倒してそのバッチで終了する（穴あきリストを返さない）。
pub async fn get_my_avatars_all() -> Result<Vec<VRCAvatar>> {
    let mut all: Vec<VRCAvatar> = Vec::new();
    let mut start_page: u32 = 0;
    loop {
        if start_page >= MAX_AVATAR_PAGES {
            log::warn!(
                "get_my_avatars_all: {} ページに達したため打ち切りました。",
                MAX_AVATAR_PAGES
            );
            break;
        }
        let batch_size = MY_AVATARS_PARALLEL.min(MAX_AVATAR_PAGES - start_page);
        let mut handles = Vec::with_capacity(batch_size as usize);
        for i in 0..batch_size {
            let offset = (start_page + i) * PAGE_SIZE;
            handles.push(tokio::spawn(fetch_my_avatars_page(offset)));
        }
        let mut reached_end = false;
        let mut had_failure = false;
        for (i, h) in handles.into_iter().enumerate() {
            match h.await {
                Ok(Ok(avatars)) => {
                    let short = (avatars.len() as u32) < PAGE_SIZE;
                    all.extend(avatars);
                    if short {
                        reached_end = true;
                    }
                }
                Ok(Err(e)) => {
                    log::warn!(
                        "自前アバター page {} の取得に失敗: {}（取得済み分は保持）",
                        start_page + i as u32,
                        e
                    );
                    had_failure = true;
                }
                Err(e) => {
                    log::warn!(
                        "自前アバター page {} のタスク結合失敗: {}（取得済み分は保持）",
                        start_page + i as u32,
                        e
                    );
                    had_failure = true;
                }
            }
        }
        if reached_end || had_failure {
            break;
        }
        start_page += batch_size;
    }

    cache::save_avatars(&all);
    Ok(all)
}

async fn fetch_favorite_group(group: &'static str) -> Result<Vec<VRCAvatar>> {
    let client = get_client();
    let mut acc: Vec<VRCAvatar> = Vec::new();
    let mut offset: u32 = 0;
    let n_str = PAGE_SIZE.to_string();
    loop {
        let offset_str = offset.to_string();
        let resp = match send_with_retry(
            || {
                client
                    .get(format!("{}/avatars/favorites", VRCHAT_API))
                    .query(&[
                        ("n", n_str.as_str()),
                        ("tag", group),
                        ("offset", offset_str.as_str()),
                    ])
                    .send()
            },
            "GET /avatars/favorites",
        )
        .await
        {
            Ok(r) => r,
            Err(e) => {
                log::warn!("お気に入りグループ {} の取得で通信エラー: {}", group, e);
                break;
            }
        };

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            log::warn!(
                "お気に入りグループ {} の取得に失敗しました: {} - {}",
                group,
                status,
                truncate_for_log(&text, 500)
            );
            break;
        }

        let avatars: Vec<VRCAvatar> = resp.json().await?;
        let count = avatars.len() as u32;

        acc.extend(avatars.into_iter().filter(|a| {
            !(a.name == "???" && a.author_name == "Unknown Author")
        }));

        if count < PAGE_SIZE {
            break;
        }
        offset += PAGE_SIZE;
        tokio::time::sleep(std::time::Duration::from_millis(API_DELAY)).await;
    }
    Ok(acc)
}

/// 6 グループを並列に取得する。同一エンドポイントへの 6 本同時リクエストは
/// VRChat API のレート制限内に収まる（経験的に 1 秒あたり数十リクエストまで許容）。
/// 各グループ内のページング（通常 1 ページで終わる）は順次のまま。
pub async fn get_favorite_avatars() -> Result<Vec<VRCAvatar>> {
    let groups: [&'static str; 6] = [
        "avatars1", "avatars2", "avatars3", "avatars4", "avatars5", "avatars6",
    ];

    let mut handles = Vec::with_capacity(groups.len());
    for group in groups {
        handles.push(tokio::spawn(fetch_favorite_group(group)));
    }

    let mut all_avatars: Vec<VRCAvatar> = Vec::new();
    for h in handles {
        match h.await {
            Ok(Ok(avatars)) => all_avatars.extend(avatars),
            Ok(Err(e)) => log::warn!("お気に入り取得失敗: {}", e),
            Err(e) => log::warn!("お気に入り取得タスクの結合に失敗: {}", e),
        }
    }

    cache::save_favorites(&all_avatars);
    Ok(all_avatars)
}

pub async fn select_avatar(avatar_id: &str) -> Result<VRCAvatar> {
    let client = get_client();
    let resp = send_with_retry(
        || {
            client
                .put(format!("{}/avatars/{}/select", VRCHAT_API, avatar_id))
                .send()
        },
        "PUT /avatars/{id}/select",
    )
    .await?;
    // select は閲覧系。403 はプライベート/BAN/権限の総合的事由なので
    // 「編集できません」とは表示せず、ステータスのみ伝える。
    let resp = ensure_success(resp, "アバターの装着に失敗しました", None).await?;
    // .json() のエラーメッセージ ("error decoding response body") だけでは
    // VRChat側が実際どんなJSONを返したのか分からず原因調査ができないため、
    // 生テキストを先に取ってから自前でパースし、失敗時は本文をログに残す
    // （SteamVRオーバーレイ経由の select で発生を確認、原因未特定）。
    let text = resp.text().await?;
    serde_json::from_str::<VRCAvatar>(&text).map_err(|e| {
        log::warn!(
            "[vrchat] select_avatar のレスポンスをパースできませんでした: {} (body: {})",
            e,
            truncate_for_log(&text, 500)
        );
        anyhow!("アバター情報の解析に失敗しました: {}", e)
    })
}

/// VRChat API でアバターの画像を更新する。
/// `files::upload_image` で取得した VRChat API プロキシ URL を imageUrl に指定する。
pub async fn update_avatar_image(avatar_id: &str, image_url: &str) -> Result<VRCAvatar> {
    let client = get_client();
    let body = serde_json::json!({ "imageUrl": image_url });
    let resp = send_with_retry(
        || {
            client
                .put(format!("{}/avatars/{}", VRCHAT_API, avatar_id))
                .json(&body)
                .send()
        },
        "PUT /avatars/{id} (imageUrl)",
    )
    .await?;
    let resp = ensure_success(
        resp,
        "アバター画像の更新に失敗しました",
        Some("このアバターは編集できません（作成者のみ変更可能です）"),
    )
    .await?;
    Ok(resp.json::<VRCAvatar>().await?)
}

/// VRChat API でアバターのメタデータ（名前など）を更新する。
/// 自分が作成したアバターのみ更新可能。
pub async fn update_avatar(avatar_id: &str, name: &str) -> Result<VRCAvatar> {
    let client = get_client();
    let body = serde_json::json!({ "name": name });
    let resp = send_with_retry(
        || {
            client
                .put(format!("{}/avatars/{}", VRCHAT_API, avatar_id))
                .json(&body)
                .send()
        },
        "PUT /avatars/{id} (name)",
    )
    .await?;
    let resp = ensure_success(
        resp,
        "アバターの更新に失敗しました",
        Some("このアバターは編集できません（作成者のみ変更可能です）"),
    )
    .await?;
    Ok(resp.json::<VRCAvatar>().await?)
}
