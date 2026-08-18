using AvatarSwitcher.OverlaySidecar.SteamVr;

namespace AvatarSwitcher.OverlaySidecar;

/// <summary>
/// Rust コア (avatar-switcher 本体) がサイドカー起動時に渡す CLI 引数。
/// </summary>
internal sealed class CliOptions
{
    public int WsPort { get; }
    public string Token { get; }
    /// <summary>overlay-ui静的ファイルサーバーのポート。Rust側が起動できなかった
    /// 場合（overlay-ui未ビルド等）は渡されない。</summary>
    public int? UiPort { get; }
    /// <summary>パネル配置方式（#28）。未指定・不正値はHandにフォールバックする。</summary>
    public PlacementMode PlacementMode { get; }

    private CliOptions(int wsPort, string token, int? uiPort, PlacementMode placementMode)
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
            if (args[i] == "--ws-port" && i + 1 < args.Length && int.TryParse(args[i + 1], out var p))
            {
                wsPort = p;
                i++;
            }
            else if (args[i] == "--token" && i + 1 < args.Length)
            {
                token = args[i + 1];
                i++;
            }
            else if (args[i] == "--ui-port" && i + 1 < args.Length && int.TryParse(args[i + 1], out var up))
            {
                uiPort = up;
                i++;
            }
            else if (args[i] == "--placement-mode" && i + 1 < args.Length)
            {
                placementMode = args[i + 1] switch
                {
                    "space" => PlacementMode.Space,
                    _ => PlacementMode.Hand,
                };
                i++;
            }
        }

        if (wsPort == null || string.IsNullOrEmpty(token))
        {
            return null;
        }

        return new CliOptions(wsPort.Value, token, uiPort, placementMode);
    }
}
