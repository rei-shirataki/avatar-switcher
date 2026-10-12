---
scope: osc
title: OSC as Input Controller
source: https://docs.vrchat.com/docs/osc-as-input-controller
status: verified
last_verified: 2026-10-09
---

# OSC as Input Controller

## 概要

OSCを使用してVRChatの入力（移動・ジャンプ・視点・インタラクション等）を外部から制御できます。
ヘッドトラッキング・ブレインウェーブ・楽器シーケンサーなど多様な入力デバイスに対応します。

## 入力タイプ

| タイプ | 型 | 値の範囲 | 注意 |
|---|---|---|---|
| **Axes** | Float | -1.0 〜 1.0 | 非使用時は必ず `0` にリセットすること |
| **Buttons** | Int | 0 or 1 | 毎回 `0` へリセットしないと連続入力にならない |

---

## Axes（軸入力）

| アドレス | 値 | 説明 |
|---|---|---|
| `/input/Vertical` | -1〜1 | 前進（1）/ 後退（-1） |
| `/input/Horizontal` | -1〜1 | 右移動（1）/ 左移動（-1） |
| `/input/LookHorizontal` | -1〜1 | 左右視点移動。VRでコンフォートターン有効時はスナップターン |
| `/input/UseAxisRight` | -1〜1 | 右手アイテム使用（動作未確認） |
| `/input/GrabAxisRight` | -1〜1 | 右手アイテム掴む（動作未確認） |
| `/input/LookVertical` | -1〜1 | 上下視点移動（上=1 / 下=-1）。※公式ドキュメントのページには載っておらず、クライアントの OSCQuery 応答にのみ存在（wiki.vrchat.com より） |
| `/input/MoveHoldFB` | -1〜1 | 保持オブジェクトを前後に移動 |
| `/input/SpinHoldCwCcw` | -1〜1 | 保持オブジェクトを時計回り（1）/ 反時計回り（-1）に回転 |
| `/input/SpinHoldUD` | -1〜1 | 保持オブジェクトを上下に回転 |
| `/input/SpinHoldLR` | -1〜1 | 保持オブジェクトを左右に回転 |

---

## Buttons（ボタン入力）

### 移動・カメラ

| アドレス | 対応 | 説明 |
|---|---|---|
| `/input/MoveForward` | 共通 | 前進（1で継続） |
| `/input/MoveBackward` | 共通 | 後退（1で継続） |
| `/input/MoveLeft` | 共通 | 左ストレイフ（1で継続） |
| `/input/MoveRight` | 共通 | 右ストレイフ（1で継続） |
| `/input/LookLeft` | 共通 | 左ターン（1の間）。Desktop=スムーズ、VR=コンフォートターン有効時のみスナップターン |
| `/input/LookRight` | 共通 | 右ターン（1の間）。Desktop=スムーズ、VR=コンフォートターン有効時のみスナップターン |
| `/input/ComfortLeft` | VR専用 | 左スナップターン |
| `/input/ComfortRight` | VR専用 | 右スナップターン |

### アクション

| アドレス | 対応 | 説明 |
|---|---|---|
| `/input/Jump` | 共通 | ジャンプ（ワールド対応時のみ） |
| `/input/Run` | 共通 | 走る（ワールド対応時のみ） |
| `/input/PanicButton` | 共通 | セーフモード有効化 |
| `/input/Voice` | 共通 | ボイス切り替え（設定により動作が異なる） |

### インタラクション（VR専用）

| アドレス | 説明 |
|---|---|
| `/input/GrabRight` | 右手でハイライトアイテムを掴む |
| `/input/UseRight` | 右手でハイライトアイテムを使用 |
| `/input/DropRight` | 右手のアイテムを放す |
| `/input/GrabLeft` | 左手でハイライトアイテムを掴む |
| `/input/UseLeft` | 左手でハイライトアイテムを使用 |
| `/input/DropLeft` | 左手のアイテムを放す |

### その他（OSCQuery 応答のみに存在）

以下は公式ドキュメントのページには載っておらず、クライアントの OSCQuery 応答にのみ存在します（出典: [wiki.vrchat.com/wiki/OSC](https://wiki.vrchat.com/wiki/OSC)）。

| アドレス | 説明 |
|---|---|
| `/input/ToggleSitStand` | 座り／立ちの切り替え |
| `/input/AFKToggle` | AFK モードの切り替え |

### ワールドデバッグビュー（`/input/ShowDebugInfo0`〜`9`）

いずれも Bool のボタン入力（write-only）です。

| アドレス | 表示内容 |
|---|---|
| `/input/ShowDebugInfo0` ※ | VRC_UiShapes（操作可能な UI）のデバッグ可視化（OSCQuery 応答のみ。ワールド作成者限定⁑） |
| `/input/ShowDebugInfo1` | 読み込まれているアセットバンドルの情報（ワールド開発にはほぼ無関係） |
| `/input/ShowDebugInfo2` | 使用中の VRChat のビルドと FPS |
| `/input/ShowDebugInfo3` | 出力ログ |
| `/input/ShowDebugInfo4` | 他ユーザーに関する各種統計 |
| `/input/ShowDebugInfo5` | ネットワーク関連のグラフ |
| `/input/ShowDebugInfo6` | ワールド内の全ネットワークオブジェクトと各種統計⁑ |
| `/input/ShowDebugInfo7` | ローカルプレイヤー付近の全 PhysBone と Contact の可視化⁑ |
| `/input/ShowDebugInfo8` | 同期されている全オブジェクトの上にパネルを重ねて、各オブジェクトの統計を表示⁑ |
| `/input/ShowDebugInfo9` | インスタンス内の全プレイヤーの足元にデバッグ統計を表示⁑ |

⁑ ワールド作成者のみ使用可能。Web サイトで「World Debugging」を有効にした場合は他のユーザーも使用できます。

### メニュー

| アドレス | 説明 |
|---|---|
| `/input/QuickMenuToggleLeft` | クイックメニューの表示切り替え（0→1で実行） |
| `/input/QuickMenuToggleRight` | クイックメニューの表示切り替え（0→1で実行） |

---

## Voice の動作詳細

| 設定 | 動作 |
|---|---|
| 「Toggle Voice」が有効 | 0→1 でミュート状態をトグル。その後 0 に戻す必要あり |
| 「Toggle Voice」が無効 | プッシュトゥミュート動作。0=ミュート、1=送話中 |

---

## Chatbox

| アドレス | 引数 | 説明 |
|---|---|---|
| `/chatbox/input` | `(String, Bool, Bool?)` | テキスト送信（詳細は [chatbox.md](chatbox.md)） |
| `/chatbox/typing` | `(Bool)` | タイピングインジケーターの制御 |

詳細は [chatbox.md](chatbox.md) を参照してください。
