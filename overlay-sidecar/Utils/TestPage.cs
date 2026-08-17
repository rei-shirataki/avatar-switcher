namespace AvatarSwitcher.OverlaySidecar.Utils;

/// <summary>
/// M1（オーバーレイ描画+クリック疎通の確認）用の静的テストページ。
/// overlay-ui (Angular) との接続はM2で行うため、ここではCEF→D3D11テクスチャ→
/// SteamVRオーバーレイのパイプラインと、コントローラーのトリガークリックが
/// CefSharpまで届くことだけを目視確認できれば十分。
/// </summary>
internal static class TestPage
{
    public const string Html = """
        <!doctype html>
        <html>
        <head>
        <meta charset="utf-8" />
        <style>
          html, body {
            margin: 0; padding: 0; width: 100%; height: 100%;
            background: #14161c; color: #f2f2f5;
            font-family: -apple-system, "Segoe UI", sans-serif;
            display: flex; flex-direction: column; align-items: center; justify-content: center;
            gap: 24px;
          }
          h1 { font-size: 40px; margin: 0; }
          #counter { font-size: 28px; color: #9ad1ff; }
          button {
            font-size: 32px; padding: 24px 40px; border-radius: 16px; border: none;
            background: #2b6fd6; color: white; cursor: pointer;
          }
          button:hover { background: #3f82ea; }
          button:active { background: #1f56ad; }
        </style>
        </head>
        <body>
          <h1>AvatarSwitcher Overlay (M1 test)</h1>
          <div id="counter">clicks: 0</div>
          <button id="btn" onclick="onClick()">クリックして確認</button>
          <script>
            let count = 0;
            function onClick() {
              count++;
              document.getElementById('counter').textContent = 'clicks: ' + count;
              console.log('[test-page] button clicked, count=' + count);
            }
          </script>
        </body>
        </html>
        """;
}
