use anyhow::{anyhow, Result};
use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine as _;
use crate::vrchat::auth::{get_client, USER_AGENT, VRCHAT_API};

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
        anyhow!("{} レスポンス解析エラー: {} | body: {}", context, e, &text[..text.len().min(500)])
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

    let resp = s3_client
        .put(url)
        .header("Content-Type", content_type)
        .header("Content-MD5", content_md5)
        .body(body)
        .send()
        .await?;

    let status = resp.status();
    let text = resp.text().await.unwrap_or_default();

    if !status.is_success() {
        return Err(anyhow!(
            "{} failed: {} - {}",
            context,
            status,
            &text[..text.len().min(500)]
        ));
    }
    Ok(())
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
    let body = resp_json(
        client
            .post(format!("{}/file", VRCHAT_API))
            .json(&serde_json::json!({
                "name":      format!("avatar_image{}", extension),
                "mimeType":  mime_type,
                "extension": extension,
                "tags":      []
            }))
            .send()
            .await?,
        "POST /file",
    )
    .await?;

    let file_id = body["id"]
        .as_str()
        .ok_or_else(|| anyhow!("POST /file: 'id' not found in: {}", body))?
        .to_string();

    // ── 2. バージョン作成 ────────────────────────────────────
    let body = resp_json(
        client
            .post(format!("{}/file/{}", VRCHAT_API, file_id))
            .json(&serde_json::json!({
                "signatureMd5":         sig_md5_b64,
                "signatureSizeInBytes": 16,
                "fileMd5":              file_md5_b64,
                "fileSizeInBytes":      file_size
            }))
            .send()
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
        client
            .put(format!(
                "{}/file/{}/{}/file/start?partNumber=1",
                VRCHAT_API, file_id, version_id
            ))
            .send()
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
        &file_md5_b64,
        raw_bytes.clone(),
        "S3 PUT file",
    )
    .await?;

    // ── 5. ファイルアップロード完了通知（etags なし: VRCX 準拠）
    resp_json(
        client
            .put(format!("{}/file/{}/{}/file/finish", VRCHAT_API, file_id, version_id))
            .json(&serde_json::json!({
                "nextPartNumber": 0,
                "maxParts":       0
            }))
            .send()
            .await?,
        "PUT file/finish",
    )
    .await?;

    // ── 6. シグネチャアップロード用 S3 presigned URL を取得 ─
    let start = resp_json(
        client
            .put(format!(
                "{}/file/{}/{}/signature/start?partNumber=1",
                VRCHAT_API, file_id, version_id
            ))
            .send()
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
        &sig_md5_b64,
        file_digest.0.to_vec(),
        "S3 PUT signature",
    )
    .await?;

    // ── 8. シグネチャアップロード完了通知（etags なし: VRCX 準拠）
    resp_json(
        client
            .put(format!("{}/file/{}/{}/signature/finish", VRCHAT_API, file_id, version_id))
            .json(&serde_json::json!({
                "nextPartNumber": 0,
                "maxParts":       0
            }))
            .send()
            .await?,
        "PUT signature/finish",
    )
    .await?;

    // ── 9. VRChat 側処理完了をポーリング（最大 20 秒）────────
    poll_until_complete(&file_id, version_id).await?;

    Ok(format!("{}/file/{}/{}/file", VRCHAT_API, file_id, version_id))
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

async fn poll_until_complete(file_id: &str, version_id: u32) -> Result<()> {
    let client = get_client();
    for _ in 0..15u32 {
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        let body: serde_json::Value = match client
            .get(format!("{}/file/{}", VRCHAT_API, file_id))
            .send()
            .await
        {
            Ok(r)  => match r.json().await { Ok(v) => v, Err(_) => continue },
            Err(_) => continue,
        };

        let status = body["versions"]
            .as_array()
            .and_then(|vs| {
                vs.iter().find(|v| v["version"].as_u64() == Some(version_id as u64))
            })
            .and_then(|v| v["file"]["status"].as_str());

        match status {
            Some("complete") => return Ok(()),
            Some("error") => return Err(anyhow!("VRChat 側でファイルの処理中にエラーが発生しました。")),
            _ => continue,
        }
    }
    Err(anyhow!("VRChat 側でのファイル処理がタイムアウトしました。"))
}
