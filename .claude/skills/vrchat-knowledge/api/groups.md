---
scope: api
title: VRChat REST API — Groups
source: https://vrchat.community/docs/api/
status: community
last_verified: 2026-10-09
---

# VRChat REST API — Groups

ベースURL: `https://api.vrchat.cloud/api/1`  
認証: `authCookie` (Cookie: `auth=...`)

> **グループ作成には VRC+ サブスクリプションが必要です。**

## エンドポイント一覧

### グループ基本操作

| メソッド | パス | operationId | 説明 |
|---|---|---|---|
| GET | `/groups` | searchGroups | グループ名またはshortCodeで検索 |
| POST | `/groups` | createGroup | グループを作成（VRC+必須） |
| GET | `/groups/{groupId}` | getGroup | グループ情報を取得 |
| PUT | `/groups/{groupId}` | updateGroup | グループ情報を更新 |
| DELETE | `/groups/{groupId}` | deleteGroup | グループを削除 |
| GET | `/groups/roleTemplates` | getGroupRoleTemplates | ロールテンプレート一覧 |

### メンバー管理

| メソッド | パス | operationId | 説明 |
|---|---|---|---|
| GET | `/groups/{groupId}/members` | getGroupMembers | メンバー一覧 |
| GET | `/groups/{groupId}/members/search` | searchGroupMembers | メンバーを検索 |
| GET | `/groups/{groupId}/members/{userId}` | getGroupMember | 特定メンバーの情報 |
| PUT | `/groups/{groupId}/members/{userId}` | updateGroupMember | メンバー情報を更新 |
| DELETE | `/groups/{groupId}/members/{userId}` | kickGroupMember | メンバーをキック |
| PUT | `/groups/{groupId}/members/{userId}/roles/{groupRoleId}` | addGroupMemberRole | メンバーにロールを付与 |
| DELETE | `/groups/{groupId}/members/{userId}/roles/{groupRoleId}` | removeGroupMemberRole | メンバーからロールを削除 |

### 参加・招待・申請

| メソッド | パス | operationId | 説明 |
|---|---|---|---|
| POST | `/groups/{groupId}/join` | joinGroup | グループに参加 |
| POST | `/groups/{groupId}/leave` | leaveGroup | グループを退出 |
| GET | `/groups/{groupId}/invites` | getGroupInvites | 招待済み一覧 |
| POST | `/groups/{groupId}/invites` | createGroupInvite | ユーザーを招待 |
| DELETE | `/groups/{groupId}/invites/{userId}` | deleteGroupInvite | 招待を取り消し |
| PUT | `/groups/{groupId}/invites` | declineGroupInvite | 招待を辞退 |
| GET | `/groups/{groupId}/requests` | getGroupRequests | 参加申請一覧 |
| DELETE | `/groups/{groupId}/requests` | cancelGroupRequest | 参加申請をキャンセル |
| PUT | `/groups/{groupId}/requests/{userId}` | respondGroupJoinRequest | 参加申請を承認/拒否 |

### BAN管理

| メソッド | パス | operationId | 説明 |
|---|---|---|---|
| GET | `/groups/{groupId}/bans` | getGroupBans | BAN済みユーザー一覧 |
| POST | `/groups/{groupId}/bans` | banGroupMember | ユーザーをBAN |
| DELETE | `/groups/{groupId}/bans/{userId}` | unbanGroupMember | BANを解除 |
| POST | `/groups/{groupId}/block` | blockGroup | グループをブロック |

### ロール管理

| メソッド | パス | operationId | 説明 |
|---|---|---|---|
| GET | `/groups/{groupId}/roles` | getGroupRoles | ロール一覧 |
| POST | `/groups/{groupId}/roles` | createGroupRole | ロールを作成 |
| PUT | `/groups/{groupId}/roles/{groupRoleId}` | updateGroupRole | ロールを更新 |
| DELETE | `/groups/{groupId}/roles/{groupRoleId}` | deleteGroupRole | ロールを削除 |
| GET | `/groups/{groupId}/permissions` | getGroupPermissions | パーミッション一覧 |

### アナウンス・投稿

