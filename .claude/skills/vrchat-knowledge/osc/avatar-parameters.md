---
scope: osc
title: OSC Avatar Parameters
source: https://docs.vrchat.com/docs/osc-avatar-parameters
status: verified
last_verified: 2026-10-09
---

# OSC Avatar Parameters

## 概要

OSC（Open Sound Control）を使用してアバターパラメータを外部から制御・監視できます。
OSCが有効な状態で新しいアバターがロードされると、アバターIDが通知されます。

## アバター変更通知

| アドレス | 型 | 方向 | 説明 |
|---|---|---|---|
| `/avatar/change` | String | Output（VRChat → 外部） | アバターロード時にアバターIDを送信 |

## パラメータの入出力

### 入力（外部 → VRChat）

```
/avatar/parameters/{name}
```

このアドレスにOSCメッセージを送ることで、対応するアバターパラメータを設定できます。

### 出力（VRChat → 外部）

パラメータの値が変化すると、設定されたアドレスへ値が送信されます。

### 対応データ型

| 型 | 説明 |
|---|---|
| `Int` | 整数値 |
| `Bool` | 真偽値 |
| `Float` | 浮動小数点数 |

## 設定ファイル

OSC有効時にVRChatがJSONコンフィグを自動生成します。このファイルを編集することで、
パラメータのOSCアドレスのリマップやデータ型の変換が可能です。

### ファイルパス

```
~\AppData\LocalLow\VRChat\VRChat\OSC\{userId}\Avatars\{avatarId}.json
```

### フォーマット例

```json
{
    "id": "avtr_9d58037b-23c7-4c9c-adbd-b1338178cd81",
    "name": "PurpleMomo",
    "parameters": [
        {
            "name": "Face",
            "input": {
                "address": "/avatar/parameters/Face",
                "type": "Int"
            },
            "output": {
                "address": "/ableton/trackselect",
                "type": "Float"
            }
        },
        {
            "name": "VelocityZ",
            "output": {
                "address": "/avatar/parameters/VelocityZ",
                "type": "Float"
            }
        }
    ]
}
```

### フィールド説明

| フィールド | 説明 |
|---|---|
| `id` | アバターID |
| `name` | アバター名 |
| `parameters[].name` | アバターパラメータ名（実際のパラメータ名と完全一致が必要） |
| `parameters[].input.address` | 受信するOSCアドレス |
| `parameters[].input.type` | 受信データ型（`Int` / `Bool` / `Float`） |
| `parameters[].output.address` | 送信先OSCアドレス |
| `parameters[].output.type` | 送信データ型（`Int` / `Bool` / `Float`） |

`input` / `output` はどちらか一方のみの設定も可能です。  
型変換も自動で行われます（例：`Int` で受け取り `Float` として外部アプリへ送信）。

## Built-in パラメータ

VRChat SDKが自動で設定する組み込みパラメータの一覧です。  
これらはコンフィグファイルなしでも `/avatar/parameters/{name}` で送受信できます。

| パラメータ名 | 型 | 値の範囲 | 説明 |
|---|---|---|---|
| `IsLocal` | Bool | true / false | ローカルで装着されているアバターか |
| `PreviewMode` | Int | 0–1 | アバタープレビューモード中か |
| `Viseme` | Int | 0–14 | 音声同期（リップシンク）インデックス |
| `Voice` | Float | 0.0–1.0 | マイク音量レベル |
| `GestureLeft` | Int | 0–7 | 左手ジェスチャー |
| `GestureRight` | Int | 0–7 | 右手ジェスチャー |
| `GestureLeftWeight` | Float | 0.0–1.0 | 左トリガーのアナログ値 |
| `GestureRightWeight` | Float | 0.0–1.0 | 右トリガーのアナログ値 |
| `AngularY` | Float | 連続値 | Y軸まわりの角速度 |
| `VelocityX` | Float | 連続値（m/s） | 横方向（左右）の移動速度 |
| `VelocityY` | Float | 連続値（m/s） | 垂直方向の移動速度 |
| `VelocityZ` | Float | 連続値（m/s） | 前方向の移動速度 |
| `VelocityMagnitude` | Float | 連続値 | 速度ベクトルの大きさ |
| `Upright` | Float | 0.0–1.0 | 直立度（0=水平, 1=直立） |
| `Grounded` | Bool | true / false | 地面に接触しているか |
| `Seated` | Bool | true / false | ステーション（椅子等）を使用しているか |
| `AFK` | Bool | true / false | AFK（離席）状態か |
| `TrackingType` | Int | 0–6 | トラッキング方式の種類 |
| `VRMode` | Int | 0–1 | VRモードか（1=VR, 0=Desktop） |
| `MuteSelf` | Bool | true / false | 自分の音声をミュートしているか |
| `InStation` | Bool | true / false | ステーションに乗っているか |
| `Earmuffs` | Bool | true / false | イヤーマフ（音量制限）が有効か |
| `IsOnFriendsList` | Bool | true / false | ローカルプレイヤーのフレンドリストにいるか |
| `AvatarVersion` | Int | 0 or 3 | SDK版本（SDKv2=0, SDKv3=3） |
| `IsAnimatorEnabled` | Bool | true / false | アニメーターが有効か |
| `ScaleModified` | Bool | true / false | アバタースケールが変更されているか |
| `ScaleFactor` | Float | 連続値 | 現在のスケール係数 |
| `ScaleFactorInverse` | Float | 連続値 | スケール係数の逆数 |
| `EyeHeightAsMeters` | Float | 連続値 | 目の高さ（メートル） |
| `EyeHeightAsPercent` | Float | 連続値 | 目の高さ（デフォルトに対する相対比率） |

### 補足（wiki.vrchat.com）

出典: [wiki.vrchat.com/wiki/OSC](https://wiki.vrchat.com/wiki/OSC)

- カスタムのアバターパラメータはすべて読み書き可能ですが、組み込み（予約）パラメータは**多くが読み取り専用**です
- 次の2つ＋表情ブレンドは、厳密には必須の組み込みではないものの、デフォルトのアニメーターに含まれ、ほぼ常に存在します

| パラメータ名 | 型 | 説明 |
|---|---|---|
| `VRCEmote` | Int | デフォルトのアニメーション（1〜16） |
| `VRCFaceBlendV` | Float | 表情ブレンド（縦） |
| `VRCFaceBlendH` | Float | 表情ブレンド（横） |

- `PreviewMode`: メニューのアバタープレビュー中は 1、それ以外は 0（**OSC 経由では常に 0**）
- `IsOnFriendsList`: ローカルユーザーがそのアバターのリモートユーザーとフレンドか（**OSC 経由では常に false**）
- `Seated`: `InStation` と同じ値の重複パラメータ。「シーテッドモード／スタンディングモード」とは別物
- `AvatarVersion`: SDK3 で作られていれば 3、そうでなければ 0
- `VelocityMagnitude` = √(VelocityX² + VelocityY² + VelocityZ²)
- `EyeHeightAsPercent`: 目の高さを既定の範囲で線形補間した値（例: 0.0 = 0.2 m、0.5 = 2.6 m、1.0 = 5.0 m）
- `AngularY`: 右向きの角速度、`VelocityX/Y/Z`: 右／上／前向きの速度（m/s）

## 注意事項

- コンフィグファイルシステムは暫定実装です。将来的にクライアント内UIに置き換えられる予定のため、
  仕様が変更される可能性があります。
- コンフィグファイルを編集することで、外部アプリ（Ableton Live 等）との連携において
  追加ソフトウェアなしにパラメータをリマップできます。
