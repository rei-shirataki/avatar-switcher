---
scope: api
title: VRChat REST API — Users
source: https://vrchat.community/docs/api/
status: community
last_verified: 2026-10-09
---

# VRChat REST API — Users

ベースURL: `https://api.vrchat.cloud/api/1`  
認証: `authCookie` (Cookie: `auth=...`)

## エンドポイント一覧

### 自分自身の情報

| メソッド | パス | operationId | 説明 |
|---|---|---|---|
| GET | `/auth/user` | getCurrentUser | ログイン & 自分のCurrentUser情報取得 |
| PUT | `/users/{userId}` | updateUser | 自分のプロフィール更新（email, birthday等） |

### ユーザー検索・取得

| メソッド | パス | operationId | 説明 |
|---|---|---|---|
| GET | `/users` | searchUsers | displayName でユーザーを検索 |
| GET | `/users/{userId}` | getUser | IDでユーザー情報を取得 |

#### searchUsers クエリパラメータ

| パラメータ | 型 | 説明 |
|---|---|---|
| `search` | string | displayName で検索（空だと空配列） |
| `developerType` | string | `none`=一般ユーザー, `internal`=モデレーター |
| `n` | integer | 取得件数 |
| `offset` | integer | ページングオフセット |

### ユーザーノート

| メソッド | パス | operationId | 説明 |
|---|---|---|---|
| GET | `/userNotes` | getUserNotes | 最近更新したユーザーノート一覧 |
| POST | `/userNotes` | updateUserNote | ユーザーへのメモを更新 |
| GET | `/userNotes/{userNoteId}` | getUserNote | 特定のユーザーノートを取得 |

### タグ管理

| メソッド | パス | operationId | 説明 |
|---|---|---|---|
| POST | `/users/{userId}/addTags` | addTags | ユーザーにタグを追加 |
| POST | `/users/{userId}/removeTags` | removeTags | ユーザーからタグを削除 |

### プロフィール・バッジ

| メソッド | パス | operationId | 説明 |
|---|---|---|---|
| GET | `/profile/{userId}` | getPublicProfile | 公開プロフィール情報を取得 |
| GET | `/profile/{userId}/private` | getPrivateProfile | 認証済みユーザーに見えるプロフィール情報を取得 |
| PUT | `/users/{userId}/badges/{badgeId}` | updateBadge | ユーザーバッジを更新 |
| PUT | `/profile/{userId}` | updateProfile | プロフィール（bio・bioLinks・userIcon・言語・バナー・背景・テーマ・アイコンフレーム等）を更新。`pronouns` / `status` / `statusDescription` は `updateUser` 側で更新する |
| GET | `/users/{userId}/tutorial` | getUserTutorialStatus | チュートリアルの完了状況を取得（対象は `X-Platform` / `X-Store` ヘッダーで指定） |
| POST | `/users/{userId}/tutorial` | completeUserTutorial | `X-Platform` / `X-Store` で指定したチュートリアルを完了済みにし、CurrentUserを返す |
| DELETE | `/users/{userId}/tutorial` | clearUserTutorials | そのプラットフォームで完了した全チュートリアルをクリアし、CurrentUserを返す（`platform-agnostic:custom:onboarding-tutorial-world:v1` など別種のチュートリアルは完了のまま残る） |
| GET | `/users/{userId}/clientConfig` | getUserClientConfig | VRChatがユーザーに紐づけて保存しているクライアント設定（`accessReduceDecorAnim` / `configString`）を取得 |
| PUT | `/users/{userId}/clientConfig` | updateUserClientConfig | クライアント設定を更新（指定したキーのみ変更。現状は `accessReduceDecorAnim`） |
| GET | `/ageVerification/status` | getAgeVerificationStatus | 自分の年齢確認ステータス（`status`）を取得。値は `18+` / `hidden` / `verified`（`verified` は廃止扱い。確認済みの18歳以上は `18+` に切り替え可能） |
| GET | `/users/{userId}/feedback` | getUserFeedback | ユーザーが送信したフィードバックを取得（2026-08-09以降の仕様（v1.22.1）で `deprecated` が解除された） |
| GET | `/users/{username}/name` | getUserByName | **非推奨**（管理者権限が必要）。ユーザー名でユーザー情報を取得 |
| GET | `/users/active` | searchActiveUsers | **非推奨**（管理者権限が必要）。アクティブユーザーをテキスト検索 |

### グループ関連

| メソッド | パス | operationId | 説明 |
|---|---|---|---|
| GET | `/users/{userId}/groups` | getUserGroups | ユーザーの公開グループ一覧を取得 |
| GET | `/users/{userId}/groups/invited` | getInvitedGroups | 招待されているグループ一覧を取得 |
| GET | `/users/{userId}/groups/permissions` | getUserAllGroupPermissions | 参加中の全グループの権限一覧を取得 |
| GET | `/users/{userId}/groups/represented` | getUserRepresentedGroup | 現在代表しているグループを取得 |
| GET | `/users/{userId}/groups/requested` | getUserGroupRequests | 参加リクエスト中のグループ一覧を取得 |
| GET | `/users/{userId}/groups/userblocked` | getBlockedGroups | ブロックしているグループ一覧を取得 |
| GET | `/users/{userId}/instances/groups` | getUserGroupInstances | ユーザーのグループインスタンス一覧を取得 |
| GET | `/users/{userId}/instances/groups/{groupId}` | getUserGroupInstancesForGroup | 特定グループのインスタンス一覧を取得 |

### ミューチュアル

