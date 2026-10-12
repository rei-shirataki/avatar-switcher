---
scope: osc
title: OSC User Camera
source: https://wiki.vrchat.com/wiki/OSC
status: verified
last_verified: 2026-10-09
---

# OSC User Camera

## 概要

VRChat のユーザーカメラ（写真撮影・配信用カメラ）を OSC で制御・監視します。`/usercamera/*` 配下のエンドポイントは、特に記載がなければ**読み書き可能**です。

> **ソースに関する注意**: このページは `docs.vrchat.com` の OSC ページ群には載っておらず、[VRChat Wiki の OSC ページ](https://wiki.vrchat.com/wiki/OSC)（VRCWiki チーム承認の公式情報ページ）にのみ記載があります。ポートや OSC メッセージの型は [overview.md](overview.md) を参照してください。

## モードとポーズ

| アドレス | 型 | 説明 |
|---|---|---|
| `/usercamera/Mode` | Int | カメラモード |
| `/usercamera/Pose` | Float × 6 | カメラの位置と回転（**読み取り専用**）。形式は `/tracking/vrsystem/head/pose` と同じ（X, Y, Z の位置 + X, Y, Z のオイラー角。[trackers.md](trackers.md)） |

`/usercamera/Mode` の値:

| 値 | モード |
|---|---|
| 0 | Off |
| 1 | Photo |
| 2 | Stream |
| 3 | Emoji |
| 4 | Multilayer |
| 5 | Print |
| 6 | Drone |

## アクション

いずれも Bool、**write-only** のボタン入力です（`1`/`true` を送ったあと `0`/`false` に戻す。詳細は [input-controller.md](input-controller.md) のボタンの扱いと同じ）。

| アドレス | 説明 |
|---|---|
| `/usercamera/Close` | カメラを閉じる（`/usercamera/Mode ,i 0` と同等） |
| `/usercamera/Capture` | 写真を撮る |
| `/usercamera/CaptureDelayed` | タイマー付きで写真を撮る |

## トグル（Bool）

| アドレス | 説明 |
|---|---|
| `/usercamera/ShowUIInCamera` | UI マスク |
| `/usercamera/LocalPlayer` | ローカルプレイヤーのマスク |
| `/usercamera/RemotePlayer` | リモートプレイヤーのマスク |
| `/usercamera/Environment` | 環境のマスク |
| `/usercamera/GreenScreen` | グリーンスクリーン |
| `/usercamera/Lock` | カメラのロック |
| `/usercamera/SmoothMovement` | スムーズな動き |
| `/usercamera/LookAtMe` | Look-At-Me の挙動 |
| `/usercamera/AutoLevelRoll` | ロールの自動水平化 |
| `/usercamera/AutoLevelPitch` | ピッチの自動水平化 |
| `/usercamera/Flying` | 飛行 |
| `/usercamera/TriggerTakesPhotos` | トリガーで撮影 |
| `/usercamera/DollyPathsStayVisible` | アニメーション中も Dolly のパスを表示（[dolly.md](dolly.md)） |
| `/usercamera/AudioFromCamera` | カメラからの音声 |
| `/usercamera/ShowFocus` | フォーカスのオーバーレイ |
| `/usercamera/Streaming` | Spout ストリーム |
| `/usercamera/RollWhileFlying` | 飛行中のロール |
| `/usercamera/OrientationIsLandscape` | 画像の向き（横向きか） |

## スライダー（Float）

| アドレス | 説明 | 既定値 | 最小 | 最大 |
|---|---|---|---|---|
| `/usercamera/Zoom` | ズーム | 45 | 20 | 150 |
| `/usercamera/Exposure` | 露出 | 0 | -10 | 4 |
| `/usercamera/FocalDistance` | 焦点距離 | 1.5 | 0 | 10 |
| `/usercamera/Aperture` | 絞り | 15 | 1.4 | 32 |
| `/usercamera/Hue` | グリーンスクリーンの色相 | 120 | 0 | 360 |
| `/usercamera/Saturation` | グリーンスクリーンの彩度 | 100 | 0 | 100 |
| `/usercamera/Lightness` | グリーンスクリーンの明度 | 60 | 0 | 50（※出典の表記。既定値 60 が最大値 50 を超えており矛盾しているため、実際の範囲は要確認） |
| `/usercamera/LookAtMeXOffset` | Look-At-Me の X オフセット | 0 | -25 | 25 |
| `/usercamera/LookAtMeYOffset` | Look-At-Me の Y オフセット | 0 | -25 | 25 |
| `/usercamera/FlySpeed` | 飛行速度 | 3 | 0.1 | 15 |
| `/usercamera/TurnSpeed` | 旋回速度 | 1 | 0.1 | 5 |
| `/usercamera/SmoothingStrength` | スムージングの強さ | 5 | 0.1 | 10 |
| `/usercamera/PhotoRate` | Dolly 撮影レート | 1 | 0.1 | 2 |
| `/usercamera/Duration` | Dolly の所要時間 | 2 | 0.1 | 60 |

## 関連

- [Dolly（カメラパスのアニメーション）](dolly.md)
- [Prints（カメラ写真の REST API）](../api/prints.md)
