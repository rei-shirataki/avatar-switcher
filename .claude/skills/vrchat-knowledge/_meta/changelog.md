---
scope: meta
title: 仕様変更ログ
source: internal
status: verified
last_verified: 2026-10-09
---

# 仕様変更ログ

VRChat OSC・API・WebSocketの仕様変更を記録するファイルです。
変更を確認したら `last_verified` とともにここに記録してください。

## 2026-10-09

- **変更内容**: 前回検証（2026-08-16）以降の更新を取り込み、あわせて「元々KBになかった情報」を追加した（Issue #1）
  - **REST API: 仕様更新に伴う修正**（基準は 2026-08-09 時点の `vrchatapi/specification` `b7fff1a`、比較先は 2026-10-08 時点の `main`。リリース済みは v1.22.1、`main` は v1.22.2-nightly.4）
    - 新規エンドポイント23件を追記: 認証系4件（`interestsAndPreferences` の GET/PUT、`oauth/redirectCode`、`sso/{provider}`）、ユーザー系8件（`profile/{userId}` PUT、`users/{userId}/tutorial` の POST/DELETE、`users/{userId}/clientConfig` の GET/PUT、`ageVerification/status`）、Economy 3件（`economy/status`、`listing/{productId}/products`、`user/{userId}/economy/balance`）、Favorites 2件、Instances 3件、Inventory 2件、Misc 3件（`beta/{betaName}`、`beta/{betaName}/register`、`frontend/branches`）
    - `removeWorldTags` のパスを `/worlds/{worldId}/removeTags` から `/worlds/{worldId}/deleteTags` に修正
    - 仕様上の `deprecated` が解除された4件（`selectFallbackAvatar` / `getSteamTransaction` / `getUserFeedback` / `getWorldMetadata`。8/9時点では非推奨、10/8時点の `main`（v1.22.1以降）では解除）の「非推奨」注記を修正
    - リクエストボディの変更を反映: `updateUser` から `bio` / `bioLinks` / `userIcon` が削除され `updateProfile` へ移動、`createInstance` に `categoryId` / `vibeIds` / `description` を追加、`updateWorld` に `recommendedCapacity` / `previewYoutubeId` / `urlList` / `disabledPropAbilities` を追加
    - 追加クエリパラメータを反映（`getGroupCalendarEvents` の `after` / `limit` / `sort`、`getPublicProfile` の `asSelf` / `withGroupsAndWorlds`、`getSubscriptions` の `gifts` / `recurring` など）と、OpenAPI 仕様上の認証（`security`）表記の変更（実挙動の変更とは限らない）を `api/overview.md` に記載
    - `api/overview.md` のカテゴリ別エンドポイント数を更新（Authentication 27 / Users 35 / Instances 9 / Favorites 10 / Economy 47 / Miscellaneous 16 / Inventory 17）
    - `updateInstance`（`PUT /instances/{worldId}:{instanceId}`、カレンダーイベントの紐付け）は **v1.22.2-nightly のみで未リリース**のため、その旨を明記して記載
    - `api/instances.md` の `type` 表から、仕様の `InstanceType` に存在しない `groupPublic` を削除し、`group` + `groupAccessType` の説明に修正
  - **OSC（`wiki.vrchat.com/wiki/OSC` 由来）を追加**: `osc/usercamera.md`（`/usercamera/*` 全エンドポイントと既定値・範囲）、`osc/dolly.md`（`/dolly/*`）を新規作成。`osc/input-controller.md` に `LookVertical` / `ToggleSitStand` / `AFKToggle` / `ShowDebugInfo0〜9`、`osc/trackers.md` に `/tracking/vrsystem/*/pose`、`osc/avatar-parameters.md` に `VRCEmote` / `VRCFaceBlendV/H` と組み込みパラメータの補足、`osc/overview.md` にメッセージ型表・OSCQuery の TCP ポート・mDNS の UDP 5353 を追加
  - **REST（`vrchat.community` のガイド系ページ）を追加**: `api/tags.md`（ユーザー・ワールド・グループ・言語タグ）、`api/shortlinks.md`（`vrch.at` / `vrc.group` のリダイレクト、グループコードからのID解決）を新規作成。`api/instances.md` にインスタンスIDの引数構造・リージョントークン・location の特殊値を追加
  - 検証範囲: OSC は `docs.vrchat.com` の10ページ（各ページの `updatedAt` は 2025-11-13、`osc-avatar-scaling` のみ 2026-04-27 でいずれも前回検証より前）とアドレスを機械的に突き合わせ、差分なしを確認。WebSocket は `vrchat.community/websocket` のイベント種別が既存の `websocket/pipeline.md` に網羅されていることを確認
  - **未実施**: 仕様のレスポンススキーマのフィールド単位の比較（操作・パラメータ・リクエストボディ参照・ステータスコードの比較までは全エンドポイントで実施）。`wiki.vrchat.com` は OSC ページのみ、OSCQuery の GitHub Wiki は未確認。`api/files.md` / `friends.md` / `invites.md` / `jams.md` / `notifications.md` / `player-moderation.md` / `prints.md` / `props.md` は上記の範囲で差分がなかったため `last_verified` のみ更新
  - `websocket/pipeline.md` は仕様側に `economy-update` / `modified-image-update` のスキーマが無い点と、`content-refresh` のスキーマが「正しいか不明」とされている点を確認したが、内容は変更していない
