---
scope: osc
title: OSC Debugging
source: https://docs.vrchat.com/docs/osc-debugging
status: verified
last_verified: 2026-10-09
---

# OSC Debugging

## 概要

VRChatに内蔵されたOSCデバッグ機能を使うことで、VRChatが受信しているOSCメッセージをゲーム内で視覚的に確認できます。

## 使い方

1. アクションメニューを開く
2. **OSC** セクションを選択
3. **OSC Debug** ボタンをクリック

フローティングスクリーンが表示され、受信中のOSCメッセージ（アドレス・値）をリアルタイムで確認できます。

## 重要な注意事項

OSCデバッグスクリーンを開くと、**メニューからOSCを無効化していた場合でも自動的にOSCが有効化されます。**

## 外部モニタリングツール

ゲーム外でOSCの送受信を確認したい場合は以下のツールも利用できます。

| ツール | 用途 |
|---|---|
| [Protokol](https://hexler.net/protokol) | OSCメッセージの受信・モニタリング |
| [TouchOSC](https://hexler.net/touchosc) | OSCメッセージの送信・テスト |
