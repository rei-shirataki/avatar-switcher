//! overlay-ui (Angular, `dist/overlay-ui/browser`) を配信するだけの最小限の
//! ローカル静的ファイルサーバー。
//!
//! CefSharpで `file://` から直接 index.html を読み込む方式は、Angularの
//! esbuildビルドが出力する `<script type="module">` がES moduleの仕様上
//! 常にCORSチェックを受け、`file://` の origin (`null`) では他ファイルの
//! 読み込みが必ずブロックされるため実機で動作しなかった
//! （`Access to script at 'file:///...' from origin 'null' has been
//! blocked by CORS policy` を実機ログで確認）。OyasumiVRも同じ理由から
//! 本番ビルドでも `file://` を使わずローカルHTTPサーバー
//! (`IpcManager.Instance.StaticBaseUrl`) 経由で配信している。
//!
//! `osc::oscquery::start_http_server` と同じく、外部クレートを追加せず
//! 素朴なTCP+HTTP/1.1レスポンスで実装する（ローカル・GETのみ・
//! 単一ディレクトリ配下のファイル配信のみという極小要件のため）。

use std::path::{Component, Path, PathBuf};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

const MAX_REQUEST_HEADER_BYTES: usize = 8 * 1024;

/// `127.0.0.1` の空きポートに bind し、`dir` 配下のファイルを配信する。
/// 戻り値は bind したポート番号（overlay-sidecar へ `--ui-port` で渡す）。
pub(crate) async fn start(dir: PathBuf) -> anyhow::Result<u16> {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let port = listener.local_addr()?.port();
    tokio::spawn(async move {
        loop {
            match listener.accept().await {
                Ok((stream, _)) => {
                    let dir = dir.clone();
                    tokio::spawn(async move {
                        let _ = tokio::time::timeout(
                            std::time::Duration::from_secs(5),
                            handle_request(stream, &dir),
                        )
                        .await;
                    });
                }
                Err(e) => log::warn!("[steamvr] 静的ファイルサーバー accept エラー: {}", e),
            }
        }
    });
    Ok(port)
}

async fn handle_request(mut stream: tokio::net::TcpStream, dir: &Path) {
    let mut buf: Vec<u8> = Vec::with_capacity(1024);
    let mut tmp = [0u8; 1024];
    loop {
        match stream.read(&mut tmp).await {
            Ok(0) => return,
            Ok(n) => {
                buf.extend_from_slice(&tmp[..n]);
                if buf.windows(4).any(|w| w == b"\r\n\r\n") {
                    break;
                }
                if buf.len() > MAX_REQUEST_HEADER_BYTES {
                    return;
                }
            }
            Err(_) => return,
        }
    }

    let request = std::str::from_utf8(&buf).unwrap_or("");
    let request_line = request.lines().next().unwrap_or("");
    let uri = request_line.split_whitespace().nth(1).unwrap_or("/");
    let path = uri.split('?').next().unwrap_or("/");

    let Some(file_path) = resolve_path(dir, path) else {
        let _ = write_response(&mut stream, 403, "text/plain", b"forbidden").await;
        return;
    };

    match tokio::fs::read(&file_path).await {
        Ok(body) => {
            let content_type = content_type_for(&file_path);
            let _ = write_response(&mut stream, 200, content_type, &body).await;
        }
        Err(_) => {
            let _ = write_response(&mut stream, 404, "text/plain", b"not found").await;
        }
    }
}

/// URIパスをディレクトリトラバーサル無しで `dir` 配下の実ファイルパスへ解決する。
/// `/` は `index.html` として扱う（Angular側もルーティング無しのSPAなので
/// サブパス配信は考慮不要）。
fn resolve_path(dir: &Path, uri_path: &str) -> Option<PathBuf> {
    let trimmed = uri_path.trim_start_matches('/');
    let rel = if trimmed.is_empty() { "index.html" } else { trimmed };
    let rel_path = Path::new(rel);
    if rel_path
        .components()
        .any(|c| matches!(c, Component::ParentDir | Component::RootDir | Component::Prefix(_)))
    {
        return None;
    }
    Some(dir.join(rel_path))
}

fn content_type_for(path: &Path) -> &'static str {
    match path.extension().and_then(|e| e.to_str()) {
        // ES module の <script type="module"> はMIMEタイプが厳格にチェックされ、
        // application/javascript 以外だと読み込み自体が拒否される。
        Some("js") => "application/javascript; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("html") => "text/html; charset=utf-8",
        Some("ico") => "image/x-icon",
        Some("svg") => "image/svg+xml",
        Some("json") => "application/json; charset=utf-8",
        Some("wasm") => "application/wasm",
        _ => "application/octet-stream",
    }
}

async fn write_response(
    stream: &mut tokio::net::TcpStream,
    status: u16,
    content_type: &str,
    body: &[u8],
) -> std::io::Result<()> {
    let status_text = match status {
        200 => "OK",
        403 => "Forbidden",
        404 => "Not Found",
        _ => "Error",
    };
    let header = format!(
        "HTTP/1.1 {status} {status_text}\r\nContent-Type: {content_type}\r\nContent-Length: {len}\r\nConnection: close\r\n\r\n",
        status = status,
        status_text = status_text,
        content_type = content_type,
        len = body.len(),
    );
    stream.write_all(header.as_bytes()).await?;
    stream.write_all(body).await?;
    stream.flush().await
}
