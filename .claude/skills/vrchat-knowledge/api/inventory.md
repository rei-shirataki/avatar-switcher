---
scope: api
title: VRChat REST API — Inventory
source: https://vrchat.community/docs/api/
status: community
last_verified: 2026-10-09
---

# VRChat REST API — Inventory

ベースURL: `https://api.vrchat.cloud/api/1`  
認証: `authCookie` (Cookie: `auth=...`)

ドロップバンドルなどのコレクタブルアイテムを管理するAPIです。

## エンドポイント一覧

### インベントリ全体

| メソッド | パス | operationId | 説明 |
|---|---|---|---|
| GET | `/inventory` | getInventory | インベントリオブジェクトを取得 |
| GET | `/inventory/collections` | getInventoryCollections | コレクション名一覧 |
| GET | `/inventory/drops` | getInventoryDrops | ドロップ一覧 |

`getInventory`（`GET /inventory`）には、クエリ `isNavBar`（boolean）と `seen`（boolean）が追加されています。

### コスメティクス

| メソッド | パス | operationId | 説明 |
|---|---|---|---|
| GET | `/cosmetics/index/{itemType}` | getCosmetics | VRChatが公開している種別のコスメティクスを、所持の有無を問わず全件一覧 |
| GET | `/user/{userId}/cosmetics` | getUserCosmetics | ユーザーが所持しているコスメティクス一覧（`acquiredOn` / `acquisition` / `id` / `itemType` / `templateId` / `userAttributes`） |

`itemType`（`getCosmetics` のパスパラメータ）は `droneskin` / `iconFrame` など。インベントリ側の `InventoryItemType` は `bundle` / `droneskin` / `emoji` / `iconFrame` / `nameplateEffect` / `portalskin` / `profileEffect` / `prop` / `sticker` / `warpeffect`。

### アイテム操作

| メソッド | パス | operationId | 説明 |
|---|---|---|---|
| GET | `/inventory/template/{inventoryTemplateId}` | getInventoryTemplate | インベントリテンプレートを取得 |
| GET | `/inventory/{inventoryItemId}` | getOwnInventoryItem | 自分のインベントリアイテムを取得 |
| PUT | `/inventory/{inventoryItemId}` | updateOwnInventoryItem | アイテムを更新 |
| DELETE | `/inventory/{inventoryItemId}` | deleteOwnInventoryItem | アイテムを削除 |
| PUT | `/inventory/{inventoryItemId}/consume` | consumeOwnInventoryItem | アイテムを消費 |
| PUT | `/inventory/{inventoryItemId}/equip` | equipOwnInventoryItem | アイテムを装備 |
| DELETE | `/inventory/{inventoryItemId}/equip` | unequipOwnInventorySlot | スロットのアイテムを装備解除（`inventoryItemId` にスロットIDを指定） |
| GET | `/user/{userId}/inventory/{inventoryItemId}` | getUserInventoryItem | 他ユーザーのアイテムを取得 |

### 共有・スポーン

| メソッド | パス | operationId | 説明 |
|---|---|---|---|
| POST | `/inventory/cloning/direct` | shareInventoryItemDirect | 他ユーザーに直接共有 |
| GET | `/inventory/cloning/pedestal` | shareInventoryItemPedestal | ペデスタル経由で共有 |
| GET | `/inventory/spawn` | spawnInventoryItem | インスタンスにアイテムをスポーン |

### リワード

| メソッド | パス | operationId | 説明 |
|---|---|---|---|
| POST | `/reward/redeem` | redeemReward | リワードを引き換え |

## 注意事項

- インベントリはVRChatのコレクタブルアイテム（ドロップバンドル等）の管理機能です
- 他ユーザーのアイテムは `getUserInventoryItem` でのみ参照可能（変更不可）。パスは `/user/{userId}/...`（単数形）です
- アイテムパスは `/inventory/{inventoryId}/item/{itemId}` ではなく、フラットな `/inventory/{inventoryItemId}` です
- `equipOwnInventoryItem`（PUT）と `unequipOwnInventorySlot`（DELETE）は同じパス `/inventory/{inventoryItemId}/equip` に対する異なるHTTPメソッドです（DELETE時の `inventoryItemId` はスロットIDとして扱われます）
- `shareInventoryItemPedestal` と `spawnInventoryItem` はGETメソッドです（POSTではありません）
