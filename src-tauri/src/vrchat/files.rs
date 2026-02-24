use anyhow::Result;
use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine as _;
use crate::vrchat::auth::{get_client, VRCHAT_API};

// ─── レスポンス型（最小限） ───────────────────────────────

#[derive(serde::Deserialize)]
struct FileCreateResp {
    id: String,
}

#[derive(serde::Deserialize)]
struct FileVersionResp {
    versions: Vec<FileVersion>,
}

#[derive(serde::Deserialize)]
struct FileVersion {
    version: u32,
    file: FilePartData,
    signature: FilePartData,
}

#[derive(serde::Deserialize)]
struct FilePartData {
    url: String,
}

/// base64 data URL（`data:image/png;base64,...`）を受け取り、
/// VRChat にアップロードして API プロキシ URL を返す。
pub async fn upload_image(data_url: &str) -> Result<String> {
    let (mime_type, raw_bytes) = parse_data_url(data_url)?;
    let extension = if mime_type == "image/png" { ".png" } else { ".jpg" };
    let digest = md5::compute(&raw_bytes);
    let md5_b64 = B64.encode(digest.0);
    let file_size = raw_bytes.len() as u64;
    let client = get_client();

    // 1. ファイルエンティティ作成
    let create: FileCreateResp = client
        .post(format!("{}/file", VRCHAT_API))
        .json(&serde_json::json!({
            "name": format!("avatar_image{}", extension),
            "mimeType": mime_type,
            "extension": extension,
            "tags": []
        }))
        .send()
        .await?
        .json()
        .await?;

    // 2. バージョン作成 → S3 プリサインド URL を取得
    let version_resp: FileVersionResp = client
        .post(format!("{}/file/{}", VRCHAT_API, create.id))
        .json(&serde_json::json!({
            "signatureMd5": md5_b64,
            "signatureSizeInBytes": 16,
            "fileMd5": md5_b64,
            "fileSizeInBytes": file_size
        }))
        .send()
        .await?
        .json()
        .await?;

    let ver = version_resp
        .versions
        .last()
        .ok_or_else(|| anyhow::anyhow!("バージョン情報が見つかりません"))?;
    let version_id = ver.version;
    let file_url = ver.file.url.clone();
    let sig_url = ver.signature.url.clone();

    // 3. S3 へファイルアップロード（VRChat 認証不要 → plain client）
    let s3 = reqwest::Client::new();
    s3.put(&file_url)
        .header("Content-Type", mime_type)
        .body(raw_bytes.clone())
        .send()
        .await?;

    // 4. S3 へシグネチャアップロード（MD5 の 16 生バイト）
    s3.put(&sig_url)
        .header("Content-Type", "application/x-binary")
        .body(digest.0.to_vec())
        .send()
        .await?;

    // 5-6. finish 通知
    let finish_body = serde_json::json!({
        "etags": [],
        "nextPartNumber": 0,
        "maxParts": 0
    });
    client
        .put(format!("{}/file/{}/{}/file/finish", VRCHAT_API, create.id, version_id))
        .json(&finish_body)
        .send()
        .await?;
    client
        .put(format!("{}/file/{}/{}/signature/finish", VRCHAT_API, create.id, version_id))
        .json(&finish_body)
        .send()
        .await?;

    // 7. VRChat 側の処理完了をポーリング（最大 20 秒、タイムアウトでも続行）
    poll_until_complete(&create.id, version_id).await?;

    Ok(format!("{}/file/{}/{}/file", VRCHAT_API, create.id, version_id))
}

fn parse_data_url(data_url: &str) -> Result<(&'static str, Vec<u8>)> {
    let (header, b64_data) = data_url
        .split_once(',')
        .ok_or_else(|| anyhow::anyhow!("data URL のフォーマットが不正です"))?;
    let mime: &'static str = if header.contains("image/png") {
        "image/png"
    } else {
        "image/jpeg"
    };
    let bytes = B64.decode(b64_data)?;
    Ok((mime, bytes))
}

async fn poll_until_complete(file_id: &str, version_id: u32) -> Result<()> {
    #[derive(serde::Deserialize)]
    struct PollResp {
        versions: Vec<PollVersion>,
    }
    #[derive(serde::Deserialize)]
    struct PollVersion {
        version: u32,
        file: PollFile,
    }
    #[derive(serde::Deserialize)]
    struct PollFile {
        status: String,
    }

    let client = get_client();
    for _ in 0..10u32 {
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        let resp: PollResp = match client
            .get(format!("{}/file/{}/{}", VRCHAT_API, file_id, version_id))
            .send()
            .await
        {
            Ok(r) => match r.json().await {
                Ok(p) => p,
                Err(_) => continue,
            },
            Err(_) => continue,
        };
        if resp
            .versions
            .iter()
            .any(|v| v.version == version_id && v.file.status == "complete")
        {
            return Ok(());
        }
    }
    // タイムアウトでもエラーにしない（VRChat 側の処理が遅い場合がある）
    Ok(())
}
