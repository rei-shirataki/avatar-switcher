---
scope: api
title: VRChat REST API — Instances
source: https://vrchat.community/docs/api/
status: community
last_verified: 2026-10-09
---

# VRChat REST API — Instances

ベースURL: `https://api.vrchat.cloud/api/1`  
認証: `authCookie` (Cookie: `auth=...`)

## エンドポイント一覧

| メソッド | パス | operationId | 説明 |
|---|---|---|---|
| POST | `/instances` | createInstance | インスタンスを作成 |
| GET | `/instances/recent` | getRecentLocations | 最近訪問したインスタンス一覧 |
| GET | `/instances/s/{shortName}` | getInstanceByShortName | ショートネームでインスタンスを取得 |
| GET | `/instances/{worldId}:{instanceId}` | getInstance | IDでインスタンスを取得 |
| DELETE | `/instances/{worldId}:{instanceId}` | closeInstance | インスタンスを閉鎖 |
| GET | `/instances/{worldId}:{instanceId}/shortName` | getShortName | インスタンスのショートネームを取得 |
| PUT | `/instances/{worldId}:{instanceId}` | updateInstance | グループインスタンスに紐づけるカレンダーイベントを設定／解除（**未リリース**: 2026-10-09時点で v1.22.2-nightly のみ。下記参照） |
| GET | `/instanceCategories` | getInstanceCategories | インスタンスを掲載できるカテゴリ一覧（認証任意） |
| GET | `/instanceVibes` | getInstanceVibes | インスタンスに付けられるバイブ一覧（認証任意） |

## インスタンスIDの形式

```
{worldId}:{instanceId}

例: wrld_12345678-abcd-...:12345~private(usr_...)~nonce(...)
```

