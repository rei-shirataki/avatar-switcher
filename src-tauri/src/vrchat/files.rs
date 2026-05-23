use anyhow::{anyhow, Result};
use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine as _;
use crate::vrchat::auth::{get_client, send_with_retry, truncate_for_log, USER_AGENT, VRCHAT_API};

// ─── ヘルパー ──────────────────────────────────────────────────

/// HTTP ステータスを確認し、エラー時はレスポンス本文をメッセージに含める。
async fn resp_json(resp: reqwest::Response, context: &str) -> Result<serde_json::Value> {
    let status = resp.status();
    let text = resp.text().await? .trim().to_string();
    if !status.is_success() {
        let msg = if text.is_empty() {
            format!("{} に失敗しました: {}", context, status)
        } else {
            format!("{} に失敗しました: {} - {}", context, status, text)
        };
        return Err(anyhow!(msg));
    }
    serde_json::from_str(&text).map_err(|e| {
        anyhow!("{} レスポンス解析エラー: {} | body: {}", context, e, truncate_for_log(&text, 500))
    })
}

/// S3 presigned URL に PUT する専用クライアント。
/// VRChat 認証クッキーを送らないよう独立クライアントを使用し、
/// Content-MD5 を必ず付与して VRChat 側の整合性検証を通過させる。
async fn s3_put(
    url: &str,
    content_type: &str,
    content_md5: &str,
    body: Vec<u8>,
    context: &str,
) -> Result<()> {
    let s3_client = reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .build()
        .map_err(|e| anyhow!("S3 client build error: {}", e))?;

    let resp = send_with_retry(
        || {
            s3_client
                .put(url)
                .header("Content-Type", content_type)
                .header("Content-MD5", content_md5)
                .body(body.clone())
                .send()
        },
        context,
    )
    .await?;

    let status = resp.status();
    let text = resp.text().await.unwrap_or_default();

    if !status.is_success() {
        return Err(anyhow!(
            "{} failed: {} - {}",
            context,
            status,
            truncate_for_log(&text, 500)
        ));
    }
    Ok(())
}

/// 失敗時に VRChat 側に残ったゴミ file エンティティを削除する（ベストエフォート）。
/// ネットワーク失敗や権限不足で削除できなくても元のエラーを優先したいので、
/// 結果はログのみに残す。
///
/// upload 失敗が 429 連鎖中に起きた場合、cleanup も同様に 429 を返しやすいので
/// `send_with_retry` 経由で再試行を試みる。
async fn cleanup_file(file_id: &str) {
    let client = get_client();
    let url = format!("{}/file/{}", VRCHAT_API, file_id);
    match send_with_retry(|| client.delete(&url).send(), "DELETE /file/{id} (cleanup)").await {
        Ok(r) if r.status().is_success() => {
            log::info!("アップロード失敗のため file {} を削除しました。", file_id);
        }
        Ok(r) => {
            log::warn!("file {} の cleanup に失敗しました: {}", file_id, r.status());
        }
        Err(e) => {
            log::warn!("file {} の cleanup で通信エラー: {}", file_id, e);
        }
    }
}

// ─── メイン処理 ────────────────────────────────────────────────

/// base64 data URL（`data:image/jpeg;base64,...`）を VRChat にアップロードし、
/// API プロキシ URL を返す。
///
/// VRChat File Upload API フロー (VRCX 実装準拠):
///   1. POST /file              → file_id 取得
///   2. POST /file/{id}         → version_id 取得
///   3. PUT  /file/{id}/{v}/file/start?partNumber=1 → S3 presigned URL 取得
///   4. PUT  S3 URL (Content-MD5 付き)
///   5. PUT  /file/{id}/{v}/file/finish { nextPartNumber:0, maxParts:0 }
///   6. PUT  /file/{id}/{v}/signature/start?partNumber=1 → S3 presigned URL 取得
///   7. PUT  S3 URL (application/x-rsync-signature, Content-MD5 付き)
///   8. PUT  /file/{id}/{v}/signature/finish { nextPartNumber:0, maxParts:0 }
///   9. Poll GET /file/{id} until status=="complete"
pub async fn upload_image(data_url: &str) -> Result<String> {
    let (mime_type, raw_bytes) = parse_data_url(data_url)?;
    let extension = if mime_type == "image/png" { ".png" } else { ".jpg" };

    // MD5 計算
    let file_digest  = md5::compute(&raw_bytes);
    let file_md5_b64 = B64.encode(file_digest.0);      // base64(MD5(file))
    let sig_digest   = md5::compute(file_digest.0);    // MD5(MD5(file))
    let sig_md5_b64  = B64.encode(sig_digest.0);       // base64(MD5(MD5(file)))
    let file_size    = raw_bytes.len() as u64;

    let client = get_client();

    // ── 1. ファイルエンティティ作成 ──────────────────────────
    let create_body = serde_json::json!({
        "name":      format!("avatar_image{}", extension),
        "mimeType":  mime_type,
        "extension": extension,
        "tags":      []
    });
    let body = resp_json(
        send_with_retry(
            || {
                client
                    .post(format!("{}/file", VRCHAT_API))
                    .json(&create_body)
                    .send()
            },
            "POST /file",
        )
        .await?,
        "POST /file",
    )
    .await?;

    let file_id = body["id"]
        .as_str()
        .ok_or_else(|| anyhow!("POST /file: 'id' not found in: {}", body))?
        .to_string();

    // file_id 取得後の失敗は VRChat 側にゴミを残すので必ず cleanup する。
    let result = upload_after_create(
        &file_id,
        mime_type,
        &file_md5_b64,
        &sig_md5_b64,
        file_size,
        raw_bytes,
        file_digest.0,
    )
    .await;

    match result {
        Ok(version_id) => Ok(format!("{}/file/{}/{}/file", VRCHAT_API, file_id, version_id)),
        Err(e) => {
            cleanup_file(&file_id).await;
            Err(e)
        }
    }
}

