//! VRChat の ID 形式検証。
//!
//! ID は URL パスに `format!` で埋め込まれるため、`../` 等を含む値を通すと
//! 認証 cookie 付きのまま意図しない API パスを叩けてしまう。入口で形式を絞る。
use anyhow::{anyhow, Result};

/// `avtr_` + UUID（8-4-4-4-12 の 16 進）のみを許可する。
pub fn validate_avatar_id(id: &str) -> Result<()> {
    let Some(uuid) = id.strip_prefix("avtr_") else {
        return Err(anyhow!("アバターIDの形式が不正です"));
    };
    let groups: Vec<&str> = uuid.split('-').collect();
    let lens = [8usize, 4, 4, 4, 12];
    let ok = groups.len() == lens.len()
        && groups
            .iter()
            .zip(lens)
            .all(|(g, n)| g.len() == n && g.chars().all(|c| c.is_ascii_hexdigit()));
    if ok {
        Ok(())
    } else {
        Err(anyhow!("アバターIDの形式が不正です"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_canonical_avatar_id() {
        assert!(validate_avatar_id("avtr_0b3f1e2a-4c5d-4e6f-8a7b-9c0d1e2f3a4b").is_ok());
    }

    #[test]
    fn rejects_path_traversal_and_separators() {
        for bad in [
            "../auth/user",
            "avtr_0b3f1e2a-4c5d-4e6f-8a7b-9c0d1e2f3a4b/../x",
            "avtr_0b3f1e2a-4c5d-4e6f-8a7b-9c0d1e2f3a4b?x=1",
            "avtr_../../logout",
        ] {
            assert!(validate_avatar_id(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn rejects_wrong_prefix_length_and_empty() {
        assert!(validate_avatar_id("").is_err());
        assert!(validate_avatar_id("wrld_0b3f1e2a-4c5d-4e6f-8a7b-9c0d1e2f3a4b").is_err());
        assert!(validate_avatar_id("avtr_0b3f1e2a-4c5d-4e6f-8a7b").is_err());
        assert!(validate_avatar_id("avtr_zb3f1e2a-4c5d-4e6f-8a7b-9c0d1e2f3a4b").is_err());
    }
}
