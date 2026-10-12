---
scope: api
title: VRChat REST API — Favorites
source: https://vrchat.community/docs/api/
status: community
last_verified: 2026-10-09
---

# VRChat REST API — Favorites

ベースURL: `https://api.vrchat.cloud/api/1`  
認証: `authCookie` (Cookie: `auth=...`)

## エンドポイント一覧

| メソッド | パス | operationId | 説明 |
|---|---|---|---|
| GET | `/auth/user/favoritelimits` | getFavoriteLimits | お気に入り数の上限を取得 |
| GET | `/favorites` | getFavorites | お気に入り一覧を取得 |
| POST | `/favorites` | addFavorite | お気に入りに追加 |
| DELETE | `/favorites/{favoriteId}` | removeFavorite | お気に入りから削除 |
| GET | `/favorite/groups` | getFavoriteGroups | お気に入りグループ一覧 |
| GET | `/favorites/groups/{favoriteGroupType}` | getFavoriteGroupsByType | 指定種別のお気に入りグループ一覧（クエリ: `ownerId`） |
| GET | `/favorites/groups/{favoriteGroupType}/{favoriteGroupName}` | getFavoriteGroupContents | グループ内のお気に入りを、参照先オブジェクトとともに一覧（クエリ: `ownerId`） |
| GET | `/favorite/group/{favoriteGroupType}/{favoriteGroupName}/{userId}` | getFavoriteGroup | 特定のお気に入りグループ情報 |
| PUT | `/favorite/group/{favoriteGroupType}/{favoriteGroupName}/{userId}` | updateFavoriteGroup | お気に入りグループを更新 |
| DELETE | `/favorite/group/{favoriteGroupType}/{favoriteGroupName}/{userId}` | clearFavoriteGroup | お気に入りグループの中身を全消去 |

`getFavoriteGroups`（`GET /favorite/groups`）には、種別で絞るクエリ `type` が追加されています。

## getFavorites クエリパラメータ

| パラメータ | 型 | 説明 |
|---|---|---|
| `type` | string | お気に入りの種別（`world`, `friend`, `avatar`） |
| `tag` | string | グループタグでフィルター |
| `n` | integer | 取得件数 |
| `offset` | integer | ページングオフセット |

## お気に入りグループの種別（favoriteGroupType）

| 値 | 対象 | 説明 |
|---|---|---|
| `world` | ワールド | お気に入りワールドのグループ |
| `friend` | フレンド | お気に入りフレンドのグループ |
| `avatar` | アバター | お気に入りアバターのグループ |

## 注意事項

- 既存の `/favorite/group/...`（単数形）と新規の `/favorites/groups/...`（複数形）はパスが似ているので注意。複数形側は `userId` をパスではなく `ownerId` クエリで指定する
- お気に入りの数には上限があり、VRC+ 加入で上限が増加します（`getFavoriteLimits` で確認可）
- `clearFavoriteGroup` は対象グループの全お気に入りを一括削除します。**元に戻せません**
