---
scope: osc
title: OSC Dolly
source: https://wiki.vrchat.com/wiki/OSC
status: verified
last_verified: 2026-10-09
---

# OSC Dolly

## 概要

ユーザーカメラの Dolly（カメラパスに沿ったアニメーション）を OSC で操作します。`/dolly/*` 配下のエンドポイントは、特に記載がなければ**読み書き可能**です。

> **ソースに関する注意**: このページは `docs.vrchat.com` の OSC ページ群には載っておらず、[VRChat Wiki の OSC ページ](https://wiki.vrchat.com/wiki/OSC)にのみ記載があります。関連するカメラ設定は [usercamera.md](usercamera.md) を参照してください。

## エンドポイント

| アドレス | 型 | 方向 | 説明 |
|---|---|---|---|
| `/dolly/Import` | String | write-only | JSON ファイルから Dolly のパスをインポート |
| `/dolly/Export` | String | write-only | 現在の Dolly パスを JSON ファイルへエクスポート。**完了時に OSC メッセージが送出される** |
| `/dolly/ExportLocal` | String | write-only | 上記と同じだが、すべてのポイントをローカル座標としてエクスポート |
| `/dolly/PlayDelayed` | Float | write-only | 指定した秒数だけ待ってからアニメーションを再生 |
| `/dolly/Play` | Bool | 読み書き | アニメーションの再生／停止。遅延中に送ると遅延をスキップ／キャンセル。**アニメーション状態が変わると OSC メッセージが送出される** |

## 関連するユーザーカメラ設定

- `/usercamera/DollyPathsStayVisible`（Bool）: アニメーション中も Dolly のパスを表示するか
- `/usercamera/PhotoRate`（Float、既定 1、0.1〜2）: Dolly 撮影レート
- `/usercamera/Duration`（Float、既定 2、0.1〜60）: Dolly の所要時間

## 注意事項

- `Import` / `Export` / `ExportLocal` はいずれも JSON ファイルを対象にします。引数の String の詳細（パスかファイル名か）は Wiki に記載がなく未確認です
- 状態変化・エクスポート完了が OSC で通知される点は、[overview.md](overview.md) のポート設定（受信ポート 9001）で受け取れます
