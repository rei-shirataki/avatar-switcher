---
scope: api
title: VRChat REST API — Notifications
source: https://vrchat.community/docs/api/
status: community
last_verified: 2026-10-09
---

# VRChat REST API — Notifications

ベースURL: `https://api.vrchat.cloud/api/1`  
認証: `authCookie` (Cookie: `auth=...`)

## エンドポイント一覧

### 旧形式（v1）

| メソッド | パス | operationId | 説明 |
|---|---|---|---|
| GET | `/auth/user/notifications` | getNotifications | 通知一覧を取得 |
| GET | `/auth/user/notifications/{notificationId}` | getNotification | 特定の通知を取得 |
| PUT | `/auth/user/notifications/{notificationId}/accept` | acceptFriendRequest | フレンド申請を承認 |
| PUT | `/auth/user/notifications/{notificationId}/see` | markNotificationAsRead | 通知を既読にする |
| PUT | `/auth/user/notifications/{notificationId}/hide` | deleteNotification | 通知を削除 |
| PUT | `/auth/user/notifications/clear` | clearNotifications | 全通知を削除 |

### 新形式（v2）

| メソッド | パス | operationId | 説明 |
|---|---|---|---|
| GET | `/notifications` | getNotificationV2s | NotificationV2一覧を取得 |
| DELETE | `/notifications` | deleteAllNotificationV2s | 全NotificationV2を削除 |
| GET | `/notifications/{notificationId}` | getNotificationV2 | **非推奨**。特定NotificationV2を取得（通常ユーザーは403） |
| DELETE | `/notifications/{notificationId}` | deleteNotificationV2 | 特定NotificationV2を削除 |
| POST | `/notifications/{notificationId}/reply` | replyNotificationV2 | NotificationV2に返信 |
| POST | `/notifications/{notificationId}/respond` | respondNotificationV2 | NotificationV2に応答 |
| POST | `/notifications/{notificationId}/see` | acknowledgeNotificationV2 | NotificationV2を既読にする |

## getNotifications クエリパラメータ

| パラメータ | 型 | 説明 |
|---|---|---|
| `hidden` | boolean | `true` で非表示通知も取得（`friendRequest` タイプのみ） |
| `after` | string | この日時以降の通知のみ取得（例: `"five_minutes_ago"`） |
| `n` | integer | 取得件数 |
| `offset` | integer | ページングオフセット |
| `type` | string | **非推奨**。`all` 等（現在は機能しない） |
| `sent` | boolean | **非推奨**。送信済み通知を取得（現在は機能しない） |

## Notification オブジェクト（v1）

| フィールド | 型 | 説明 |
|---|---|---|
| `id` | string | 通知ID（`not_...` または `frq_...`） |
| `type` | string | 通知種別（下記参照） |
| `senderUserId` | string | 送信者のユーザーID |
| `senderUsername` | string | 送信者の表示名 |
| `receiverUserId` | string | 受信者のユーザーID |
| `message` | string | 通知メッセージ |
| `details` | object | 種別依存の追加データ |
| `seen` | boolean | 既読かどうか |
| `created_at` | string | 作成日時（ISO 8601） |

## 通知の type 一覧

| type | 説明 | 承認操作 |
|---|---|---|
| `friendRequest` | フレンド申請 | `PUT /accept` で承認 |
| `invite` | ワールドへの招待 | `PUT /accept` で参加 |
| `requestInvite` | 招待のリクエスト | `PUT /accept` で送信 |
| `inviteResponse` | 招待への返答 | — |
| `requestInviteResponse` | 招待リクエストへの返答 | — |
| `votetokick` | キックへの投票 | — |

## フレンド申請の承認フロー

```
1. GET /auth/user/notifications?hidden=false
   → type=friendRequest の通知を探す（IDは frq_... 形式）

2. PUT /auth/user/notifications/{frq_...}/accept
   → フレンド成立
```

受信した申請を**拒否/削除**する場合:
```
PUT /auth/user/notifications/{frq_...}/hide
```

## 注意事項

- フレンド申請の承認は Notifications API 経由。Friends API の `deleteFriendRequest` は**送信した**申請を取り消す用
- `getNotificationV2`（`GET /notifications/{id}`）は非推奨かつ通常ユーザーでは 403 が返る（管理者権限が必要）
