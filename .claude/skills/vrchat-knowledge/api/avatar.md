---
scope: api
title: VRChat REST API — Avatars
source: https://vrchat.community/docs/api/
status: community
last_verified: 2026-10-09
---

# VRChat REST API — Avatars

ベースURL: `https://api.vrchat.cloud/api/1`  
認証: `authCookie` (Cookie: `auth=...`)

## エンドポイント一覧

| メソッド | パス | operationId | 説明 |
|---|---|---|---|
| GET | `/avatars` | searchAvatars | アバター検索（自分または featured のみ） |
| POST | `/avatars` | createAvatar | アバター作成 |
| GET | `/avatars/favorites` | getFavoritedAvatars | お気に入りアバター一覧 |
| GET | `/avatars/licensed` | getLicensedAvatars | ライセンス取得済みアバター一覧 |
| GET | `/avatars/{avatarId}` | getAvatar | IDでアバター情報取得 |
| PUT | `/avatars/{avatarId}` | updateAvatar | アバター情報更新 |
| DELETE | `/avatars/{avatarId}` | deleteAvatar | アバター削除 |
| DELETE | `/avatars/{avatarId}/impostor` | deleteImpostor | 生成済みImpostorを削除 |
| POST | `/avatars/{avatarId}/impostor/enqueue` | enqueueImpostor | Impostor生成をキューに追加 |
| PUT | `/avatars/{avatarId}/select` | selectAvatar | アバターを装着 |
| PUT | `/avatars/{avatarId}/selectFallback` | selectFallbackAvatar | フォールバックアバターとして設定（対象がフォールバック用タグ付きでない場合は403。2026-08時点の仕様で `deprecated` は解除） |
| GET | `/avatarStyles` | getAvatarStyles | アバタースタイル一覧 |
| GET | `/avatars/impostor/queue/stats` | getImpostorQueueStats | Impostor生成キューの統計 |
| GET | `/users/{userId}/avatar` | getOwnAvatar | 自分の現在のアバターを取得（他ユーザー指定はエラー） |

## searchAvatars クエリパラメータ

| パラメータ | 型 | 説明 |
|---|---|---|
| `user` | string | `me` を指定すると自分のアバターのみ |
| `userId` | string | 特定ユーザーのアバターを検索 |
| `featured` | boolean | フィーチャードアバターのみ |
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

## 主要レスポンスフィールド（Avatar オブジェクト）

| フィールド | 型 | 説明 |
|---|---|---|
| `id` | string | アバターID（例: `avtr_xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx`） |
| `name` | string | アバター名 |
| `description` | string | 説明文 |
| `authorId` | string | 作成者のユーザーID |
| `authorName` | string | 作成者の表示名 |
| `imageUrl` | string | サムネイル画像URL |
| `releaseStatus` | string | 公開状態（`public`, `private`, `hidden`） |
| `tags` | string[] | タグ一覧 |
| `featured` | boolean | フィーチャードアバターかどうか |
| `unityPackages` | object[] | 対応プラットフォームのパッケージ情報 |
| `updatedAt` | string | 最終更新日時（ISO 8601） |
| `createdAt` | string | 作成日時（ISO 8601） |

## 注意事項

- `searchAvatars` は**自分のアバターまたはfeaturedアバターのみ**検索可能。他ユーザーのアバターは検索不可
- `selectAvatar` でアバターを装着すると即座に反映される
- アバター作成時にカスタムIDを指定することが可能だが、既存IDは使用不可
- `selectFallbackAvatar` を呼び出すには対象アバターがフォールバック用としてタグ付けされている必要がある
