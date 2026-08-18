using AvatarSwitcher.OverlaySidecar.Bridge;
using AvatarSwitcher.OverlaySidecar.SteamVr;
using CefSharp;
using CefSharp.OffScreen;

namespace AvatarSwitcher.OverlaySidecar;

/// <summary>
/// SteamVR オーバーレイサイドカーのエントリポイント。
/// Rust コア (avatar-switcher 本体) から --ws-port / --token を渡されて起動する。
///
/// M1: CEFのオフスクリーン描画 → D3D11テクスチャ → SteamVRオーバーレイの
/// パイプラインと、コントローラーのトリガークリックの疎通を実装。
/// overlay-ui (Angular) との接続、アバター一覧の表示はM2で追加する。
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

        AppDomain.CurrentDomain.UnhandledException += (_, e) =>
            Console.Error.WriteLine($"[steamvr] unhandled exception: {e.ExceptionObject}");

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

        if (!InitCef())
        {
            return 1;
        }

        try
        {
            new OverlayManager(bridge, cts.Token).Run();
            await Task.Delay(Timeout.Infinite, cts.Token);
        }
        catch (OperationCanceledException)
        {
            // Ctrl+C 等による正常終了。
        }
        finally
        {
            Cef.Shutdown();
        }

        return 0;
    }

    private static bool InitCef()
    {
        var settings = new CefSettings();

        // CEF 122+ の Chrome bootstrap は、既定のRootCachePath
        // (%LOCALAPPDATA%\CEF\User Data) を複数プロセスで共有しようとすると
        // Chromiumのプロセスシングルトン機構に引っかかり、2つ目以降の起動が
        // 既存プロセスへコマンドラインを転送して自身は終了してしまう
        // （結果、Rust側の監視ループが再起動を延々繰り返す）。
        // プロセスごとに一意なRootCachePathを与えることでこれを回避する。
        // 参照: OyasumiVR Program.cs のコメント
        // (github.com/Raphiiko/OyasumiVR, issue #168/#166/#165)
        var cacheRoot = Path.Combine(
            Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData),
            "com.rei-shirataki.avatar-switcher", "overlay-cef-cache");
        settings.RootCachePath = Path.Combine(cacheRoot, Environment.ProcessId.ToString());
        settings.CachePath = "";
        settings.PersistSessionCookies = false;
        settings.CefCommandLineArgs.Add("disable-features", "MetricsService,PersistentHistograms");
        settings.CefCommandLineArgs.Add("disable-crash-reporter", "true");
        settings.CefCommandLineArgs.Add("disable-spell-checking", "true");

        if (!Cef.Initialize(settings))
        {
            Console.Error.WriteLine("[steamvr] CEFの初期化に失敗しました");
            return false;
        }

        return true;
    }
}
