---
scope: osc
title: OSC DIY
source: https://docs.vrchat.com/docs/osc-diy
status: verified
last_verified: 2026-10-09
---

# OSC DIY

## 概要

VRChatのOSC機能を使ったカスタムプログラムの実装ガイドです。
ネットワーク接続と非同期メッセージングの基礎知識が必要です。

## 利用可能なAPI

| API | 説明 | 詳細 |
|---|---|---|
| Avatar Parameters | アバターパラメータの制御・監視 | [avatar-parameters.md](avatar-parameters.md) |
| Input Controller | プレイヤー入力の制御 | [input-controller.md](input-controller.md) |
| Chatbox | テキスト送信・タイピング表示 | [chatbox.md](chatbox.md) |
| Trackers | 外部トラッカーの統合 | [trackers.md](trackers.md) |
| Eye Tracking | アイトラッキングデータ送信 | [eye-tracking.md](eye-tracking.md) |
| Avatar Scaling | アバタースケール調整 | [avatar-scaling.md](avatar-scaling.md) |

## OSCメッセージの構造

OSCメッセージは**アドレス**と**値**で構成されます。

```
アドレス: /avatar/parameters/VelocityZ
値:       0.75  (float)
```

- アドレスはスラッシュ区切りの階層構造（例: `/fridge/door/butter`）
- 値の型: Avatar Parametersでは `int`、`float`、`bool` をサポート

## 通信モデル

OSCは**単方向通信**です。

| 役割 | 説明 |
|---|---|
| Sender（送信側） | OSCメッセージを送出する |
| Receiver（受信側） | OSCメッセージを受信・処理する |

ハンドシェイク不要。VRChatは同時に Sender にも Receiver にもなります。

```
外部アプリ  →  port 9000  →  VRChat（Receiver）
外部アプリ  ←  port 9001  ←  VRChat（Sender）
```

## 推奨ライブラリ

| 言語 | ライブラリ | 備考 |
|---|---|---|
| C# | [OscCore（VRChat推奨ブランチ）](https://github.com/vrchat/osccore/tree/all-in-one) | Unity 対応。VRChat公式ドキュメントが直接リンクする all-in-one ブランチが最もパフォーマンス・メモリ効率に優れる |
| Python | [python-osc](https://github.com/attwad/python-osc) | 軽量・シンプル |

## Unity での実装上の注意

- OSCメッセージはバックグラウンドスレッドで受信されます
- Unity のオブジェクト操作（Transform, Animator 等）はメインスレッドから行う必要があります
- 受信データをキューに溜めて、`Update()` 内で処理するパターンが一般的です

```csharp
// 疑似コード例
private Queue<(string address, object value)> _messageQueue = new();

void OnOscMessage(string address, object value) {
    // バックグラウンドスレッドで呼ばれる
    lock (_messageQueue) { _messageQueue.Enqueue((address, value)); }
}

void Update() {
    // メインスレッドで処理
    while (_messageQueue.TryDequeue(out var msg)) {
        ApplyToAnimator(msg.address, msg.value);
    }
}
```

## デバッグ

VRChat内蔵のOSCデバッグ画面を使うと受信メッセージをリアルタイムで確認できます。  
詳細: [debugging.md](debugging.md)
