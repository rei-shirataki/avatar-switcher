# AvatarSwitcher

VRChat のアバター切り替え・身長（Eye Height）調整をひとつのデスクトップアプリから素早く行うためのツールです。[Tauri](https://tauri.app/) + [Angular](https://angular.dev/) で構築されています。

## できること

- **VRChat アカウントでログイン**（2FA 対応）し、自分のアバター一覧・お気に入りアバター一覧を取得
- **アバター切り替え**：VRChat REST API（アカウント側の装着アバターを更新）と OSC（起動中の VRChat クライアントへ即時反映）を並行送信し、体感の遅延なく切り替え
  - VRChat のクイックメニューなど外部操作でアバターが切り替わった場合も OSC 経由で検知し、サイドバーの表示に即座に反映（双方向同期）
- **Eye Height（目線の高さ）のリアルタイム調整**（OSC で送受信）
  - 即時反映モード（instant）とイージング補間によるスムーズ変化モード（smooth）を切り替え可能
  - VRChat 側で変更された値もエコー処理を挟んで取り込み、アプリの表示と同期
- **アバターのフォルダ分類**によるお気に入り管理
- **アバターの表示名・サムネイルのローカル上書き**（VRChat 側の情報を変更せず、アプリ内表示のみカスタマイズ）
- **SWR 方式のディスクキャッシュ**によるアバター一覧の高速表示・オフライン閲覧
- カスタムタイトルバーを備えたフレームレスウィンドウ UI（ダークテーマ）

## 技術スタック

| 領域 | 技術 |
| --- | --- |
| フロントエンド | Angular 20 (standalone components, signals) |
| デスクトップシェル | Tauri 2 (Rust) |
| VRChat 連携 | VRChat REST API（ログイン・アバター取得）／ OSC（アバター切り替え・Eye Height 送信） |
| ローカル永続化 | `@tauri-apps/plugin-store`（フォルダ・オーバーライド・設定）／ アプリデータディレクトリ上の JSON ファイル（アバター一覧のディスクキャッシュ） |

## セットアップ

### 必要環境

- [Node.js](https://nodejs.org/)（npm）
- [Rust](https://www.rust-lang.org/tools/install)
- Tauri の [各 OS 向け前提パッケージ](https://tauri.app/start/prerequisites/)

### 依存関係のインストール

```bash
npm install
```

### 開発モードで起動

```bash
npm run tauri dev
```

Angular 開発サーバー（`http://localhost:1420`）が Tauri のウィンドウ内に読み込まれます。

### プロダクションビルド

```bash
npm run tauri build
```

## プロジェクト構成

```
src/                      # Angular フロントエンド
  app/core/services/      # VRChat 認証・アバター・Eye Height・Tauri 連携サービス
  app/core/models/        # 型定義
  app/shared/components/  # サイドバー・タイトルバー・アバターカードなど共通 UI
  app/views/              # ログイン・アバター一覧・身長調整・設定の各画面
src-tauri/                # Tauri (Rust) バックエンド
  src/vrchat/             # VRChat REST API クライアント・認証・キャッシュ
  src/osc/                # VRChat への OSC 送信（アバター切り替え・Eye Height）
  src/storage/            # フォルダ・アバターオーバーライドの永続化コマンド
```

## 推奨 IDE 設定

[VS Code](https://code.visualstudio.com/) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer) + [Angular Language Service](https://marketplace.visualstudio.com/items?itemName=Angular.ng-template)
