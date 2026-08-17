using System;
using System.Threading;
using System.Threading.Tasks;
using AvatarSwitcher.OverlaySidecar.Bridge;
using AvatarSwitcher.OverlaySidecar.SteamVr;

namespace AvatarSwitcher.OverlaySidecar;

/// <summary>
/// SteamVR オーバーレイサイドカーのエントリポイント。
/// Rust コア (avatar-switcher 本体) から --ws-port / --token を渡されて起動する。
///
/// M0 時点では WS 接続の確立と OpenVR 初期化リトライループのみを実装する。
/// CefSharp によるオフスクリーン描画・Direct3D11 テクスチャアップロード・
/// ダッシュボードオーバーレイの生成は M1 で追加する。
/// </summary>
internal static class Program
{
    private static async Task<int> Main(string[] args)
    {
        var options = CliOptions.Parse(args);
        if (options == null)
        {
            Console.Error.WriteLine("usage: overlay-sidecar --ws-port <port> --token <token>");
            return 1;
        }

        using var cts = new CancellationTokenSource();
        Console.CancelKeyPress += (_, e) =>
        {
            e.Cancel = true;
            cts.Cancel();
        };

        await using var bridge = new WsBridgeClient(options.WsPort, options.Token);
        try
        {
            await bridge.ConnectAsync(cts.Token);
        }
        catch (Exception e)
        {
            // Rust コア側の監視ループ (steamvr::sidecar::spawn) がバックオフ付きで
            // 再起動するため、ここでは接続失敗時に非0で終了するだけでよい。
            Console.Error.WriteLine($"[steamvr] WSブリッジへの接続に失敗しました: {e.Message}");
            return 1;
        }

        try
        {
            await new OpenVrWatcher(bridge).RunAsync(cts.Token);
        }
        catch (OperationCanceledException)
        {
            // Ctrl+C 等による正常終了。
        }

        return 0;
    }
}