| メソッド | パス | operationId | 説明 |
|---|---|---|---|
| GET | `/groups/{groupId}/announcement` | getGroupAnnouncements | アナウンスを取得 |
| POST | `/groups/{groupId}/announcement` | createGroupAnnouncement | アナウンスを作成（既存は削除される） |
| DELETE | `/groups/{groupId}/announcement` | deleteGroupAnnouncement | アナウンスを削除 |
| GET | `/groups/{groupId}/posts` | getGroupPosts | 投稿一覧 |
| POST | `/groups/{groupId}/posts` | addGroupPost | 投稿を作成 |
| PUT | `/groups/{groupId}/posts/{notificationId}` | updateGroupPost | 投稿を編集 |
| DELETE | `/groups/{groupId}/posts/{notificationId}` | deleteGroupPost | 投稿を削除 |

### ギャラリー

| メソッド | パス | operationId | 説明 |
|---|---|---|---|
| POST | `/groups/{groupId}/galleries` | createGroupGallery | ギャラリーを作成 |
| PUT | `/groups/{groupId}/galleries/{groupGalleryId}` | updateGroupGallery | ギャラリーを更新 |
| DELETE | `/groups/{groupId}/galleries/{groupGalleryId}` | deleteGroupGallery | ギャラリーを削除 |
| GET | `/groups/{groupId}/galleries/{groupGalleryId}` | getGroupGalleryImages | ギャラリー画像一覧 |
| POST | `/groups/{groupId}/galleries/{groupGalleryId}/images` | addGroupGalleryImage | 画像を追加 |
| DELETE | `/groups/{groupId}/galleries/{groupGalleryId}/images/{groupGalleryImageId}` | deleteGroupGalleryImage | 画像を削除 |

### その他

| メソッド | パス | operationId | 説明 |
|---|---|---|---|
| GET | `/groups/{groupId}/instances` | getGroupInstances | グループインスタンス一覧 |
| GET | `/groups/{groupId}/auditLogs` | getGroupAuditLogs | 監査ログ一覧 |
| GET | `/groups/{groupId}/auditLogTypes` | getGroupAuditLogEntryTypes | 監査ログ種別一覧 |
| PUT | `/groups/{groupId}/representation` | updateGroupRepresentation | 代表グループを変更 |
| GET | `/groups/{groupId}/transfer` | getGroupTransferability | 譲渡可能かを確認 |
| POST | `/groups/{groupId}/transfer` | initiateOrAcceptGroupTransfer | グループ譲渡を開始/承認 |
| DELETE | `/groups/{groupId}/transfer` | cancelGroupTransfer | グループ譲渡をキャンセル |

## 追加されたクエリパラメータ（2026-08以降の仕様）

| operationId | パラメータ | 説明 |
|---|---|---|
| getGroup | `purpose` (string) | 用途の指定（説明は仕様に未記載） |
| getGroupGalleryImages | `v` (integer) | レスポンスバージョン。`2` を指定すると画像をページネーション付きオブジェクトで包んで返す |

`searchGroups`（`GET /groups`）は、OpenAPI 仕様に `authCookie` 必須が明示されました（以前は `security` の記載がありませんでした）。

## searchGroups クエリパラメータ

| パラメータ | 型 | 説明 |
|---|---|---|
| `query` | string | グループ名またはshortCodeで検索 |
| `n` | integer | 取得件数 |
| `offset` | integer | ページングオフセット |

## Group オブジェクトの主要フィールド

| フィールド | 型 | 説明 |
|---|---|---|
| `id` | string | グループID（`grp_...`） |
| `name` | string | グループ名 |
| `shortCode` | string | ショートコード（URLなどに使用） |
| `description` | string | 説明文 |
| `ownerId` | string | オーナーのユーザーID |
| `memberCount` | integer | メンバー数 |
| `membershipStatus` | string | 自分の参加状態 |
| `myMember` | object | 自分のメンバー情報 |
| `roles` | object[] | ロール一覧（`includeRoles=true` 時） |
| `joinState` | string | 参加方式（`open`, `request`, `invite`） |
| `privacy` | string | 公開設定 |

## 注意事項

- `createGroupAnnouncement` は既存のアナウンスを全て削除してから新規作成する。通常の投稿は `posts` エンドポイントを使うこと
- `getGroup` は `includeRoles=true` を付けないとロール情報が返らない
- グループ譲渡は双方の同意が必要（送信側と受信側の両方が操作を行う）
