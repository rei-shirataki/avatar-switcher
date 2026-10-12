---
scope: api
title: VRChat REST API — 概要・認証
source: https://vrchat.community/docs/api/
status: community
last_verified: 2026-10-09
---

# VRChat REST API — 概要・認証

## 重要な前提

> **このAPIはVRChatによる公式サポートなし。**  
> コミュニティがリバースエンジニアリングで維持するドキュメントであり、予告なく変更・廃止される可能性があります。  
> **乱用するとアカウント停止のリスクがあります。**

ドキュメント: [vrchat.community](https://vrchat.community/)  
OpenAPI仕様: [github.com/vrchatapi/specification](https://github.com/vrchatapi/specification)

## ベースURL

```
https://api.vrchat.cloud/api/1
```

## セキュリティスキーム

| 名前 | 種別 | 説明 |
|---|---|---|
| `authCookie` | Cookie | `auth` Cookie によるセッション認証 |
| `authHeader` | HTTP Basic | `Authorization: Basic {base64}` によるログイン時認証 |
| `twoFactorAuthCookie` | Cookie | `twoFactorAuth` Cookie による2FA済み状態の維持 |

## 認証フロー

### 1. ログイン

```
GET /auth/user
Authorization: Basic {base64(urlencode(username):urlencode(password))}
```

- `{base64(...)}` は `username` と `password` を**それぞれURLエンコード**してコロンで結合し、Base64エンコードしたもの
- 成功すると `auth` Cookie と `twoFactorAuth` Cookie が Set-Cookie で返される
- **以降のリクエストは `auth` Cookie を送り続けることでセッションを維持**

> **WARNING**: ログインするたびに新しいセッションを消費します。同時セッション数には上限があります（上限値は非公開）。  
> **`auth` Cookie を保存・再利用してください。** 毎回ログインするとすぐにセッション上限に達します。

### 2. 2FAが必要な場合

ログインレスポンスで `requiresTwoFactorAuth` が返される場合、以下のいずれかで2FAを完了させます：

| エンドポイント | 用途 |
|---|---|
| `POST /auth/twofactorauth/totp/verify` | TOTP（認証アプリ）コード |
| `POST /auth/twofactorauth/emailotp/verify` | メールOTPコード |
| `POST /auth/twofactorauth/otp/verify` | リカバリーコード |

```json
{ "code": "123456" }
```

### 2.5 認証まわりのその他のエンドポイント

| メソッド | パス | operationId | 説明 |
|---|---|---|---|
| GET | `/auth/user/interestsAndPreferences` | getInterestsAndPreferences | 自分がオンにしている興味・好みを取得 |
| PUT | `/auth/user/interestsAndPreferences` | updateInterestsAndPreferences | 興味・好みをオン(`true`)／オフ(`false`)にする。本文に無いキーは現状維持 |
| GET | `/oauth/redirectCode` | getOAuthRedirectCode | 現在のセッションをOAuthリダイレクトに引き渡すための短命コード(`code`: `redirect_...`)を発行 |
| GET | `/sso/{provider}` | getSsoToken | 外部サービス用のSSOトークン(`token`)を発行。`provider` は `canny` / `furality` のみ。他は400 |

`interestsAndPreferences` のキー(いずれも boolean。値が `true` のキーだけが返る): `Anime` / `Art` / `Avatars` / `BigGroup` / `Explore` / `Fantasy` / `Fashion` / `FindAvatars` / `Furries` / `Horror` / `LanguageLearning` / `MeetPeople` / `Music` / `Mystery` / `SciFi` / `SmallGroup` / `Surprise`

### 3. セッション確認

```
GET /auth
```

現在の `auth` Cookie が有効かどうかを検証します。

### 4. ログアウト

```
PUT /logout
Cookie: auth=...
```

セッションを無効化します。

### OpenAPI 仕様上の認証表記の変更（2026-08-09以降）

OpenAPI 仕様の `security` 表記が変わったもので、実際の挙動が変わったとは限りません（確認していません）。

| 区分 | operationId |
|---|---|
| `security` が未指定から `authCookie` 必須に明示された | `searchGroups`、`getGroupCalendarEventICS`、`createWorld` |
| `authCookie` 必須から「認証なしでも可（認証任意）」に変わった | `getInstance`、`getShortName`、`getAvatarStyles`、`checkUserExists`、`discoverCalendarEvents`、`getFeaturedCalendarEvents` |
| 認証任意として新設 | `getInstanceCategories`、`getInstanceVibes` |
| `security: []`（認証不要）が明示された | `confirmEmail`、`registerUserAccount`、`verifyLoginPlace`、`getCSS`、`getJavaScript` |

`checkUserExists`（`GET /auth/exists`）は 429（レート制限）を返すことがあります。

## 共通ヘッダー

| ヘッダー | 値 | 説明 |
|---|---|---|
| `User-Agent` | `MyApp/1.0 contact@example.com` | 識別子として必須（空だとブロックされることあり） |
| `Content-Type` | `application/json` | POSTリクエスト時 |
| `Cookie` | `auth=authcookie_...` | 認証済みリクエスト時 |

## レート制限

- HTTP 429 が返された場合は**指数バックオフ**で再試行（初回1秒、以降倍増）
- レート制限のタイミングや閾値は一定ではなく予測不可能
- データは積極的にキャッシュして不要なAPIコールを減らすこと
- 429 を受け取ったら即座に停止すること

## エンドポイントカテゴリ

| カテゴリ | エンドポイント数 | 概要 |
|---|---|---|
| Authentication | 27 | ログイン・2FA・メール確認・セッション |
| Users | 35 | ユーザー情報・検索・更新・グループ関連・永続化データ |
| Worlds | 16 | ワールド検索・作成・管理・公開ステータス |
| Avatars | 14 | アバター検索・作成・管理・Impostor |
| Friends | 6 | フレンド申請・管理 |
| Groups | 53 | グループ管理・ロール・メンバー |
| Instances | 9 | インスタンス作成・取得・更新、カテゴリ・バイブ |
| Notifications | 13 | 通知管理 |
| Favorites | 10 | お気に入り管理 |
| Files | 19 | ファイルアップロード・管理 |
| Economy | 47 | 購入・サブスクリプション・取引 |
| Miscellaneous | 16 | 設定・ヘルスチェック |
| Calendar | 13 | グループカレンダーイベントの検索・作成・管理 |
| Inventory | 17 | インベントリアイテム・ドロップ・クローニング・コスメティクス管理 |
| Invite | 11 | インスタンス招待・招待メッセージの送受信 |
| Jams | 5 | ワールドジャムの一覧・提出管理 |
| PlayerModeration | 4 | ユーザーによるミュート・ブロック等のモデレーション |
| Prints | 5 | VRChatカメラで撮影したプリントの管理 |
| Props | 8 | インスタンスにスポーン可能なPropの管理 |

## 関連ドキュメント

| ファイル | 内容 |
|---|---|
| [tags.md](tags.md) | ユーザー・ワールド・グループ・言語タグの意味 |
| [shortlinks.md](shortlinks.md) | `vrch.at` / `vrc.group` の短縮リンクとリダイレクト、グループコードからのID解決 |
| [instances.md](instances.md) | インスタンスIDの構造、リージョン、location の特殊値 |

## 利用可能なSDK

| 言語 | リポジトリ |
|---|---|
| TypeScript / JavaScript | vrchatapi/vrchatapi-javascript |
| Python | vrchatapi/vrchatapi-python |
| Dart | vrchatapi/vrchatapi-dart |
| .NET (C#) | vrchatapi/vrchatapi-csharp |
| Java | vrchatapi/vrchatapi-java |
| Rust | vrchatapi/vrchatapi-rust |
