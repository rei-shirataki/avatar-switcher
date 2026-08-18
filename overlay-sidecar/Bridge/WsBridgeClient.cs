using System;
using System.Net.WebSockets;
using System.Text;
using System.Text.Json;
using System.Threading;
using System.Threading.Tasks;

namespace AvatarSwitcher.OverlaySidecar.Bridge;

/// <summary>
/// Rust コアの WebSocket ブリッジ (`src-tauri/src/steamvr/bridge.rs`) に接続するクライアント。
/// 接続直後に `sidecar.hello` を送ってトークン認証を通す（トークンが一致しないと
/// Rust 側が即座に切断する）。
///
/// アバターデータのやり取り (avatars.list / avatars.select、M2で追加予定) は
/// overlay-ui (CefSharp 内で動く Angular) が直接この WS サーバーに接続して行う
/// 設計のため、このクラスはサイドカーのライフサイクル通知専用。
/// </summary>
internal sealed class WsBridgeClient : IAsyncDisposable
{
    private readonly int _wsPort;
    private readonly string _token;
    private readonly ClientWebSocket _socket = new();
    private readonly SemaphoreSlim _sendLock = new(1, 1);

    /// <summary>overlay-ui (CefSharp内のAngular) が自分でRust coreのWSサーバーへ
    /// 接続する際に必要。URLクエリ (?port=..&amp;token=..) として渡す。</summary>
    public int WsPort => _wsPort;
    public string Token => _token;

    public WsBridgeClient(int wsPort, string token)
    {
        _wsPort = wsPort;
        _token = token;
    }

    public async Task ConnectAsync(CancellationToken ct)
    {
        var uri = new Uri($"ws://127.0.0.1:{_wsPort}/");
        await _socket.ConnectAsync(uri, ct);
        var hello = JsonSerializer.Serialize(new
        {
            type = "sidecar.hello",
            pid = Environment.ProcessId,
            token = _token,
        });
        await SendAsync(hello, ct);
    }

    public Task SendSteamVrStatusAsync(bool running, CancellationToken ct)
    {
        var json = JsonSerializer.Serialize(new
        {
            type = "sidecar.steamvr-status",
            running,
        });
        return SendAsync(json, ct);
    }

    private async Task SendAsync(string json, CancellationToken ct)
    {
        var bytes = Encoding.UTF8.GetBytes(json);
        // ClientWebSocket.SendAsync は同時に複数呼ぶと例外になるため直列化する。
        await _sendLock.WaitAsync(ct);
        try
        {
            await _socket.SendAsync(bytes, WebSocketMessageType.Text, endOfMessage: true, ct);
        }
        finally
        {
            _sendLock.Release();
        }
    }

    public async ValueTask DisposeAsync()
    {
        if (_socket.State == WebSocketState.Open)
        {
            try
            {
                await _socket.CloseAsync(WebSocketCloseStatus.NormalClosure, "shutdown", CancellationToken.None);
            }
            catch
            {
                // ベストエフォート。プロセス終了時の切断失敗は無視する。
            }
        }

        _socket.Dispose();
        _sendLock.Dispose();
    }
}
