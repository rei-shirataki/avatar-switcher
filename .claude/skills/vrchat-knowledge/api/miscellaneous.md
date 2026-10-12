---
scope: api
title: VRChat REST API — Miscellaneous
source: https://vrchat.community/docs/api/
status: community
last_verified: 2026-10-09
---

# VRChat REST API — Miscellaneous

ベースURL: `https://api.vrchat.cloud/api/1`  
認証: `authCookie` (Cookie: `auth=...`)

他のカテゴリに分類されない汎用エンドポイントです。

## エンドポイント一覧

| メソッド | パス | operationId | 認証 | 説明 |
|---|---|---|---|---|
| GET | `/config` | getConfig | 不要 | APIの設定情報を取得 |
| GET | `/health` | getHealth | 不要 | APIの稼働状態を確認（**非推奨、常に401を返す**） |
| GET | `/time` | getSystemTime | 不要 | サーバーの現在時刻を取得 |
| GET | `/visits` | getCurrentOnlineUsers | 不要 | 現在のオンラインユーザー数を取得 |
| GET | `/infoPush` | getInfoPush | 必須 | インフォメーション通知を取得 |
| GET | `/auth/permissions` | getAssignedPermissions | 必須 | 自分に付与されているパーミッション一覧 |
| GET | `/beta/{betaName}` | getBeta | 不要 | ベータプログラムと、登録時に入力が必要なフィールド（`userFields`）を取得 |
| GET | `/beta/{betaName}/register` | getBetaRegistration | 必須 | 自分のベータプログラム登録情報を取得（未登録は404） |
| GET | `/frontend/branches` | getFrontendBranches | 必須 | 自分が切り替え可能なフロントエンドのブランチ一覧 |
| GET | `/css/app.css` | getCSS | 不要 | フロントエンドのCSSを取得（302リダイレクト） |
| GET | `/js/app.js` | getJavaScript | 不要 | フロントエンドのJSを取得（302リダイレクト） |

## 主な用途

### `GET /config`

APIの動作設定（利用規約バージョン・機能フラグ・制限値等）を含むオブジェクトを返します。
アプリ起動時のヘルスチェックや設定取得に使用されます。

### `GET /health`

> **非推奨（`deprecated: true`）**: VRChatが理由不明のままこのエンドポイントを制限しており、現在は常に401 Unauthorizedを返します。ヘルスチェック用途には使用できません。

```json
{ "ok": true, "serverName": "...", "buildVersionTag": "..." }
```

（本来はAPIの稼働状態・サーバー名・ビルドバージョンタグを返す想定でしたが、現状は機能していません）

### `GET /time`

サーバーの現在時刻（UTC）を返します。クライアントとサーバーの時刻同期に使用できます。

### `GET /visits`

現在オンラインのユーザー数を整数で返します。

### `GET /auth/permissions`

VRC+ 等のサブスクリプションによって付与されているパーミッション情報を返します。

## 注意事項

- `/css/app.css` と `/js/app.js` は Cloudfront への 302 リダイレクトを返します。HTTPライブラリがリダイレクト追従に対応している必要があります
- `/permissions`（管理者向け）と `/auth/permissions`（一般ユーザー向け）は別のエンドポイントです
- `GET /health` は非推奨で、現在は常に401を返します。稼働確認には別の手段（例: `/config` や `/time` への疎通確認）を検討してください
- `/infoPush` は `authCookie` による認証が必須です
