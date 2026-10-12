---
title: OSC Overview
scope: osc
source: https://docs.vrchat.com/docs/osc-overview
source_wiki: https://wiki.vrchat.com/wiki/Open_Sound_Control
status: verified
last_verified: 2026-10-09
---

# VRChat OSC — 概要

## このファイルでわかること
VRChat の OSC 機能の有効化方法、ポート設定、基本的な通信モデル。

## 概要

OSC（Open Sound Control）は音声機器間通信のために設計されたプロトコル。
VRChat では 2022年2月から OSC サポートを導入し、アバター制御・外部アプリ連携に利用できる。

## 有効化方法

Action Menu → OSC → Enabled

デバッグ確認: Action Menu → OSC → OSC Debug

## ポート設定

| 方向 | ポート | 説明 |
|---|---|---|
| 受信（外部→VRChat） | 9000 | 外部アプリから VRChat へ送信 |
| 送信（VRChat→外部） | 9001 | VRChat から外部アプリへ送信 |
| OSCQuery（外部⇄VRChat） | TCP（可変） | OSCQuery の HTTP サーバー（localhost = `127.0.0.1` 上）。外部アプリが OSCQuery のデータを取得する |
| mDNS（外部⇄VRChat） | UDP 5353 | マルチキャスト DNS によるサービス検出（OSCQuery の自動検出に使用） |

TCP と mDNS の行の出典: [wiki.vrchat.com/wiki/OSC](https://wiki.vrchat.com/wiki/OSC)。

## メッセージの型

VRChat の OSC エンドポイントは、1つのメッセージに複数の値を載せられるものが多く、各値は次のいずれかの型です。

| 名前 | タイプタグ | 説明 |
|---|---|---|
| int | `i` | 32ビット符号付き整数（ビッグエンディアン） |
| float | `f` | 32ビット IEEE 754 単精度浮動小数点（ビッグエンディアン） |
| boolean | `T` / `F` | 真 / 偽 |
| string | `s` | ASCII テキスト（エンドポイントによっては UTF-8 も受け付ける） |

OSC メッセージのタイプタグ文字列は `,` で始まり、値ごとに1文字が続きます（例: `,sTT`）。

引数が1つだけのときは、等価な別の型の値も受け付けることがあります。

| boolean | int | float |
|---|---|---|
| false | 0 | 0.0 |
| true | 1 | 1.0 |

例: boolean のアバターパラメータ `/avatar/parameters/TurboEncabulator` に `,T`、`,i 1`、`,f 1.0` のいずれを送っても `true` になります。

VRChat は **OSC バンドルの受信・処理には対応**していますが、**自身はバンドルを送信しません**。

## ポートの上書き（コマンドライン引数）

```
--osc=inPort:senderIP:outPort

# 例: デフォルト設定の明示
--osc=9000:127.0.0.1:9001

# 例: LAN 上の別デバイスに送信
--osc=9000:192.168.1.42:9001
```

ログファイルで実際のポート番号を確認可能：
```
2025.01.02 03:04:05 Debug - Advertising Service VRChat-Client-XXXXXX of type OSC on 9000
```

## 推奨ライブラリ

| 言語 | ライブラリ |
|---|---|
| C# | OscCore（all-in-one ブランチ）|
| Python | python-osc |

## 推奨スタンドアロンアプリケーション

| 用途 | アプリ | 備考 |
|---|---|---|
| 送信 | TouchOSC | 無料のWindowsクライアント |
| 受信・モニタリング | Protokol | OSCメッセージの確認用 |

## 関連ファイル

- [Avatar Parameters](./avatar-parameters.md)
- [Avatar Scaling](./avatar-scaling.md)
- [Chatbox](./chatbox.md)
- [Input Controller](./input-controller.md)
- [Trackers](./trackers.md)
- [User Camera](./usercamera.md)
- [Dolly](./dolly.md)
- [Eye Tracking](./eye-tracking.md)
- [Debugging](./debugging.md)
- [OSCQuery](./oscquery.md)
- [Resources](./resources.md)