- **影響ファイル**: `api/*.md`（全19件の `last_verified` 更新、うち内容変更は11件）、`api/tags.md`・`api/shortlinks.md`（新規）、`osc/usercamera.md`・`osc/dolly.md`（新規）、`osc/overview.md`・`input-controller.md`・`trackers.md`・`avatar-parameters.md`、`_meta/index.md`、`SKILL.md`、`README.md`
- **ソース**: https://github.com/vrchatapi/specification（`b7fff1a` → `main`/v1.22.2-nightly.4）、https://wiki.vrchat.com/wiki/OSC、https://docs.vrchat.com/docs/osc-overview（および配下のOSCページ）、https://vrchat.community/ （`instances` / `shortlinks` / `tags` / `websocket` / `faq`）

## 2026-08-16

- **変更内容**: `SKILL.md` に frontmatter（`name` / `description`）を追加。従来 frontmatter が存在せず、スキル一覧表示時に H1 見出しがフォールバック表示されており、トリガー精度が低下していた
  - `description` にOSC・REST API・WebSocketの主要キーワードを列挙し、スキル自動呼び出しの判定材料とした
  - `SKILL.md` の構成ファイル表（30行超）を `_meta/index.md` と重複させず、セクション単位の要約表に圧縮（内容の一次情報は `_meta/index.md` に一本化）
- **影響ファイル**: `SKILL.md`
- **ソース**: https://code.claude.com/docs/en/skills.md#frontmatter-reference（Claude Code Skills frontmatter仕様）

