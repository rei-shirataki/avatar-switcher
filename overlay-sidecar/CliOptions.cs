using AvatarSwitcher.OverlaySidecar.SteamVr;

namespace AvatarSwitcher.OverlaySidecar;

/// <summary>
/// Rust コア (avatar-switcher 本体) がサイドカー起動時に渡す CLI 引数。
/// </summary>
internal sealed class CliOptions
{
    public int WsPort { get; }
    /// <summary>WS ブリッジの認証トークン。通常は Rust コアが標準入力の1行目で渡す
    /// （コマンドラインはプロセス一覧から読めるため）。手動起動の検証用に
    /// `--token` も受け付ける。</summary>
    public string? Token { get; }
    /// <summary>overlay-ui静的ファイルサーバーのポート。Rust側が起動できなかった
    /// 場合（overlay-ui未ビルド等）は渡されない。</summary>
    public int? UiPort { get; }
    /// <summary>パネル配置方式（#28）。未指定・不正値はHandにフォールバックする。</summary>
    public PlacementMode PlacementMode { get; }

    private CliOptions(int wsPort, string? token, int? uiPort, PlacementMode placementMode)
    {
        WsPort = wsPort;
        Token = token;
        UiPort = uiPort;
        PlacementMode = placementMode;
    }

    public static CliOptions? Parse(string[] args)
    {
        int? wsPort = null;
        string? token = null;
        int? uiPort = null;
        var placementMode = PlacementMode.Hand;
        for (var i = 0; i < args.Length; i++)
        {
            var name = args[i];
            // 値が欠けていると次のフラグを値として取り込んでしまうため、"--" 始まりは値として扱わない。
            var value = i + 1 < args.Length && !args[i + 1].StartsWith("--", StringComparison.Ordinal)
                ? args[i + 1]
                : null;
            switch (name)
            {
                case "--ws-port" when TryParsePort(value, out var p):
                    wsPort = p;
                    i++;
                    break;
                case "--token" when !string.IsNullOrEmpty(value):
                    token = value;
                    i++;
                    break;
                case "--ui-port" when TryParsePort(value, out var up):
                    uiPort = up;
                    i++;
                    break;
                case "--placement-mode" when value != null:
                    placementMode = value switch
                    {
                        "space" => PlacementMode.Space,
                        _ => PlacementMode.Hand,
                    };
                    i++;
                    break;
                default:
                    // 不正な値や未知の引数を黙って無視すると、起動失敗の原因が分からなくなる。
                    Console.Error.WriteLine($"[steamvr] 不正なコマンドライン引数を無視します: {name}");
                    break;
            }
        }

        if (wsPort == null)
        {
            return null;
        }

        return new CliOptions(wsPort.Value, token, uiPort, placementMode);
    }

    /// <summary>1〜65535 のポート番号として読めるか。範囲外を通すと URI 生成で例外になり、
    /// 起動失敗→再起動のループになる。</summary>
    internal static bool TryParsePort(string? value, out int port) =>
        int.TryParse(value, out port) && port is >= 1 and <= 65535;
}
