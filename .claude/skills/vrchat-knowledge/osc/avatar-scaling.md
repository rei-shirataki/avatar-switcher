---
scope: osc
title: OSC Avatar Scaling
source: https://docs.vrchat.com/docs/osc-avatar-scaling
status: verified
last_verified: 2026-10-09
---

# OSC Avatar Scaling

## 概要

OSCを使用してアバターの目の高さ（アイハイト）を読み取り・設定できます。

## エンドポイント一覧

| アドレス | 型 | 方向 | 説明 |
|---|---|---|---|
| `/avatar/eyeheight` | Float | 読み書き | 現在の目の高さ（メートル） |
| `/avatar/eyeheightmin` | Float | 読み取り | ユーザーが選択できる最小値（Udon設定） |
| `/avatar/eyeheightmax` | Float | 読み取り | ユーザーが選択できる最大値（Udon設定） |
| `/avatar/eyeheightscalingallowed` | Bool | 読み取り | スケール変更が許可されているか |

---

## `/avatar/eyeheight`

### 仕様

| 項目 | 値 |
|---|---|
| 型 | Float |
| 単位 | メートル |
| 最小値 | 0.01 m（1 cm） |
| 最大値 | 10,000 m（10 km） |
| 公式サポート範囲 | 0.1 m 〜 100 m |

### 動作

- **読み取り / イベント**: ユーザー・Udon・アバター切り替えによってスケールが変化したとき、現在のアイハイトが送信されます。
- **書き込み**: メートル値を送ることでアイハイトを設定できます。

### 注意事項

- 公式サポート範囲（0.1m〜100m）を超えた値を設定するとHUD上に警告が表示されます。
- 範囲外の値はサポート外のため、不具合が発生しても対応されない場合があります。

---

## `/avatar/eyeheightmin` / `/avatar/eyeheightmax`

| 項目 | 最小値 | 最大値 |
|---|---|---|
| デフォルト | 0.2 m | 5.0 m |
| 設定元 | Udon スクリプト | Udon スクリプト |
| OSC書き込みへの影響 | なし（OSCは制限を受けない） | なし（OSCは制限を受けない） |

ユーザーがスライダーで選択できる範囲を制限しますが、OSCによる書き込みはこの制限を受けません。
これらのアドレス自体は読み取り専用（VRChat → 外部への通知）であり、外部から書き込んでも値を変更することはできません。

---

## `/avatar/eyeheightscalingallowed`

- `false` のとき、`/avatar/eyeheight` への書き込みは無視されます。
- Udonスクリプトまたはワールドタグによって無効化されることがあります。

---

## 注意事項

- Udonスクリプトがスケール制限を強制している場合、OSCで設定した値が上書きされ、
  実際に適用された値が二次イベントとして送信されることがあります。
