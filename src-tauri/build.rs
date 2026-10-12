fn main() {
    // `bundle.resources` は存在しないパスを許さず、cargo check / tauri dev でも
    // build.rs が失敗してしまう。overlay-sidecar の publish 成果物は
    // `beforeBuildCommand`（リリースビルド時のみ）で生成されるため、未生成でも
    // ビルドできるよう空ディレクトリを先に用意しておく。
    // 空のままバンドルされた場合、sidecar.rs は実行ファイル未検出として扱う。
    let _ = std::fs::create_dir_all("../overlay-sidecar/publish");
    tauri_build::build()
}
