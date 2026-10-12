---
name: VRChat Knowledge Base
description: VRChatの外部連携仕様(OSC・REST API・WebSocket)に関するナレッジベース。VRChat OSCの送受信実装(Avatar Parameters、Chatbox、Input Controller、Eye Tracking、トラッカー統合、アバタースケール、OSCQuery、User Camera、Dolly)、VRChat REST API呼び出し実装(ユーザー・ワールド・アバター・フレンド・グループ・招待・通知・インスタンス・お気に入り・プレイヤーモデレーション・Economy・ファイルアップロード・カレンダー・インベントリ・Props・Jams・Prints、タグ、短縮リンク)、VRChat WebSocket/Pipelineイベント購読、OyasumiVR・Pulsoid・VRCXなどのVRChat連携ツール開発、VRChatボット・自動化スクリプト開発の際に使用する。
---

# VRChat Knowledge Base — SKILL

## このスキルを使うタイミング

以下のキーワードや状況が含まれる場合、このスキルを参照すること：

- VRChat OSC（送受信、Avatar Parameters、Chatbox、Input、Eye Tracking、トラッカー、User Camera、Dolly）
- VRChat API / REST API（ユーザー情報取得・更新、ワールド、認証、フレンド、グループ、招待、通知、インスタンス、お気に入り、プレイヤーモデレーション、Economy・クレジット、ファイルアップロード、カレンダー、インベントリ、Props、Jams、Prints、システム設定、タグ、短縮リンク）
- VRChat WebSocket / Pipeline イベント
- OyasumiVR、Pulsoid、VRCX などの VRChat 連携ツール
- VRChat のボット・自動化スクリプト開発

## 構成ファイル

全ファイルの索引・詳細な内容説明は **[`_meta/index.md`](_meta/index.md) を参照**（このファイルでは重複させない）。

| セクション | ディレクトリ | 信頼度 |
|---|---|---|
| OSC（公式サポート） | `osc/` （13ファイル） | ✅ verified |
| REST API（非公式） | `api/` （21ファイル） | ✅ community |
| WebSocket（非公式） | `websocket/` （1ファイル） | ✅ community |
| メタ情報 | `_meta/index.md`, `_meta/changelog.md` | ✅ verified |

## 重要な前提知識

- **OSC** は VRChat 公式サポート（`docs.vrchat.com` / `wiki.vrchat.com`）
- **REST API** は非公式・コミュニティドキュメント（`vrchat.community`）
- REST API は予告なく変更される可能性があり、乱用するとアカウント停止のリスクがある
- ドキュメントの `status` フィールドで信頼度を確認すること

## AI エージェントへの指示

1. `_meta/index.md` が存在すれば最初に読む。なければ `osc/overview.md` などセクションの overview から読む
2. タスクに関連するセクションファイルを参照する
3. `status: unverified` または `status: community` の情報は実装前に確認を促す
4. コード例を提示するとき、そのコード例が属するスコープ（`scope: osc` / `scope: api` / `scope: websocket`）が今解こうとしている問題のスコープと一致していることを確認する。OSCのコード例をAPI実装に流用しないこと。`status: verified` のファイルのコード例を `status: community` より優先するのはスコープが一致する場合に限る
