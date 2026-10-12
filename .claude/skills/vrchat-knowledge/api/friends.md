---
scope: api
title: VRChat REST API — Friends
source: https://vrchat.community/docs/api/
status: community
last_verified: 2026-10-09
---

# VRChat REST API — Friends

ベースURL: `https://api.vrchat.cloud/api/1`  
認証: `authCookie` (Cookie: `auth=...`)

## エンドポイント一覧

| メソッド | パス | operationId | 説明 |
|---|---|---|---|
| GET | `/auth/user/friends` | getFriends | フレンド一覧を取得 |
| DELETE | `/auth/user/friends/{userId}` | unfriend | フレンドを解除 |
| GET | `/user/{userId}/friendStatus` | getFriendStatus | フレンド状態を確認 |
| POST | `/user/{userId}/friendRequest` | friend | フレンド申請を送る |
| DELETE | `/user/{userId}/friendRequest` | deleteFriendRequest | 送信済みフレンド申請を取り消す |
| POST | `/users/{userId}/boop` | boop | Boopを送る |

## getFriends クエリパラメータ

| パラメータ | 型 | 説明 |
|---|---|---|
| `offline` | boolean | `true` でオフラインフレンドも含める |
| `n` | integer | 取得件数 |
| `offset` | integer | ページングオフセット |

## getFriendStatus レスポンス

```json
{
  "isFriend": true,
  "outgoingRequest": false,
  "incomingRequest": false
}
```

| フィールド | 型 | 説明 |
|---|---|---|
| `isFriend` | boolean | 現在フレンドかどうか |
| `outgoingRequest` | boolean | 自分から申請中かどうか |
| `incomingRequest` | boolean | 相手から申請が来ているかどうか |

## フレンド申請の受諾フロー

フレンド申請の受諾は **Notifications API** 経由で行います。

1. `GET /auth/user/notifications` で `type: friendRequest` の通知を取得
2. 通知IDを使って `PUT /auth/user/notifications/{notificationId}/accept` で承認

詳細: [notifications.md](notifications.md)

## 注意事項

- 受信したフレンド申請を削除する場合は `deleteFriendRequest` ではなく `deleteNotification`（Notifications API）を使う
- `boop` はフレンドのみ送信可能
