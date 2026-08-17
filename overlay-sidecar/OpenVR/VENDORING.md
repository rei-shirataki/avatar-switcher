# ベンダリング元

このディレクトリのファイルは [ValveSoftware/openvr](https://github.com/ValveSoftware/openvr) の公式配布物をそのまま取得したもの。手書き・改変はしていない。

- 取得元コミット: `0924064316de3effbcd1acf1e309182a2deb1c05` (master, 2026-08-17 時点)
- `openvr_api.cs` ← `headers/openvr_api.cs`（公式 C# バインディング、自動生成ファイル）
- `openvr_api.dll` ← `bin/win64/openvr_api.dll`（sha256: `bab8ac6ef64e68a9ca53315b0014d131088584b2efdfa6db511d67ec03cfcb4a`）
- `openvr_api.dll.sig` ← `bin/win64/openvr_api.dll.sig`（Valve 署名）
- `LICENSE.openvr` ← リポジトリルートの `LICENSE`（BSD-3-Clause 相当）

## 更新方法

OpenVR SDK の新バージョンを取り込む場合は、上記3ファイルを同じパスから再取得して置き換える。`openvr_api.cs` は自動生成ファイルのため手動編集しない。
