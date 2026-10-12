---
scope: osc
title: OSC Chatbox
source: https://docs.vrchat.com/docs/osc-as-input-controller
status: verified
last_verified: 2026-10-09
---

# OSC Chatbox

## 概要

OSCを使用してVRChatのチャットボックスにテキストを送信したり、タイピングインジケーターを制御したりできます。

> **参照元について**: Chatbox の仕様は `https://docs.vrchat.com/docs/osc-as-input-controller` に記載されています（独立したページは存在しません）。

## エンドポイント一覧

| アドレス | 方向 | 説明 |
|---|---|---|
| `/chatbox/input` | Input（外部 → VRChat） | テキストをチャットボックスに送信 |
| `/chatbox/typing` | Input（外部 → VRChat） | タイピングインジケーターを制御 |

---

## `/chatbox/input` — テキスト送信

### パラメータ

| 引数 | 型 | 必須 | 説明 |
|---|---|---|---|
| 第1引数 | String | 必須 | 送信するテキスト内容 |
| 第2引数 | Bool | 必須 | `true` = 即時送信 / `false` = キーボードを開いてテキストをセット |
| 第3引数 | Bool | 任意 | 通知音を鳴らすか（省略時: `true`） |

### 制限事項

- テキストは最大 **144文字**
- チャットボックスに表示される行数は最大 **9行**（改行・ワードラップを含む）

### 動作の詳細

- 第2引数が `true` の場合：テキストがそのまま即時送信されます。
- 第2引数が `false` の場合：VRChatのキーボードUIが開き、指定テキストが入力欄にあらかじめセットされます（ユーザーが確認・編集してから送信）。

### 使用例

```
# 即時送信（通知音あり）
/chatbox/input "Hello, world!" true

# キーボードを開いてテキストをセット
/chatbox/input "Hello, world!" false

# 即時送信（通知音なし）
/chatbox/input "Hello, world!" true false
```

---

## `/chatbox/typing` — タイピングインジケーター

### パラメータ

| 引数 | 型 | 必須 | 説明 |
|---|---|---|---|
| 第1引数 | Bool | 必須 | `true` = インジケーター表示 / `false` = 非表示 |

### 使用例

```
# タイピング中を表示
/chatbox/typing true

# タイピング終了（非表示）
/chatbox/typing false
```
