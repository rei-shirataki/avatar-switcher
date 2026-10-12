---
scope: api
title: VRChat REST API — 短縮リンクとリダイレクト
source: https://vrchat.community/shortlinks
status: community
last_verified: 2026-10-09
---

# VRChat REST API — 短縮リンクとリダイレクト

ユーザー・インスタンス・グループ・カレンダーイベントを共有するための短縮URLと、そのリダイレクトの仕組みです。出典は [vrchat.community/shortlinks](https://vrchat.community/shortlinks) で、**コミュニティによる調査**です。いずれも最終的には `https://vrchat.com/home/...` のページへ飛びます。

## HTTP リダイレクトの挙動

- `GET` に対し、`302 Found` / `302 Moved Temporarily` / `307 Temporary Redirect` を連鎖して返し、`Location` ヘッダーに次の URL を入れる
- 宛先が API（パスが `/api/1/` で始まる）の場合は、**適切な `User-Agent` ヘッダーが必須**（[overview.md](overview.md) の共通ヘッダーを参照）
- リダイレクトを自動で追わない設定にして `Location` を読めば、解決先のIDだけを取得できる

## 種別ごとの短縮URL

| 対象 | 短縮URL | リダイレクト先 |
|---|---|---|
| ユーザー | `https://vrch.at/<userId>` | `https://vrchat.com/home/user/<userId>` |
| インスタンス | `https://vrch.at/<shortName>` | `https://vrchat.com/i/<shortName>` → `https://vrchat.com/home/launch?worldId=<worldId>&instanceId=<instanceId>&shortName=<shortName>` |
| ワールド | `https://vrch.at/<worldId>` | `https://vrchat.com/home/launch?worldId=<worldId>`（ワールド情報ページではなく、インスタンスの特殊ケースとして扱われる） |
| グループ | `https://vrc.group/<groupCode>.<discriminator>` | `https://vrchat.com/api/1/groups/redirect/<code>` → `https://api.vrchat.com/api/1/groups/redirect/<code>` → `https://vrchat.com/home/group/<groupId>` |
| カレンダーイベント | `https://vrch.at/c/<shortCode>` | `https://vrchat.com/api/1/shortCode/c/<shortCode>` → `https://vrchat.com/home/group/<groupId>/calendar/<calendarId>` |

### 注意点

- **旧形式のユーザーID**（例: VRChat公式アカウントの `8JoV9XEdpo`）は、`vrch.at/<id>` ではインスタンスの `shortName` として解釈されるため、ユーザーに解決されない
- **カレンダーイベントの最後のリダイレクトは推測**です。2026-04-06 時点で `/api/1/shortCode/c/<shortCode>` は常に `404 Not Found` を返し、ウェブサイト内の未使用コードに基づいて書かれています
- 短縮名（`shortName`）自体は REST API の `getShortName` / `getInstanceByShortName`（[instances.md](instances.md)）でも扱える

## グループコードからグループIDを解決する

グループ側の最後のリダイレクトは、**グループ検索 API を使わずに「グループコード＋識別子」から groupId を解決する**用途に使えます。

```
GET https://api.vrchat.com/api/1/groups/redirect/VRCHAT.0000
→ 302 Location: https://vrchat.com/home/group/grp_7ccb6ca3-cd36-4dab-9ab1-7bcf08d794e4
```

リクエスト例（`User-Agent` が必須）:

```
GET /VRCHAT.0000 HTTP/1.1
Host: vrc.group
User-Agent: example/4.2.67 example@vrchat.community; https://discord.gg/vrchat; https://github.com/vrchatapi/vrchat.community
Accept: */*

HTTP/1.1 302 Found
Location: https://vrchat.com/api/1/groups/redirect/VRCHAT.0000
```

> `groups/redirect` は OpenAPI 仕様には含まれていない（上記はウェブ側の挙動を観察したもの）。仕様に載っている `searchGroups` は [groups.md](groups.md) を参照。
