---
scope: api
title: VRChat REST API — Prints
source: https://vrchat.community/docs/api/
status: community
last_verified: 2026-10-09
---

# VRChat REST API — Prints

ベースURL: `https://api.vrchat.cloud/api/1`  
認証: `authCookie` (Cookie: `auth=...`)

PrintはVRChatカメラから直接印刷した写真機能です。

## エンドポイント一覧

| メソッド | パス | operationId | 説明 |
|---|---|---|---|
| POST | `/prints` | uploadPrint | 写真をアップロードして作成 |
| GET | `/prints/user/{userId}` | getUserPrints | ユーザーのPrint一覧 |
| GET | `/prints/{printId}` | getPrint | 特定のPrintを取得 |
| POST | `/prints/{printId}` | editPrint | Printを編集 |
| DELETE | `/prints/{printId}` | deletePrint | Printを削除 |

## uploadPrint / editPrint のフィールド

multipart/form-data で送信します。フィールド名は `file` ではなく `image`、キャプションは `caption` ではなく `note`、撮影日時は `capturedAt` ではなく `timestamp` です。

| フィールド | 型 | uploadPrint | editPrint | 説明 |
|---|---|---|---|---|
| `image` | binary (PNG) | 必須 | 必須 | 画像データ |
| `note` | string | 任意 | 任意 | キャプション |
| `timestamp` | string (ISO 8601) | 必須 | — | 撮影日時 |
| `worldId` | string | 任意 | — | 撮影したワールドのID |
| `worldName` | string | 任意 | — | 撮影したワールドの名前 |

## 注意事項

- `getUserPrints` は**自分のUserIDのみ**指定可能。他ユーザーのPrintは取得できません
