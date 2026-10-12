---
scope: api
title: VRChat REST API — Files
source: https://vrchat.community/docs/api/
status: community
last_verified: 2026-10-09
---

# VRChat REST API — Files

ベースURL: `https://api.vrchat.cloud/api/1`  
認証: `authCookie` (Cookie: `auth=...`)

アバター・ワールド・画像等のアセットファイルをアップロード・管理するAPIです。
ワールドやアバターのアップロードはこのAPIを通じて行われます。

## エンドポイント一覧

### ファイルオブジェクト管理

| メソッド | パス | operationId | 説明 |
|---|---|---|---|
| GET | `/files` | getFiles | ファイル一覧を取得（`tag`、`n`、`offset` でフィルタ） |
| POST | `/file` | createFile | Fileオブジェクトを作成 |
| GET | `/file/{fileId}` | getFile | Fileオブジェクトの情報を取得 |
| DELETE | `/file/{fileId}` | deleteFile | Fileオブジェクトを削除 |
| POST | `/file/image` | uploadImage | 画像（アイコン・ギャラリー・スタンプ・絵文字等）をアップロード |
| POST | `/gallery` | uploadGalleryImage | ギャラリー画像をアップロード |
| POST | `/icon` | uploadIcon | アイコンをアップロード |
| GET | `/adminassetbundles/{adminAssetBundleId}` | getAdminAssetBundle | AdminAssetBundleオブジェクトを取得 |

### ファイルバージョン管理

| メソッド | パス | operationId | 説明 |
|---|---|---|---|
| POST | `/file/{fileId}` | createFileVersion | 新しいファイルバージョンを作成 |
| GET | `/file/{fileId}/{versionId}` | downloadFileVersion | ファイルバージョンをダウンロード |
| DELETE | `/file/{fileId}/{versionId}` | deleteFileVersion | ファイルバージョンを削除（最新バージョンのみ） |

### ファイルデータアップロード

| メソッド | パス | operationId | 説明 |
|---|---|---|---|
| PUT | `/file/{fileId}/{versionId}/{fileType}/start` | startFileDataUpload | アップロードを開始（URLを取得） |
| PUT | `/file/{fileId}/{versionId}/{fileType}/finish` | finishFileDataUpload | アップロードを完了としてマーク |
| GET | `/file/{fileId}/{versionId}/{fileType}/status` | getFileDataUploadStatus | アップロード状態を確認 |

### アセット解析

| メソッド | パス | operationId | 説明 |
|---|---|---|---|
| GET | `/analysis/{fileId}/{versionId}` | getFileAnalysis | アバターのパフォーマンス解析 |
| GET | `/analysis/{fileId}/{versionId}/security` | getFileAnalysisSecurity | セキュリティ解析 |
| GET | `/analysis/{fileId}/{versionId}/standard` | getFileAnalysisStandard | 標準パフォーマンス解析 |

### コンテンツ同意

| メソッド | パス | operationId | 説明 |
|---|---|---|---|
| GET | `/agreement` | getContentAgreementStatus | コンテンツ同意状態を確認 |
| POST | `/agreement` | submitContentAgreement | コンテンツ同意を提出 |

## ファイルアップロードフロー（ワールド・アバター）

```
1. POST /file
   → FileオブジェクトのIDを取得

2. POST /file/{fileId}
   → 新バージョンを作成

3. PUT /file/{fileId}/{versionId}/file/start
   → S3への署名付きアップロードURLを取得

4. PUT <署名付きURL>  ← S3に直接アップロード（VRChat APIではない）

5. PUT /file/{fileId}/{versionId}/file/finish
   → アップロード完了を通知

6. （アバター・ワールドのみ）
   PUT /file/{fileId}/{versionId}/signature/start
   → signatureファイルのアップロード開始

7. PUT /file/{fileId}/{versionId}/signature/finish
   → signatureファイルのアップロード完了
```

## fileType の値

| 値 | 説明 |
|---|---|
| `file` | メインのアセットファイル（`.vrcw`, `.vrca` 等） |
| `signature` | 署名ファイル（アバター・ワールドで必須） |
| `delta` | 差分ファイル |

## 注意事項

- `uploadImage` は PNG バイナリを multipart/form-data で送信します（`file` と `tag` が必須）
- アニメーション画像の場合は `frames`（フレーム数、2〜64）と `framesOverTime`（アニメーションFPS、1〜64）が必要です
- ファイルバージョンは最新のものしか削除できません
- `getFiles` の `userId` クエリパラメータは非推奨（`deprecated: true`）で、常に500パーミッションエラーになります
- `startFileDataUpload` の `partNumber` クエリパラメータは非推奨です
- `uploadGalleryImage`（`POST /gallery`）と `uploadIcon`（`POST /icon`）は `file` フィールドのみを受け付けます。`uploadImage`（`POST /file/image`）は `file` に加えて `tag` が必須で、アニメーション用の `frames`/`framesOverTime` 等も指定できます