/// ステップ 2 以降を担当。`file_id` 取得後の失敗時に呼び出し側で cleanup させるため、
/// 独立した関数に分離している。
async fn upload_after_create(
    file_id: &str,
    mime_type: &'static str,
    file_md5_b64: &str,
    sig_md5_b64: &str,
    file_size: u64,
    raw_bytes: Vec<u8>,
    file_digest: [u8; 16],
) -> Result<u32> {
    let client = get_client();

    // ── 2. バージョン作成 ────────────────────────────────────
    let version_body = serde_json::json!({
        "signatureMd5":         sig_md5_b64,
        "signatureSizeInBytes": 16,
        "fileMd5":              file_md5_b64,
        "fileSizeInBytes":      file_size
    });
    let body = resp_json(
        send_with_retry(
            || {
                client
                    .post(format!("{}/file/{}", VRCHAT_API, file_id))
                    .json(&version_body)
                    .send()
            },
            "POST /file/{id}",
        )
        .await?,
        "POST /file/{id}",
    )
    .await?;

    let versions = body["versions"]
        .as_array()
        .ok_or_else(|| anyhow!("POST /file/{{id}}: 'versions' not found in: {}", body))?;
    let ver = versions
        .last()
        .ok_or_else(|| anyhow!("POST /file/{{id}}: 'versions' array is empty"))?;
    let version_id = ver["version"]
        .as_u64()
        .ok_or_else(|| anyhow!("POST /file/{{id}}: 'version' not found in: {}", ver))? as u32;

    // ── 3. ファイルアップロード用 S3 presigned URL を取得 ────
    let start = resp_json(
        send_with_retry(
            || {
                client
                    .put(format!(
                        "{}/file/{}/{}/file/start?partNumber=1",
                        VRCHAT_API, file_id, version_id
                    ))
                    .send()
            },
            "PUT file/start",
        )
        .await?,
        "PUT file/start",
    )
    .await?;
    let file_upload_url = start["url"]
        .as_str()
        .ok_or_else(|| anyhow!("PUT file/start: 'url' not found in: {}", start))?
        .to_string();

    // ── 4. S3 へファイルをアップロード（Content-MD5 付き）──
    s3_put(
        &file_upload_url,
        mime_type,
        file_md5_b64,
        raw_bytes,
        "S3 PUT file",
    )
    .await?;

    // ── 5. ファイルアップロード完了通知（etags なし: VRCX 準拠）
    let finish_body = serde_json::json!({
        "nextPartNumber": 0,
        "maxParts":       0
    });
    resp_json(
        send_with_retry(
            || {
                client
                    .put(format!(
                        "{}/file/{}/{}/file/finish",
                        VRCHAT_API, file_id, version_id
                    ))
                    .json(&finish_body)
                    .send()
            },
            "PUT file/finish",
        )
        .await?,
        "PUT file/finish",
    )
    .await?;

    // ── 6. シグネチャアップロード用 S3 presigned URL を取得 ─
    let start = resp_json(
        send_with_retry(
            || {
                client
                    .put(format!(
                        "{}/file/{}/{}/signature/start?partNumber=1",
                        VRCHAT_API, file_id, version_id
                    ))
                    .send()
            },
            "PUT signature/start",
        )
        .await?,
        "PUT signature/start",
    )
    .await?;
    let sig_upload_url = start["url"]
        .as_str()
        .ok_or_else(|| anyhow!("PUT signature/start: 'url' not found in: {}", start))?
        .to_string();

    // ── 7. S3 へシグネチャをアップロード ────────────────────
    // VRCX 準拠: MIME = application/x-rsync-signature, Content-MD5 付き
    s3_put(
        &sig_upload_url,
        "application/x-rsync-signature",
        sig_md5_b64,
        file_digest.to_vec(),
        "S3 PUT signature",
    )
    .await?;

    // ── 8. シグネチャアップロード完了通知（etags なし: VRCX 準拠）
    resp_json(
        send_with_retry(
            || {
                client
                    .put(format!(
                        "{}/file/{}/{}/signature/finish",
                        VRCHAT_API, file_id, version_id
                    ))
                    .json(&finish_body)
                    .send()
            },
            "PUT signature/finish",
        )
        .await?,
        "PUT signature/finish",
    )
    .await?;

    // ── 9. VRChat 側処理完了をポーリング（指数バックオフ、合計 ~60 秒）─
    poll_until_complete(file_id, version_id).await?;

    Ok(version_id)
}

