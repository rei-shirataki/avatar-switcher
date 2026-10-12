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
/// アバターデータのやり取り (avatars.list / avatars.select) は overlay-ui
/// (CefSharp 内で動く Angular) が直接この WS サーバーに接続して行う設計のため、
/// このクラスはサイドカーのライフサイクル通知専用。
///
/// サーバーからの push は使わないが、読まないと TCP バッファに溜まり続け、
/// 切断も検知できない。受信ループで読み捨てて切断を検知し、バックオフ付きで
/// 再接続する（ClientWebSocket は再利用できないため都度作り直す）。
/// </summary>
internal sealed class WsBridgeClient : IAsyncDisposable
{
    private static readonly TimeSpan[] ReconnectBackoff =
    {
        TimeSpan.FromSeconds(1), TimeSpan.FromSeconds(3), TimeSpan.FromSeconds(10), TimeSpan.FromSeconds(30),
    };

    private readonly int _wsPort;
    private readonly string _token;
    private readonly SemaphoreSlim _sendLock = new(1, 1);
    private readonly CancellationTokenSource _lifetime = new();
    private ClientWebSocket _socket = new();
    private Task? _loop;
    // 再接続後に Rust 側へ現在の状態を伝え直すため、最後に通知した値を保持する。
    private bool? _lastSteamVrRunning;

    /// <summary>overlay-ui (CefSharp内のAngular) が自分でRust coreのWSサーバーへ
    /// 接続する際に必要。URLクエリ (?port=..&amp;token=..) として渡す。</summary>
    public int WsPort => _wsPort;
    public string Token => _token;

    public WsBridgeClient(int wsPort, string token)
    {
        _wsPort = wsPort;
        _token = token;
    }

    /// <summary>初回接続。失敗は例外で返す（Rust 側の監視ループがサイドカーごと再起動する）。
    /// 成功後は背景で受信ループと自動再接続を開始する。</summary>
    public async Task ConnectAsync(CancellationToken ct)
    {
        await OpenAsync(ct);
        _loop = Task.Run(() => RunAsync(_lifetime.Token));
    }

    private async Task OpenAsync(CancellationToken ct)
    {
        var socket = new ClientWebSocket();
        await socket.ConnectAsync(new Uri($"ws://127.0.0.1:{_wsPort}/"), ct);

        await _sendLock.WaitAsync(ct);
        try
        {
            var old = _socket;
            _socket = socket;
            old.Dispose();
        }
        finally
        {
            _sendLock.Release();
        }

        await SendAsync(JsonSerializer.Serialize(new
        {
            type = "sidecar.hello",
            pid = Environment.ProcessId,
            token = _token,
        }), ct);

        // 再接続時は Rust 側の状態が失われているため、最後の状態を再送する。
        if (_lastSteamVrRunning is { } running)
        {
            await SendAsync(SteamVrStatusJson(running), ct);
        }
    }

    private async Task RunAsync(CancellationToken ct)
    {
        var attempt = 0;
        while (!ct.IsCancellationRequested)
        {
            await DrainAsync(ct);
            if (ct.IsCancellationRequested) return;

            Console.Error.WriteLine("[steamvr] WSブリッジから切断されました。再接続します");
            while (!ct.IsCancellationRequested)
            {
                var delay = ReconnectBackoff[Math.Min(attempt, ReconnectBackoff.Length - 1)];
                attempt++;
                try
                {
                    await Task.Delay(delay, ct);
                    await OpenAsync(ct);
                    attempt = 0;
                    Console.WriteLine("[steamvr] WSブリッジへ再接続しました");
                    break;
                }
                catch (OperationCanceledException) when (ct.IsCancellationRequested)
                {
                    return;
                }
                catch (Exception e)
                {
                    Console.Error.WriteLine($"[steamvr] WSブリッジへの再接続に失敗: {e.Message}");
                }
            }
        }
    }

    /// <summary>接続が閉じるかエラーになるまで受信を読み捨てる。</summary>
    private async Task DrainAsync(CancellationToken ct)
    {
        var socket = _socket;
        var buffer = new byte[4096];
        try
        {
            while (socket.State == WebSocketState.Open && !ct.IsCancellationRequested)
            {
                var result = await socket.ReceiveAsync(buffer, ct);
                if (result.MessageType == WebSocketMessageType.Close) return;
            }
        }
        catch (Exception) when (!ct.IsCancellationRequested)
        {
            // 切断・プロトコルエラー。呼び出し側が再接続する。
        }
        catch (OperationCanceledException)
        {
        }
    }

    /// <summary>SteamVR 稼働状態を通知する。未接続なら最新値を保持し、再接続後に送る。
    /// 通知失敗でメインループを止めないよう例外は投げない。</summary>
    public async Task SendSteamVrStatusAsync(bool running, CancellationToken ct)
    {
        _lastSteamVrRunning = running;
        try
        {
            await SendAsync(SteamVrStatusJson(running), ct);
        }
        catch (Exception e) when (e is not OperationCanceledException)
        {
            Console.Error.WriteLine($"[steamvr] SteamVR状態の通知に失敗（再接続後に再送します）: {e.Message}");
        }
    }

    private static string SteamVrStatusJson(bool running) =>
        JsonSerializer.Serialize(new { type = "sidecar.steamvr-status", running });

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
        _lifetime.Cancel();

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

        if (_loop != null)
        {
            try
            {
                await _loop;
            }
            catch
            {
                // 終了時の例外は無視する。
            }
        }

        _socket.Dispose();
        _sendLock.Dispose();
        _lifetime.Dispose();
    }
}
