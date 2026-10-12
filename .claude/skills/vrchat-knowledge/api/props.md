---
scope: api
title: VRChat REST API — Props
source: https://vrchat.community/docs/api/
status: community
last_verified: 2026-10-09
---

# VRChat REST API — Props

ベースURL: `https://api.vrchat.cloud/api/1`  
認証: `authCookie` (Cookie: `auth=...`)

Propはインスタンス内にスポーン可能なインタラクタブルアイテムです。

## エンドポイント一覧

| メソッド | パス | operationId | 説明 |
|---|---|---|---|
| GET | `/props` | listProps | Prop一覧を取得 |
| POST | `/props` | createProp | Propを作成 |
| GET | `/props/{propId}` | getProp | Prop情報を取得 |
| PUT | `/props/{propId}` | updateProp | Propを更新 |
| DELETE | `/props/{propId}` | deleteProp | Propを削除 |
| GET | `/props/{propId}/publish` | getPropPublishStatus | Propの公開状態を確認 |
| PUT | `/props/{propId}/publish` | publishProp | Propを公開 |
| DELETE | `/props/{propId}/publish` | unpublishProp | Propを非公開に |

## Prop オブジェクトの主要フィールド

| フィールド | 型 | 説明 |
|---|---|---|
| `id` | string | Prop ID |
| `name` | string | Prop名 |
| `assetUrl` | string | アセットバンドルURL |
| `platform` | string | 対応プラットフォーム |
| `unityVersion` | string | 対応Unityバージョン |
| `spawnType` | string | スポーン方式 |
| `worldPlacementMask` | string | ワールド配置マスク |

## 注意事項

- Propの更新時にアセットバンドルを変更する場合は `name`、`assetUrl`、`platform`、`unityVersion`、`assetVersion`、`spawnType`、`worldPlacementMask` を全て含める必要があります
- `propSignature` が設定されている場合は更新時に `propSignature` も必要です
