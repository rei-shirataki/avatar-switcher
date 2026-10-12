---
scope: api
title: VRChat REST API — Worlds
source: https://vrchat.community/docs/api/
status: community
last_verified: 2026-10-09
---

# VRChat REST API — Worlds

ベースURL: `https://api.vrchat.cloud/api/1`  
認証: `authCookie` (Cookie: `auth=...`)

## エンドポイント一覧

| メソッド | パス | operationId | 認証 | 説明 |
|---|---|---|---|---|
| GET | `/worlds` | searchWorlds | 必須 | クエリフィルターでワールド検索 |
| POST | `/worlds` | createWorld | 必須 | ワールド作成 |
| GET | `/worlds/active` | getActiveWorlds | 必須 | アクティブなワールド一覧 |
| GET | `/worlds/favorites` | getFavoritedWorlds | 必須 | お気に入りワールド一覧 |
| GET | `/worlds/recent` | getRecentWorlds | 必須 | 最近訪問したワールド一覧 |
| GET | `/worlds/{worldId}` | getWorld | 不要 | IDでワールド情報取得 |
| PUT | `/worlds/{worldId}` | updateWorld | 必須 | ワールド情報更新 |
| DELETE | `/worlds/{worldId}` | deleteWorld | 必須 | ワールド削除（非表示化） |
| POST | `/worlds/{worldId}/addTags` | addWorldTags | 必須 | ワールドにタグを追加 |
| POST | `/worlds/{worldId}/deleteTags` | removeWorldTags | 必須 | ワールドからタグを削除（パスは `deleteTags`。ユーザー版は `removeTags` で名前が異なる） |
| GET | `/worlds/{worldId}/metadata` | getWorldMetadata | 不要 | ワールドのカスタムメタデータを取得（`updateWorld` で任意のオブジェクトを設定可能。実運用では未使用とみられる。2026-08時点の仕様で `deprecated` は解除） |
| DELETE | `/worlds/{worldId}/platform/{publishedPlatform}` | deleteWorldPlatform | 必須 | ワールドの特定プラットフォーム版を削除 |
| GET | `/worlds/{worldId}/publish` | getWorldPublishStatus | 必須 | ワールドの公開ステータスを取得 |
| PUT | `/worlds/{worldId}/publish` | publishWorld | 必須 | ワールドを公開（週1回まで） |
| DELETE | `/worlds/{worldId}/publish` | unpublishWorld | 必須 | ワールドの公開を取り消す |
| GET | `/worlds/{worldId}/{instanceId}` | getWorldInstance | 必須 | ワールドのインスタンス情報を取得 |

## searchWorlds クエリパラメータ

| パラメータ | 型 | 説明 |
|---|---|---|
| `search` | string | テキスト検索 |
| `user` | string | `me` を指定すると自分のワールドのみ |
| `userId` | string | 特定ユーザーのワールドを検索 |
| `featured` | boolean | フィーチャードワールドのみ |
| `sort` | string | 並び順 |
| `order` | string | `ascending` / `descending` |
| `n` | integer | 取得件数 |
| `offset` | integer | ページングオフセット |
| `tag` | string | タグでフィルター |
| `notag` | string | 除外タグ |
| `releaseStatus` | string | `public`, `private`, `hidden` |
| `platform` | string | プラットフォームでフィルター |
| `maxUnityVersion` | string | 最大Unityバージョン |
| `minUnityVersion` | string | 最小Unityバージョン |
| `noplatform` | string | このプラットフォームに対応しないワールドを除外 |
| `fuzzy` | boolean | （ソース仕様に説明文なし。パラメータ名のみ確認） |
| `avatarSpecific` | boolean | アバターワールドのみを検索 |

## 主要レスポンスフィールド（World オブジェクト）

| フィールド | 型 | 説明 |
|---|---|---|
| `id` | string | ワールドID（例: `wrld_xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx`） |
| `name` | string | ワールド名 |
| `description` | string | 説明文 |
| `authorId` | string | 作成者のユーザーID |
| `authorName` | string | 作成者の表示名 |
| `imageUrl` | string | サムネイル画像URL |
| `releaseStatus` | string | 公開状態（`public`, `private`, `hidden`） |
| `capacity` | integer | 最大収容人数 |
| `visits` | integer | 訪問者数 |
| `favorites` | integer | お気に入り数 |
| `tags` | string[] | タグ一覧 |
| `publicationDate` | string | 公開日時（ISO 8601） |
| `updatedAt` | string | 最終更新日時（ISO 8601） |

## updateWorld リクエストボディ

`PUT /worlds/{worldId}` のフィールドです（すべて任意）。

| フィールド | 型 | 説明 |
|---|---|---|
| `name` / `description` | string | 名称・説明 |
| `authorId` / `authorName` | string | 作者 |
| `capacity` / `recommendedCapacity` | integer | 最大／推奨収容人数 |
| `tags` | string[] | タグ（[tags.md](tags.md)） |
| `releaseStatus` | string | 公開状態 |
| `imageUrl` / `assetUrl` / `assetVersion` / `unityPackageUrl` / `unityVersion` / `platform` | string | アセット関連 |
| `previewYoutubeId` | string | null | プレビュー用 YouTube 動画ID |
| `urlList` | string[] | ワールドが使用する URL の一覧 |
| `disabledPropAbilities` | string[] | 無効化する Prop の機能 |

## 注意事項

- `getWorld` は認証なしでも動作するが、一部フィールド（訪問者数等）が `0` になる
- `deleteWorld` はワールドが完全に削除されるわけではなく `releaseStatus` が `hidden` に変更されるだけ。ワールドIDは永続的に予約される
- ワールド作成時、`assetUrl` は `.vrcw` 拡張子のファイルオブジェクト、`imageUrl` は画像ファイルオブジェクトが必要