// ─── ユーティリティ ────────────────────────────────────────────

fn parse_data_url(data_url: &str) -> Result<(&'static str, Vec<u8>)> {
    let (header, b64_data) = data_url
        .split_once(',')
        .ok_or_else(|| anyhow!("data URL のフォーマットが不正です"))?;
    let mime: &'static str = if header.contains("image/png") { "image/png" } else { "image/jpeg" };
    let raw = B64.decode(b64_data)?;

    // Verify binary magic bytes so a mismatched or non-image payload is rejected
    // before it reaches the VRChat upload API.
    match mime {
        "image/png" => {
            if !raw.starts_with(b"\x89PNG\r\n\x1a\n") {
                return Err(anyhow!("PNG マジックナンバーが不正です"));
            }
        }
        _ => {
            // JPEG: first three bytes must be FF D8 FF
            if !raw.starts_with(&[0xFF, 0xD8, 0xFF]) {
                return Err(anyhow!("JPEG マジックナンバーが不正です"));
            }
        }
    }

    Ok((mime, raw))
}

/// VRChat 側でファイル処理が `complete` になるまで指数バックオフでポーリング。
///
/// 旧実装は 2 秒固定 × 15 回 (=30 秒) だった。サイズの大きい画像や VRChat 側の
/// バックエンド遅延に対して取りこぼしが発生していた。`error` ステータスは即座に
/// 中断、それ以外は 1s → 2s → 4s → 8s → 16s → 30s の指数バックオフで最大 ~60 秒待つ。
///
/// 401/403/404 を受け取った場合（Cookie 失効・権限喪失・file 削除）は再試行せず
/// 即座に Err を返し、ユーザーをタイムアウトの 60 秒待ちから救う。
async fn poll_until_complete(file_id: &str, version_id: u32) -> Result<()> {
    let client = get_client();
    let mut delay_ms: u64 = 1000;
    let mut elapsed_ms: u64 = 0;
    const TIMEOUT_MS: u64 = 60_000;

    loop {
        tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
        elapsed_ms = elapsed_ms.saturating_add(delay_ms);

        let resp = match client
            .get(format!("{}/file/{}", VRCHAT_API, file_id))
            .send()
            .await
        {
            Ok(r) => r,
            Err(_) => {
                if elapsed_ms >= TIMEOUT_MS {
                    break;
                }
                delay_ms = (delay_ms * 2).min(30_000);
                continue;
            }
        };

        let http_status = resp.status();
        if http_status == reqwest::StatusCode::UNAUTHORIZED
            || http_status == reqwest::StatusCode::FORBIDDEN
            || http_status == reqwest::StatusCode::NOT_FOUND
        {
            return Err(anyhow!(
                "ファイル状態の取得に失敗しました（再試行不可）: {}",
                http_status
            ));
        }
        // 5xx・429 等は一過性とみなして再試行。本ループのバックオフで吸収する。
        if !http_status.is_success() {
            if elapsed_ms >= TIMEOUT_MS {
                break;
            }
            delay_ms = (delay_ms * 2).min(30_000);
            continue;
        }

        let body: serde_json::Value = match resp.json().await {
            Ok(v) => v,
            Err(_) => {
                if elapsed_ms >= TIMEOUT_MS {
                    break;
                }
                delay_ms = (delay_ms * 2).min(30_000);
                continue;
            }
        };

        let status = body["versions"]
            .as_array()
            .and_then(|vs| vs.iter().find(|v| v["version"].as_u64() == Some(version_id as u64)))
            .and_then(|v| v["file"]["status"].as_str());

        match status {
            Some("complete") => return Ok(()),
            Some("error") => {
                return Err(anyhow!("VRChat 側でファイルの処理中にエラーが発生しました。"))
            }
            _ => {
                if elapsed_ms >= TIMEOUT_MS {
                    break;
                }
                delay_ms = (delay_ms * 2).min(30_000);
            }
        }
    }
    Err(anyhow!("VRChat 側でのファイル処理がタイムアウトしました。"))
}