`instanceId` は、インスタンス名とアクセス設定の引数を `~` で連結した文字列です。出典: [vrchat.community/instances](https://vrchat.community/instances)（コミュニティによる解析。VRChatは2024-05-02のDeveloper Updateで、将来ユーザーIDのようなUUID風の方式へ置き換える意向を示している）。

### 引数の並び順

```
{name}~{friends | hidden | private | group + groupAccessType}~ageGate~canRequestInvite~region~nonce
```

| 引数 | 意味 |
|---|---|
| `{name}` | インスタンス名（数字など） |
| `~hidden(<userId>)` | Friends+（フレンドのフレンドも参加可） |
| `~friends(<userId>)` | Friends |
| `~private(<userId>)` | Invite |
| `~private(<userId>)~canRequestInvite` | Invite+ |
| `~group(<groupId>)~groupAccessType(public)` | Group Public |
| `~group(<groupId>)~groupAccessType(plus)` | Group Plus |
| `~group(<groupId>)~groupAccessType(members)` | Group Members |
| `~ageGate` | 18歳以上の年齢確認済みユーザーのみ参加可（プロフィールで確認済み表示をしている必要はない） |
| `~region(<token>)` | Photonサーバーの地域（下表）。省略時は `us` |
| `~nonce(<value>)` | 非公開インスタンスのIDを推測されないようにする暗号鍵。パブリックのlocationには含まれない |

- Public には引数がありません（`~` 以降なし）
- `friends` / `hidden` / `private` / `group` のいずれかがあると、そのインスタンスには「オーナー」が存在します
- **オーナー** = インスタンスの作成者（永続的。投票なしで即キック可能）。**インスタンスマスター** = Photon の同期マスターで、最も長く滞在しているユーザー（オブジェクト同期を担当）。両者は別物です

### リージョン（`region`）

| 地域 | ホスト | トークン |
|---|---|---|
| USA, West | San José | `us` |
| USA, East | Washington D.C. | `use` |
| Europe | Amsterdam | `eu` |
| Japan | Tokyo | `jp` |

`createInstance` の `region` には `eu` / `jp` / `unknown` / `us` / `use` が指定できます（必須）。

### location の特殊値

| 値 | 意味 |
|---|---|
| `""` | 疑似null |
| `offline` | VRChatクライアント未起動、またはPipeline未接続（ブラウザタブのみなど） |
| `traveling` | クライアントがインスタンス間を移動中（ワールドDL・同期中など）。`traveling:traveling` の形になることもある |
| `private` | ログイン中のユーザーから場所が見えない（Ask Me / Do Not Disturb 状態、Invite / Invite+ / Group インスタンスなど） |

## createInstance リクエストボディ

| フィールド | 型 | 説明 |
|---|---|---|
| `worldId` | string | **必須**。ワールドID |
| `type` | string | **必須**。`public` / `friends` / `hidden` / `private` / `group` |
| `region` | string | **必須**。`us` / `use` / `eu` / `jp` / `unknown` |
| `ownerId` | string | オーナー（ユーザーIDまたはグループID） |
| `groupAccessType` | string | `type: group` のとき。`members`（既定） / `plus` / `public` |
| `roleIds` | string[] | `type: group` かつ `groupAccessType: members` のとき参加を許可するロールID |
| `displayName` | string | インスタンスの表示名 |
| `description` | string | 説明文 |
| `categoryId` | string | カテゴリID（`icat_...`。`getInstanceCategories` で取得） |
| `vibeIds` | string[] | バイブID（`ivib_...`。`getInstanceVibes` で取得） |
| `calendarEntryId` | string | 紐づけるカレンダーイベントID |
| `canRequestInvite` | boolean | `private` を Invite+ にする。`friends` では拒否される（既定 `false`） |
| `ageGate` | boolean | 年齢確認済みユーザーのみ（既定 `false`） |
| `inviteOnly` | boolean | 既定 `false` |
| `queueEnabled` | boolean | キューを有効化（既定 `false`） |
| `closedAt` | string (date-time) | この時刻以降は入室不可。パブリックでは無効 |
| `hardClose` | boolean | 現在は未使用（将来、閉鎖時にキックするかのフラグになる予定） |
| `contentSettings` | object | コンテンツ設定 |
| `instancePersistenceEnabled` / `playerPersistenceEnabled` | boolean | null | インスタンス／プレイヤーのPersistence |

## カテゴリとバイブ

| 種別 | フィールド |
|---|---|
| InstanceCategory | `id`（`icat_...`）、`name`、`iconUrl`（`vrchat://category_...`）、`order`、`deleted` |
| InstanceVibe | `id`（`ivib_...`）、`title`、`deleted` |

`Instance` オブジェクトには `categoryId` / `displayVibeId` / `vibeIds` が追加されています。

## updateInstance（未リリース）

`PUT /instances/{worldId}:{instanceId}` は、グループインスタンスに紐づけるカレンダーイベントを設定／解除します（`specification` の #597。2026-10-09時点でリリース済みの v1.22.1 には含まれず、v1.22.2-nightly 以降）。

```json
{ "calendarEntryId": "cal_..." }
```

- 解除するときは `calendarEntryId: null` を送る（フィールド自体は必須）
- グループインスタンスの更新には `group-instance-manage` と `group-instance-calendar-link` の両権限が必要
- 対象イベントは「開始が6時間以内」または「終了から6時間以内」である必要がある
- エラー: 400（検証エラー）/ 401 / 403 / 404
## closeInstance クエリパラメータ

| パラメータ | 型 | 説明 |
|---|---|---|
| `hardClose` | boolean | ハードクローズするか（デフォルト: `false`） |
| `closedAt` | string (ISO 8601) | この時刻以降は入室不可にする。省略時は即時閉鎖 |

## Instance オブジェクトの主要フィールド

| フィールド | 型 | 説明 |
|---|---|---|
| `id` | string | インスタンスID |
| `worldId` | string | ワールドID |
| `instanceId` | string | インスタンス番号 + アクセス修飾子 |
| `shortName` | string | ショートリンク用の短縮名 |
| `ownerId` | string | オーナーのユーザーID |
| `type` | string | インスタンスタイプ（下記参照） |
| `userCount` | integer | 現在の人数 |
| `capacity` | integer | 最大収容人数 |
| `full` | boolean | 満員かどうか |
| `platforms` | object | プラットフォーム別の人数 |
| `categoryId` | string | null | カテゴリID |
| `displayVibeId` / `vibeIds` | string | null / string[] | 表示するバイブ／付与されたバイブ |
| `description` | string | null | 説明文 |
| `calendarEntryId` | string | 紐づくカレンダーイベントID |
| `languages` / `languageRatio` / `dominantLanguage` | string[] / object / string | 参加者の言語分布（`languages` は `languageRatio` のキーを割合順に並べたもの） |
| `recommendedCapacity` | integer | 推奨収容人数 |
| `disabledPropAbilities` | array | 無効化されているProp機能 |

## インスタンスタイプ（type）

| 値 | 説明 |
|---|---|
| `public` | 誰でも参加可能 |
| `hidden` | フレンド+（フレンドのフレンドも参加可） |
| `friends` | フレンドのみ |
| `private` | 招待のみ |
| `group` | グループインスタンス。アクセス範囲は `groupAccessType`（`members` / `plus` / `public`）で決まる |

## 注意事項

- `getInstance` に無効な `instanceId` を渡すと `null` が返ります（エラーではない）
- `closeInstance` はオーナー自身、またはグループインスタンスの場合は `group-instance-manage` 権限が必要
- `getRecentLocations` は自分が最近訪れたインスタンスのみ返します
