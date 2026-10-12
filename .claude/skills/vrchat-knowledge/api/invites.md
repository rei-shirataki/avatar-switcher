---
scope: api
title: VRChat REST API — Invites
source: https://vrchat.community/docs/api/
status: community
last_verified: 2026-10-09
---

# VRChat REST API — Invites

ベースURL: `https://api.vrchat.cloud/api/1`  
認証: `authCookie` (Cookie: `auth=...`)

## エンドポイント一覧

| メソッド | パス | operationId | 説明 |
|---|---|---|---|
| POST | `/invite/{userId}` | inviteUser | ユーザーをインスタンスに招待 |
| POST | `/invite/{userId}/photo` | inviteUserWithPhoto | 写真付きでユーザーを招待 |
| POST | `/invite/myself/to/{worldId}:{instanceId}` | inviteMyselfTo | 自分自身をインスタンスに招待 |
| POST | `/requestInvite/{userId}` | requestInvite | ユーザーに招待を要求 |
| POST | `/requestInvite/{userId}/photo` | requestInviteWithPhoto | 写真付きで招待を要求 |
| POST | `/invite/{notificationId}/response` | respondInvite | 招待に返答 |
| POST | `/invite/{notificationId}/response/photo` | respondInviteWithPhoto | 写真付きで招待に返答 |
| GET | `/message/{userId}/{messageType}` | getInviteMessages | 招待メッセージ一覧 |
| GET | `/message/{userId}/{messageType}/{slot}` | getInviteMessage | 特定スロットの招待メッセージ |
| PUT | `/message/{userId}/{messageType}/{slot}` | updateInviteMessage | 招待メッセージを更新 |
| DELETE | `/message/{userId}/{messageType}/{slot}` | resetInviteMessage | 招待メッセージをリセット |

## messageType の値

| 値 | 説明 |
|---|---|
| `message` | 招待時に使用するメッセージ（スロット0〜11） |
| `response` | 招待返答時に使用するメッセージ（スロット0〜7） |
| `request` | 招待リクエスト時に使用するメッセージ（スロット0〜11） |
| `requestResponse` | 招待リクエスト返答時に使用するメッセージ |

## 招待フロー

```
# ユーザーをインスタンスに招待
POST /invite/{userId}
Body: { "instanceId": "wrld_xxx:12345~private(...)", "messageSlot": 0 }

# 自分がインスタンスに入りたいとき（招待を要求）
POST /requestInvite/{userId}
Body: { "messageSlot": 0 }

# 招待への返答
POST /invite/{notificationId}/response
Body: { "responseSlot": 0 }
```

## 注意事項

- 招待・招待要求の結果はそれぞれ `invite` / `requestInvite` 型の Notification として返されます
- 写真付き招待（`inviteUserWithPhoto`）では PNG バイナリを multipart/form-data で送信します
- 招待メッセージはスロット番号で管理されており、カスタムテキストを事前登録しておく必要があります
