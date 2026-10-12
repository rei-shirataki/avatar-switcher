---
scope: osc
title: OSC Trackers
source: https://docs.vrchat.com/docs/osc-trackers
status: verified
last_verified: 2026-10-09
---

# OSC Trackers

## 概要

OSCを使用して外部トラッカーの位置・回転データをVRChatに送信できます。
最大8つのトラッカー（ヒップ・チェスト・両足・両膝・両肘）と頭部のトラッキングをサポートしています。

## OSCアドレス

### トラッカー（番号指定）

| アドレス | 型 | 説明 |
|---|---|---|
| `/tracking/trackers/{1-8}/position` | Float × 3（X, Y, Z） | トラッカーの位置 |
| `/tracking/trackers/{1-8}/rotation` | Float × 3（X, Y, Z） | トラッカーの回転（オイラー角） |

### 頭部

| アドレス | 型 | 説明 |
|---|---|---|
| `/tracking/trackers/head/position` | Float × 3（X, Y, Z） | 頭部の位置 |
| `/tracking/trackers/head/rotation` | Float × 3（X, Y, Z） | 頭部の回転（オイラー角） |

### システムポーズ（読み取り専用）

VRChat からの出力で、`Float × 6`（X, Y, Z の位置、X, Y, Z のオイラー角）です。公式ドキュメントのページには載っておらず、クライアントの OSCQuery 応答にのみ存在します（出典: [wiki.vrchat.com/wiki/OSC](https://wiki.vrchat.com/wiki/OSC)）。

| アドレス | 説明 |
|---|---|
| `/tracking/vrsystem/head/pose` | ヘッドセットのシステムポーズ |
| `/tracking/vrsystem/leftwrist/pose` | 左手首のシステムポーズ |
| `/tracking/vrsystem/rightwrist/pose` | 右手首のシステムポーズ |

トラッカーは最大8個（腰・胸・両足・両ひざ・両ひじ）で、台数を減らす（例: 腰と足だけ）ほうが、VRChat の IK が誤差を補正しやすく結果が良くなることがあります。

## 座標系

| 項目 | 仕様 |
|---|---|
| 上方向 | +Y軸 |
| スケール | 1.0 = 1メートル（現実空間と対応） |
| 利き手 | 左手座標系 |
| 回転表現 | オイラー角（度数法） |
| 回転適用順 | Z → X → Y |

## 頭部データの動作

- **位置**: `/tracking/trackers/head/position` に合わせてOSCトラッキング空間全体がシフトされ、毎フレーム完全に（補間なしで）同期されます。
- **回転**: `/tracking/trackers/head/rotation` はヨー（yaw）方向のアライメントに使用され、通常はトラッキング空間のヨーが目標回転へ徐々に lerp（線形補間）されます。
  - ただし、連続送信ではなく単発のメッセージ（300ms以内に次のメッセージが来ない場合）と判定された場合は、lerpではなく一度だけ即座にアライメントされます。
  - 300ms以内に次の頭部回転メッセージが届いた場合は連続的なストリーミングデータとみなされます。

## 推奨設定

- 全8トラッカーを常に送信することが最適とは限りません。
- まずは**足とヒップのみ**送信し、トラッキング精度が高い場合に段階的にトラッカー数を増やすことを推奨します。
