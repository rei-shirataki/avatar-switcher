---
scope: api
title: VRChat REST API — Calendar
source: https://vrchat.community/docs/api/
status: community
last_verified: 2026-10-09
---

# VRChat REST API — Calendar

ベースURL: `https://api.vrchat.cloud/api/1`  
認証: `authCookie` (Cookie: `auth=...`)

VRChat コミュニティのカレンダーイベント（グループイベント等）を管理するAPIです。

## エンドポイント一覧

### イベント閲覧

| メソッド | パス | operationId | 説明 |
|---|---|---|---|
| GET | `/calendar` | getCalendarEvents | 自分のカレンダーイベント一覧（月単位） |
| GET | `/calendar/featured` | getFeaturedCalendarEvents | フィーチャードイベント一覧（月単位） |
| GET | `/calendar/following` | getFollowedCalendarEvents | フォロー中のイベント一覧（月単位） |
| GET | `/calendar/discover` | discoverCalendarEvents | イベントを探索 |
| GET | `/calendar/search` | searchCalendarEvents | キーワードでイベントを検索 |

### グループイベント管理

| メソッド | パス | operationId | 説明 |
|---|---|---|---|
| GET | `/calendar/{groupId}` | getGroupCalendarEvents | グループのイベント一覧 |
| POST | `/calendar/{groupId}/event` | createGroupCalendarEvent | グループにイベントを作成 |
| GET | `/calendar/{groupId}/{calendarId}` | getGroupCalendarEvent | 特定のグループイベントを取得 |
| PUT | `/calendar/{groupId}/{calendarId}/event` | updateGroupCalendarEvent | グループイベントを更新 |
| DELETE | `/calendar/{groupId}/{calendarId}` | deleteGroupCalendarEvent | グループイベントを削除 |
| GET | `/calendar/{groupId}/{calendarId}.ics` | getGroupCalendarEventICS | ICS形式でダウンロード |
| GET | `/calendar/{groupId}/next` | getGroupNextCalendarEvent | グループの次のイベントを取得 |
| POST | `/calendar/{groupId}/{calendarId}/follow` | followGroupCalendarEvent | イベントをフォロー/アンフォロー |

> **注**: グループイベント系のパスは `/groups/{groupId}/calendar/...` ではなく `/calendar/{groupId}/...` です。パスパラメータ名も `calendarEventId` ではなく `calendarId` です。

## クエリパラメータ（月指定）

| パラメータ | 型 | 説明 |
|---|---|---|
| `date` | string | 対象月を指定（例: `2026-05`）。省略時は当月 |

`getGroupCalendarEvents`（`GET /calendar/{groupId}`）には、クエリ `after`（この日時より後に始まるイベントのみ返す。date-time）、`limit`、`sort`（例: `startTime_ascending`）が追加されています。

`discoverCalendarEvents`（`GET /calendar/discover`）はカーソルベースのページネーションを使用します。初回は `nextCursor` なしで呼び出し、以降はレスポンスの `nextCursor` を次回リクエストに渡します。

## 注意事項

- ICS形式（`getGroupCalendarEventICS`）はカレンダーアプリ（Google Calendar, Outlook等）にインポートできます
- グループイベントの作成・編集にはグループ内のイベント管理権限が必要です
