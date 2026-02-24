use anyhow::Result;
use crate::vrchat::auth::{get_client, VRCHAT_API};
use crate::vrchat::models::VRCAvatar;

pub async fn get_my_avatars(offset: u32) -> Result<Vec<VRCAvatar>> {
    let client = get_client();
    let resp = client
        .get(format!("{}/avatars", VRCHAT_API))
        .query(&[
            ("user", "me"),
            ("releaseStatus", "all"),
            ("n", "100"),
            ("offset", &offset.to_string()),
            ("sort", "updated"),
            ("order", "descending"),
        ])
        .send()
        .await?;

    if !resp.status().is_success() {
        return Ok(vec![]);
    }

    let avatars: Vec<VRCAvatar> = resp.json().await?;
    Ok(avatars)
}

pub async fn get_favorite_avatars() -> Result<Vec<VRCAvatar>> {
    let client = get_client();
    let groups = ["avatars1", "avatars2", "avatars3", "avatars4", "avatars5", "avatars6"];
    let mut all_avatars: Vec<VRCAvatar> = Vec::new();

    for (i, group) in groups.iter().enumerate() {
        // VRChat API のレート制限対策: リクエスト間に 100ms のディレイを入れる
        if i > 0 {
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }

        let mut offset = 0u32;
        loop {
            let resp = client
                .get(format!("{}/avatars/favorites", VRCHAT_API))
                .query(&[
                    ("n", "100"),
                    ("tag", group),
                    ("offset", &offset.to_string()),
                ])
                .send()
                .await?;

            if !resp.status().is_success() {
                break;
            }

            let avatars: Vec<VRCAvatar> = resp.json().await?;
            let count = avatars.len();
            all_avatars.extend(avatars.into_iter().filter(|a| {
                !(a.name == "???" && a.author_name == "Unknown Author")
            }));

            if count < 100 {
                break;
            }
            offset += 100;
            // ページをまたぐ場合もディレイを挟む
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
    }

    Ok(all_avatars)
}

pub async fn select_avatar(avatar_id: &str) -> Result<VRCAvatar> {
    let client = get_client();
    let resp = client
        .put(format!("{}/avatars/{}/select", VRCHAT_API, avatar_id))
        .send()
        .await?;

    if !resp.status().is_success() {
        return Err(anyhow::anyhow!("Failed to select avatar: {}", resp.status()));
    }

    let avatar: VRCAvatar = resp.json().await?;
    Ok(avatar)
}

/// VRChat API でアバターの画像を更新する。
/// `files::upload_image` で取得した VRChat API プロキシ URL を imageUrl に指定する。
pub async fn update_avatar_image(avatar_id: &str, image_url: &str) -> Result<VRCAvatar> {
    let client = get_client();
    let resp = client
        .put(format!("{}/avatars/{}", VRCHAT_API, avatar_id))
        .json(&serde_json::json!({ "imageUrl": image_url }))
        .send()
        .await?;

    if resp.status() == reqwest::StatusCode::FORBIDDEN {
        return Err(anyhow::anyhow!(
            "このアバターは編集できません（作成者のみ変更可能です）"
        ));
    }
    if !resp.status().is_success() {
        return Err(anyhow::anyhow!(
            "アバター画像の更新に失敗しました: {}",
            resp.status()
        ));
    }

    Ok(resp.json::<VRCAvatar>().await?)
}

/// VRChat API でアバターのメタデータ（名前など）を更新する。
/// 自分が作成したアバターのみ更新可能。
pub async fn update_avatar(avatar_id: &str, name: &str) -> Result<VRCAvatar> {
    let client = get_client();
    let body = serde_json::json!({ "name": name });
    let resp = client
        .put(format!("{}/avatars/{}", VRCHAT_API, avatar_id))
        .json(&body)
        .send()
        .await?;

    if resp.status() == reqwest::StatusCode::FORBIDDEN {
        return Err(anyhow::anyhow!(
            "このアバターは編集できません（作成者のみ変更可能です）"
        ));
    }
    if !resp.status().is_success() {
        return Err(anyhow::anyhow!(
            "アバターの更新に失敗しました: {}",
            resp.status()
        ));
    }

    Ok(resp.json::<VRCAvatar>().await?)
}