| メソッド | パス | operationId | 説明 |
|---|---|---|---|
| GET | `/users/{userId}/mutuals` | getMutuals | 自分と指定ユーザーのミューチュアル数を取得 |
| GET | `/users/{userId}/mutuals/friends` | getMutualFriends | 自分と指定ユーザーの共通フレンド一覧を取得 |
| GET | `/users/{userId}/mutuals/groups` | getMutualGroups | 自分と指定ユーザーの共通グループ一覧を取得 |

### 永続化データ（World Persistence）

| メソッド | パス | operationId | 説明 |
|---|---|---|---|
| DELETE | `/users/{userId}/persist` | deleteAllUserPersistenceData | ユーザーの全ワールド分の永続化データを削除 |
| DELETE | `/users/{userId}/{worldId}/persist` | deleteUserPersistence | 指定ワールドの永続化データを削除 |
| GET | `/users/{userId}/{worldId}/persist/exists` | checkUserPersistenceExists | 指定ワールドの永続化データの有無を確認 |

## 追加されたクエリパラメータ・ヘッダー（2026-08以降の仕様）

| operationId | 名前 | 説明 |
|---|---|---|
| getPublicProfile | `asSelf` (boolean) | 自分のプロフィール上でVRChatがユーザー本人に見せるプロパティを含める。他ユーザーでは無視される |
| getPublicProfile | `withGroupsAndWorlds` (boolean) | レスポンスに `groups` / `publicWorlds` / `totalPublicWorldsCount` / `worldFavoriteLists` を含める |
| getUserTutorialStatus / completeUserTutorial / clearUserTutorials | `X-Platform` (header) | チュートリアルが属するプラットフォーム。`standalonewindows` / `android` / `ios` はそのまま保持され、それ以外は `null` として記録される |
| 同上 | `X-Store` (header) | チュートリアルが属するストア。送信した値がそのまま記録される |

## updateUser リクエストボディ

`PUT /users/{userId}` が受け付けるフィールドです（すべて任意）。

| フィールド | 型 | 説明 |
|---|---|---|
| `displayName` | string | 表示名 |
| `revertDisplayName` | boolean | 表示名を元に戻す。**`currentPassword` も必須** |
| `email` / `password` / `currentPassword` | string | メールアドレス・パスワードの変更 |
| `birthday` | string | 誕生日 |
| `acceptedTOSVersion` | integer | 同意した利用規約のバージョン |
| `status` / `statusDescription` | string | オンライン状態・ステータスメッセージ |
| `pronouns` | string | 代名詞 |
| `tags` | string[] | タグ |
| `contentFilters` | string[] | コンテンツ表示のゲート。`content_` で始まるタグ |
| `isBoopingEnabled` | boolean | Boop を受け付けるか |
| `hasSharedConnectionsOptOut` | boolean | Mutuals 機能からオプトアウト |
| `hasDiscordFriendsOptOut` | boolean | Discord Friend Connections 機能からオプトアウト |
| `allowWorldsToCountFriendsInInstance` | boolean | 「Allow Worlds to Count Friends in Instance」設定（Udon のフレンド情報メソッドと同時に導入） |
| `unsubscribe` | boolean | メール購読の解除 |

> `bio` / `bioLinks` / `userIcon` は以前 `updateUser` で更新できましたが、2026-08以降の仕様では**リクエストボディから削除**され、`PUT /profile/{userId}`（`updateProfile`）に移りました。

## updateProfile リクエストボディ

`PUT /profile/{userId}` のフィールドです（すべて任意）。

| フィールド | 型 | 説明 |
|---|---|---|
| `bio` | string | 自己紹介文 |
| `bioLinks` | string[] | プロフィールのリンク |
| `userIcon` | string | ユーザーアイコン（有効な VRChat のファイルURL） |
| `languages` | string[] | 言語（[tags.md](tags.md) の `language_*` に対応） |
| `bannerType` | string | `avatarBanner` / `color` / `customImage` |
| `bannerColor` | Color | `bannerType: color` のときの色 |
| `backgroundType` | string | `default` / `gradient` / `inventory` / `texture` |
| `backgroundTextureId` | string | 背景テクスチャID |
| `themeId` | string | テーマID |
| `iconFrame` / `nameplateEffect` / `profileEffect` | string | インベントリのテンプレートID（コスメティクス。[inventory.md](inventory.md)） |

## 主要レスポンスフィールド（User オブジェクト）

| フィールド | 型 | 説明 |
|---|---|---|
| `id` | string | ユーザーID（例: `usr_xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx`） |
| `displayName` | string | 表示名 |
| `username` | string | ユーザー名（非推奨フィールド） |
| `avatarImageUrl` | string | アバター画像URL |
| `status` | string | オンライン状態（`active`, `join me`, `ask me`, `busy`, `offline`） |
| `statusDescription` | string | ステータスメッセージ |
| `bio` | string | 自己紹介文 |
| `location` | string | 現在地（インスタンスID） |
| `isFriend` | bool | 自分のフレンドかどうか |
| `tags` | string[] | タグ一覧（`system_trust_*` 等） |
| `last_login` | string | 最終ログイン日時（ISO 8601） |

## 注意事項

- `searchUsers` で他ユーザーを検索するには認証が必要
- `getUser` は認証なしでも公開情報を取得可能だが一部フィールドは `0` または空になる
- `updateUser` は自分のアカウントのみ更新可能。パスワード変更には `currentPassword` が必要
- `getUserByName`・`searchActiveUsers` は非推奨で、どちらも管理者権限が必要（`getUserFeedback` は2026-08-09以降の仕様（v1.22.1）で `deprecated` が解除された）