- **変更内容**: 全31コンテンツファイル（OSC 11・REST API 19・WebSocket 1）を一斉再検証。`last_verified` が2026-05-21で約3ヶ月停滞していたため、公式ドキュメント（`docs.vrchat.com`）および `vrchatapi/specification` の OpenAPI 定義（raw YAML）と全件照合した。主な修正点は以下の通り（軽微な文言修正は省略）：
  - **REST API（大きな乖離）**
    - `api/economy.md`: 収録エンドポイントが43件中15件のみだった（商品リスティング・購入履歴・残高・Tilia連携・サブスクリプション等を大幅追加、非推奨エンドポイントに `deprecated` 注記）
    - `api/user.md`: 21件のエンドポイント漏れ（プロフィール・バッジ・グループ関連・ミューチュアル・永続化データ）を追加
    - `api/world.md`: 8件のエンドポイント漏れ（tags操作・platform削除・publish系・getWorldInstance）を追加
    - `api/inventory.md`: パス構造が誤り（`/inventory/{inventoryId}/item/{itemId}` → 実際は `/inventory/{inventoryItemId}` のフラット構造）、複数エンドポイントのHTTPメソッドも誤っていたため全面修正
    - `api/groups.md`（53エンドポイント）: `addGroupMemberRole` のメソッド誤り(POST→PUT)、`declineGroupInvite`/`getGroupGalleryImages`/`getGroupTransferability` 等のパス誤りを修正
    - `api/calendar.md`: グループカレンダーのパスが誤り（`/groups/{groupId}/calendar/...` → 実際は `/calendar/{groupId}/...`）、`.ics` サフィックスの誤りも修正
    - `api/favorites.md`: `clearFavoriteGroup` のメソッド・パスが誤り（PUT+`/clear` → 実際は DELETE のみ）
    - `api/invites.md`: 写真付き招待3エンドポイントのパス構造誤りを修正
    - `api/player-moderation.md`: 存在しない `unblock` を削除し `muteChat`/`unmuteChat` を追加。`hideAvatar`/`showAvatar` が2022年10月以降ローカルストレージ管理に移行しAPIには反映されない点を注記
    - `api/files.md`: analysis系エンドポイントのパス誤り、`getFiles`/`uploadGalleryImage`/`uploadIcon`/`getAdminAssetBundle` の漏れを修正
    - `api/props.md` / `api/prints.md` / `api/jams.md` / `api/miscellaneous.md`: パスパラメータ名・HTTPメソッド・フィールド名の誤りを修正（詳細は各ファイル参照）
    - `api/overview.md`: エンドポイントカテゴリ表のカウントが全面的に陳腐化していたため実測値に更新し、欠落していた7カテゴリ（Calendar/Inventory/Invite/Jams/PlayerModeration/Prints/Props）を追加
  - **OSC（軽微〜中程度の修正）**
    - `osc/resources.md`: ツール・ライブラリ一覧に多数の漏れ・誤分類・言語表記ミスがあり大幅修正（ハートレート5件・IRL Control 4件・Misc 2件を新規追加等）
    - `osc/avatar-scaling.md`: `eyeheightmin`/`eyeheightmax` の方向が「読み書き」と誤記されていたが実際は読み取り専用
    - `osc/oscquery.md`: 公式のOSCQuery接続ガイドへの言及が丸ごと欠落していたため追加
    - `osc/eye-tracking.md` / `osc/trackers.md` / `osc/diy.md` / `osc/overview.md`: 説明の精緻化・欠落情報の追加
    - `osc/chatbox.md` / `osc/debugging.md` / `osc/input-controller.md` / `osc/avatar-parameters.md`: 内容は最新のソースと一致（`last_verified` のみ更新）
  - **WebSocket**
    - `websocket/pipeline.md`: 未文書化だった7件のUserイベント（`user-badge-assigned` 等）、`location` フィールドの `traveling` 状態、エラーメッセージ例を追加
  - 全ファイルで `status:` / `source:` フィールドは変更せず、内容確認結果のみ反映（`source:` が汎用URLの場合、実際の照合には `vrchatapi/specification` の該当YAMLファイルを使用）
  - `_meta/index.md`: 上記再検証結果を反映し、`api/groups.md` のエンドポイント数表記を44→53に修正、`api/economy.md` の概要説明にTilia決済連携・商品リスト管理を追記、`last_verified` を更新
- **影響ファイル**: `osc/*.md`（11件）、`api/*.md`（19件）、`websocket/pipeline.md`、`_meta/index.md`
- **ソース**: https://docs.vrchat.com/ 各ページ、https://github.com/vrchatapi/specification（`main`ブランチ、`openapi/components/paths/*.yaml`）

## 2026-05-21

- **初回作成**: ナレッジベース全体を構築
  - OSC: 11ファイル（overview, avatar-parameters, avatar-scaling, chatbox, input-controller, trackers, eye-tracking, debugging, oscquery, resources, diy）
  - REST API: 16ファイル（overview, user, world, avatar, friends, notifications, instances, groups, favorites, invites, player-moderation, economy, files, calendar, inventory, props, jams, prints, miscellaneous）
  - WebSocket: 1ファイル（pipeline）
  - メタ: 2ファイル（index, changelog）
- `osc/chatbox.md`: 対象URL `docs.vrchat.com/docs/osc-chatbox` が404のため、`osc-as-input-controller` ページの情報を元に構成
- REST API全ファイル: `vrchat.community` および `github.com/vrchatapi/specification` v1.20.7 を参照

## 記録フォーマット

```
## YYYY-MM-DD

- **変更内容**: 何が変わったか
- **影響ファイル**: 変更が必要なファイルとその対応
- **ソース**: 変更を確認した出典URL
```
