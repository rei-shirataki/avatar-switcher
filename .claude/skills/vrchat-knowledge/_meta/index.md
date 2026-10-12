---
scope: meta
title: VRChat Knowledge Base — 全体インデックス
source: internal
status: verified
last_verified: 2026-10-09
---

# VRChat Knowledge Base — 全体インデックス

VRChatの外部連携（OSC・REST API・WebSocket）に関するナレッジベース。

## セクション構成

### OSC（公式サポート）

ソース: `docs.vrchat.com` / `wiki.vrchat.com`  
信頼度: **verified**（公式ドキュメント）

| ファイル | 内容 |
|---|---|
| [osc/overview.md](../osc/overview.md) | ポート設定・有効化・推奨ツール |
| [osc/avatar-parameters.md](../osc/avatar-parameters.md) | `/avatar/parameters/{name}` の制御・Built-inパラメータ30種 |
| [osc/avatar-scaling.md](../osc/avatar-scaling.md) | `/avatar/eyeheight` によるスケール調整 |
| [osc/chatbox.md](../osc/chatbox.md) | `/chatbox/input`・`/chatbox/typing` |
| [osc/input-controller.md](../osc/input-controller.md) | Axes 9種・Buttons 20種 |
| [osc/trackers.md](../osc/trackers.md) | 外部トラッカー（位置・回転）の送信 |
| [osc/eye-tracking.md](../osc/eye-tracking.md) | アイトラッキングデータ送信 |
| [osc/debugging.md](../osc/debugging.md) | OSCデバッグ画面 |
| [osc/oscquery.md](../osc/oscquery.md) | OSCQuery自動検出プロトコル |
| [osc/resources.md](../osc/resources.md) | ツール・ライブラリ一覧 |
| [osc/diy.md](../osc/diy.md) | カスタム実装ガイド・OSCメッセージ構造・推奨ライブラリ |
| [osc/usercamera.md](../osc/usercamera.md) | ユーザーカメラの制御（`/usercamera/*`）。出典は VRChat Wiki |
| [osc/dolly.md](../osc/dolly.md) | Dolly（カメラパス）の操作（`/dolly/*`）。出典は VRChat Wiki |

### REST API（非公式・コミュニティ）

ソース: `vrchat.community` / `github.com/vrchatapi/specification`  
信頼度: **community**（非公式・予告なく変更される可能性あり）

> ⚠️ 乱用するとアカウント停止のリスクあり

| ファイル | 内容 |
|---|---|
| [api/overview.md](../api/overview.md) | ベースURL・認証フロー・レート制限・SDK一覧 |
| [api/user.md](../api/user.md) | ユーザー検索・取得・更新 |
| [api/world.md](../api/world.md) | ワールド検索・作成・管理 |
| [api/avatar.md](../api/avatar.md) | アバター検索・作成・装着 |
| [api/friends.md](../api/friends.md) | フレンド申請・フレンド状態確認・unfriend |
| [api/notifications.md](../api/notifications.md) | 通知管理・フレンド申請の承認フロー（v1/v2） |
| [api/instances.md](../api/instances.md) | インスタンス作成・取得・閉鎖・タイプ一覧・インスタンスID構造・リージョン・カテゴリ/バイブ |
| [api/groups.md](../api/groups.md) | グループ全操作（メンバー・ロール・招待・投稿等、53エンドポイント） |
| [api/favorites.md](../api/favorites.md) | お気に入り（ワールド・アバター・フレンド）・グループ管理 |
| [api/invites.md](../api/invites.md) | 招待送受信・招待メッセージ（スロット）管理 |
| [api/player-moderation.md](../api/player-moderation.md) | ミュート・ブロック・アバター非表示等のモデレーション |
| [api/economy.md](../api/economy.md) | VRC+・クレジット・商品/ストア・購入履歴・残高・Tilia決済連携・収益メトリクス |
| [api/files.md](../api/files.md) | アセットファイルアップロードフロー・バージョン管理 |
| [api/calendar.md](../api/calendar.md) | グループカレンダーイベント・ICSダウンロード |
| [api/inventory.md](../api/inventory.md) | コレクタブルアイテム・ドロップ・装備管理 |
| [api/props.md](../api/props.md) | インスタンス内スポーンアイテム（Prop）管理 |
| [api/jams.md](../api/jams.md) | 創作コンテスト（Jam）一覧・投稿管理 |
| [api/prints.md](../api/prints.md) | VRChatカメラ写真（Print）アップロード・管理 |
| [api/miscellaneous.md](../api/miscellaneous.md) | システム設定・ヘルスチェック・オンライン人数・パーミッション・ベータプログラム・フロントエンドブランチ |
| [api/tags.md](../api/tags.md) | ユーザー・ワールド・グループ・言語タグの意味 |
| [api/shortlinks.md](../api/shortlinks.md) | `vrch.at` / `vrc.group` の短縮リンク・リダイレクト・グループコードからのID解決 |

### WebSocket（非公式・コミュニティ）

ソース: `vrchat.community`  
信頼度: **community**

| ファイル | 内容 |
|---|---|
| [websocket/pipeline.md](../websocket/pipeline.md) | Pipeline接続・全イベント種別 |

## status フィールドの意味

| 値 | 意味 |
|---|---|
| `verified` | 公式ドキュメントと照合済み |
| `community` | コミュニティドキュメント。非公式・変更リスクあり |
| `unverified` | 未検証。実装前に要確認 |

## 関連リンク

| リソース | URL |
|---|---|
| VRChat OSC公式ドキュメント | https://docs.vrchat.com/docs/osc-overview |
| VRChat Wiki | https://wiki.vrchat.com/wiki/Open_Sound_Control |
| vrchat.community API ドキュメント | https://vrchat.community/ |
| OpenAPI Specification | https://github.com/vrchatapi/specification |
