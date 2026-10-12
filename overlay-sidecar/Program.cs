using AvatarSwitcher.OverlaySidecar.Bridge;
using AvatarSwitcher.OverlaySidecar.SteamVr;
using CefSharp;
using CefSharp.OffScreen;

namespace AvatarSwitcher.OverlaySidecar;

/// <summary>
/// SteamVR オーバーレイサイドカーのエントリポイント。
/// Rust コア (avatar-switcher 本体) から --ws-port を、標準入力でトークンを渡されて起動する。
///
/// M1: CEFのオフスクリーン描画 → D3D11テクスチャ → SteamVRオーバーレイの
/// パイプラインと、コントローラーのトリガークリックの疎通を実装。
/// overlay-ui (Angular) との接続、アバター一覧の表示はM2で追加する。
/// </summary>
internal static class Program
{
    /// <summary>
    /// Cef.Initialize と Cef.Shutdown は同じスレッドで呼ぶ必要があるため、Main は同期で書き、
    /// メインスレッド上で両方を呼ぶ（async Main だと await の前後でスレッドが変わり得る）。
    /// </summary>
    private static int Main(string[] args)
    {
        // 標準出力はRust側でファイルへリダイレクトされる。既定の出力エンコーディングは
        // コンソール未接続時にOSごとのコードページに揺れ、ログの日本語が文字化けするため固定する。
        Console.OutputEncoding = System.Text.Encoding.UTF8;

        var options = CliOptions.Parse(args);
        if (options == null)
        {
            Console.Error.WriteLine(
                "usage: overlay-sidecar --ws-port <port> [--ui-port <port>] [--placement-mode hand|space] [--token <token>]\n" +
                "  --token を省略した場合は標準入力の1行目をトークンとして読む");
            return 1;
        }

        var tokenFromStdin = options.Token == null;
        var token = options.Token ?? Console.In.ReadLine()?.Trim();
        if (string.IsNullOrEmpty(token))
        {
            Console.Error.WriteLine("[steamvr] WSブリッジのトークンが渡されていません");
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
        if (tokenFromStdin)
        {
            WatchParentViaStdin(cts);
        }

        var bridge = new WsBridgeClient(options.WsPort, token);
        try
        {
            try
            {
                bridge.ConnectAsync(cts.Token).GetAwaiter().GetResult();
            }
            catch (OperationCanceledException) when (cts.IsCancellationRequested)
            {
                return 0;
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

            var manager = new OverlayManager(bridge, options.UiPort, options.PlacementMode, cts.Token);
            manager.Run();
            cts.Token.WaitHandle.WaitOne();

            // MainLoop が最後の Deactivate（オーバーレイ・ブラウザの破棄）を終えてから CEF を止める。
            if (!manager.WaitForExit(TimeSpan.FromSeconds(5)))
            {
                Console.Error.WriteLine("[steamvr] OverlayManager の停止待ちがタイムアウトしました");
            }
            Cef.Shutdown();
            Console.WriteLine("[steamvr] 正常に終了しました");
            return 0;
        }
        finally
        {
            bridge.DisposeAsync().AsTask().GetAwaiter().GetResult();
        }
    }

    /// <summary>
    /// Rust コアはサイドカーの生存中ずっと標準入力を開いたままにし、終了させたいとき
    /// （アプリ終了時）に閉じる。アプリがクラッシュ・強制終了した場合も OS がパイプを
    /// 閉じるので、EOF を検知したら正常終了する（検知しないと WS の再接続を永久に
    /// 続ける孤児プロセスになり、次に起動したサイドカーとオーバーレイを取り合う）。
    /// </summary>
    private static void WatchParentViaStdin(CancellationTokenSource cts)
    {
        new Thread(() =>
        {
            try
            {
                while (Console.In.Read() != -1)
                {
                }
            }
            catch (Exception e)
            {
                Console.Error.WriteLine($"[steamvr] 標準入力の監視に失敗: {e.Message}");
            }

            Console.WriteLine("[steamvr] 親プロセスとの標準入力が閉じたため終了します");
            cts.Cancel();
        })
        { IsBackground = true, Name = "stdin-watch" }.Start();
    }

    /// <summary>
    /// 過去のサイドカー（終了済みPID）が残したキャッシュディレクトリを削除する。
    /// PID ごとに作る設計のため、放置すると起動のたびにディスクを消費し続ける。
    /// 稼働中のPIDのものは複数サイドカー同居の可能性があるため触らない。
    /// </summary>
    private static void CleanupStaleCaches(string cacheRoot)
    {
        if (!Directory.Exists(cacheRoot)) return;
        foreach (var dir in Directory.EnumerateDirectories(cacheRoot))
        {
            if (!int.TryParse(Path.GetFileName(dir), out var pid) || pid == Environment.ProcessId) continue;
            try
            {
                using var _ = System.Diagnostics.Process.GetProcessById(pid);
                continue; // まだ生きている
            }
            catch (ArgumentException)
            {
                // 該当PIDのプロセスなし = 終了済み
            }

            try
            {
                Directory.Delete(dir, recursive: true);
            }
            catch (Exception e)
            {
                Console.Error.WriteLine($"[steamvr] 古いCEFキャッシュの削除に失敗: {dir} ({e.Message})");
            }
        }
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
        CleanupStaleCaches(cacheRoot);
        settings.RootCachePath = Path.Combine(cacheRoot, Environment.ProcessId.ToString());
        // 既定では exe と同じディレクトリに INFO レベルの debug.log がローテーション無しで
        // 書かれ、JS のコンソール出力もページ URL 付きで残る。エラーのみにし、
        // 終了済みPIDのディレクトリごと掃除されるキャッシュ配下へ書く。
        Directory.CreateDirectory(settings.RootCachePath);
        settings.LogFile = Path.Combine(settings.RootCachePath, "cef.log");
        settings.LogSeverity = LogSeverity.Error;
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
