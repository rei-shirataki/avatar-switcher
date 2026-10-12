---
scope: websocket
title: VRChat WebSocket — Pipeline
source: https://vrchat.community/websocket/
status: community
last_verified: 2026-08-16
---

# VRChat WebSocket — Pipeline

## 概要

VRChat のリアルタイムイベントをWebSocket経由で受信できます。
フレンドのオンライン状態変化・通知・グループ変更などをプッシュ受信します。

## 接続

### エンドポイント

```
wss://pipeline.vrchat.cloud/?authToken={auth_cookie_value}
```

- `{auth_cookie_value}` は `auth` Cookieの値（`authcookie_...` 形式）
- 複数の同時接続をサポート

### 必須ヘッダー

| ヘッダー | 例 |
|---|---|
| `User-Agent` | `MyApp/1.0 contact@example.com` |

## メッセージフォーマット

```json
{
  "type": "event-type-name",
  "content": "{ \"stringified\": \"json\" }"
}
```

> **重要**: `content` フィールドはほとんどのイベントで**二重エンコード**されています。  
> `content` を受け取ったら `JSON.parse(content)` を行う必要があります。  
> 例外: `see-notification`、`hide-notification`（通知IDが直接入る）

---

## イベント一覧

### Notification イベント

| type | content | 説明 |
|---|---|---|
| `notification` | Notification オブジェクト | 招待・フレンド申請等の通知を受信 |
| `response-notification` | Notification オブジェクト | 送信済み招待への返答 |
| `see-notification` | `{ notificationId: string }` | 通知が既読になった |
| `hide-notification` | `{ notificationId: string }` | 通知が非表示になった |
| `clear-notification` | — | 全通知クリア |
| `notification-v2` | NotificationV2 オブジェクト | 新形式通知 |
| `notification-v2-update` | NotificationV2 オブジェクト | 新形式通知の更新 |
| `notification-v2-delete` | `{ id: string }` | 新形式通知の削除 |

### Friend イベント

| type | content | 説明 |
|---|---|---|
| `friend-add` | `{ userId, user }` | フレンドになった |
| `friend-delete` | `{ userId }` | フレンド解除された |
| `friend-online` | `{ userId, user, platform }` | フレンドがオンラインに |
| `friend-active` | `{ userId, user }` | フレンドがアクティブに（ワールド外） |
| `friend-offline` | `{ userId }` | フレンドがオフラインに |
| `friend-update` | `{ userId, user }` | フレンドのプロフィール更新 |
| `friend-location` | `{ userId, user, location, worldId, instance }` | フレンドのワールド移動 |

### User イベント（自分自身）

| type | content | 説明 |
|---|---|---|
| `user-update` | `{ userId, user }` | 自分のプロフィール更新 |
| `user-location` | `{ userId, location, worldId, instance }` | 自分のワールド移動 |
| `user-badge-assigned` | `{ badge: Badge }` | バッジを取得した（VRC+登録等） |
| `user-badge-unassigned` | `{ badgeId: string }` | バッジを失った（VRC+失効等） |
| `content-refresh` | `{ contentType, fileId, itemId, itemType, actionType }` | プロフィール画像等のコンテンツを追加・削除した |
| `modified-image-update` | `{ fileId, pixelSize, versionNumber, needsProcessing }` | 画像ファイルが変更された |
| `economy-update` | `{ dirtyPurchases: boolean }` | Economyと連動した操作を行った（購入等） |
| `instance-queue-joined` | `{ instanceLocation, position }` | インスタンスの参加待ちキューに入った |
| `instance-queue-ready` | `{ instanceLocation, expiry }` | キューの先頭に到達した（`expiry` は優先権が切れる日時） |

`content-refresh` の `contentType` は `avatar` / `world` / `gallery` / `icon` / `emoji` / `sticker` / `print` / `inventory` 等の値を取り、`actionType` は通常 `created` / `deleted`（`inventory` の場合のみ `add` / `delete` も観測される）。

### Group イベント

| type | content | 説明 |
|---|---|---|
| `group-joined` | `{ groupId }` | グループに参加した |
| `group-left` | `{ groupId }` | グループを退出した |
| `group-member-updated` | `{ groupId, member }` | グループメンバーが更新された |
| `group-role-updated` | `{ groupId, role }` | グループロールが更新された |

---

## location フィールドの形式

| 値 | 意味 |
|---|---|
| `""`（空文字列） | 疑似null値（該当データなし） |
| `"offline"` | VRChatクライアントを起動していない、またはPipelineに未接続 |
| `"traveling"` | インスタンス間を移動中（ワールドのダウンロード・同期中）。`"traveling:traveling"` の形式で来ることもある |
| `"private"` | 現在ログイン中のユーザーからは場所が非公開（Ask Me/取り込み中設定、Invite/Invite+/Groupインスタンス等） |
| `"{worldId}:{instanceId}"` | 特定のワールドとインスタンス |
| `"{worldId}:{instanceId}~{accessType}({userId})"` | アクセスタイプ付き |

`"traveling"` の間は `travelingToLocation` フィールドに移動先のlocation文字列が入ります。

## エラーメッセージ

WebSocket接続が正常に開いた後でも、不正な状態になった場合はエラーメッセージが送られて接続がクローズされます。

```json
{
  "err": "エラーの説明文",
  "<additional>": "エラーの文脈を示す追加プロパティ"
}
```

例:
- VRChat側の障害時: `{"err":"Error finding user {userId}"}`
- authTokenを発行したIPと異なるIPから接続した場合: `{"err":"authToken doesn't correspond with an active session","authToken":"{authToken}","ip":"{ipAddress}"}`

## 注意事項

- 接続が切れた場合は再接続が必要（自動再接続はない）
- `content` の二重デシリアライズを忘れずに
- 認証Cookieが失効していると接続が拒否される
- `User-Agent` ヘッダーを適切に設定すること
