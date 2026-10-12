---
scope: osc
title: OSC Eye Tracking
source: https://docs.vrchat.com/docs/osc-eye-tracking
status: verified
last_verified: 2026-10-09
---

# OSC Eye Tracking

## 概要

外部アイトラッキングデバイスのデータをOSC経由でVRChatに送信できます。
まぶたの開閉量と視線方向の2種類のデータを送信します。

## エンドポイント一覧

### まぶた

| アドレス | 型 | 値の範囲 | 説明 |
|---|---|---|---|
| `/tracking/eye/EyesClosedAmount` | Float | 0.0–1.0 | 両眼の閉じ具合（0=全開, 1=全閉）。現時点では両眼同時に単一の値で制御する方式のみ対応（左右個別制御は非対応） |

### 視線方向（いずれか1つを選択して使用）

| アドレス | 引数 | 説明 |
|---|---|---|
| `/tracking/eye/CenterPitchYaw` | Float × 2（Pitch, Yaw） | 中央視線のピッチ・ヨー（度）。距離なし、レイキャストで収束距離を自動計算 |
| `/tracking/eye/CenterPitchYawDist` | Float × 3（Pitch, Yaw, Dist） | 中央視線のピッチ・ヨー（度）と距離（メートル） |
| `/tracking/eye/CenterVec` | Float × 3（X, Y, Z） | 中央視線の正規化ベクトル |
| `/tracking/eye/CenterVecFull` | Float × 3（X, Y, Z） | 中央視線ベクトル（HMDローカル座標、非正規化）。ベクトルの長さ（メートル）が収束距離を決定 |
| `/tracking/eye/LeftRightPitchYaw` | Float × 4（左Pitch, 左Yaw, 右Pitch, 右Yaw） | 左右それぞれの視線ピッチ・ヨー（度） |
| `/tracking/eye/LeftRightVec` | Float × 6（左X, Y, Z, 右X, Y, Z） | 左右それぞれの視線ベクトル |

## 値の例

| アドレス | 値の例 |
|---|---|
| `CenterPitchYaw` | `-15.252, 20.128` |
| `CenterPitchYawDist` | `-15.252, 20.128, 0.503` |
| `CenterVec` | `0.332, 0.263, 0.905` |
| `CenterVecFull` | `0.167, 0.132, 0.456` |
| `LeftRightPitchYaw` | `-14.903, 23.592, -15.560, 16.503` |
| `LeftRightVec` | `0.387, 0.257, 0.886, 0.274, 0.268, 0.923` |

## 重要な制限事項

- **アドレスは大文字小文字を区別します**（例：`CenterPitchYaw` と `centerpitchyaw` は別物）。
- データ受信がない状態が **10秒** 続くと、アイトラッキングがデフォルト動作（アバター内蔵の瞬き等）に戻ります。
- 視線方向アドレスは**同時に1つのみ使用可能**です（複数のアドレスに同時送信しないこと）。
- 一般的に、pitch・yawの値が正の場合はそれぞれ下方向・右方向への回転を意味します。
