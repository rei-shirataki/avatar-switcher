---
scope: api
title: VRChat REST API — Jams
source: https://vrchat.community/docs/api/
status: community
last_verified: 2026-10-09
---

# VRChat REST API — Jams

ベースURL: `https://api.vrchat.cloud/api/1`  
認証: `authCookie` (Cookie: `auth=...`)

Jam はアバターやワールドの創作コンテストイベントです。

## エンドポイント一覧

| メソッド | パス | operationId | 説明 |
|---|---|---|---|
| GET | `/jams` | getJams | Jam一覧を取得 |
| GET | `/jams/{jamId}` | getJam | 特定のJam情報を取得 |
| GET | `/jams/{jamId}/submissions` | getJamSubmissions | Jamへの投稿一覧 |
| POST | `/jams/{jamId}/submissions` | submitJamContent | Jamにコンテンツを投稿 |
| DELETE | `/jams/{jamId}/submissions/{jamSubmissionId}` | deleteJamSubmission | 投稿を取り消し |

## getJams クエリパラメータ

| パラメータ | 型 | 説明 |
|---|---|---|
| `type` | string | Jamの種別（`avatar` または `world`） |

## 注意事項

- 投稿できるのは自分がアップロードしたコンテンツのみ
- Jamの開催期間内にコンテンツのアップロードと投稿の両方を完了させる必要があります
- `getJamSubmissions` は `contentId`（ワールド/アバターIDでフィルター）または `submitterId`（投稿者IDでフィルター）でフィルタリング可能
