namespace AvatarSwitcher.OverlaySidecar;

/// <summary>
/// Rust コア (avatar-switcher 本体) がサイドカー起動時に渡す CLI 引数。
/// </summary>
internal sealed class CliOptions
{
    public int WsPort { get; }
    public string Token { get; }

    private CliOptions(int wsPort, string token)
    {
        WsPort = wsPort;
        Token = token;
    }

    public static CliOptions? Parse(string[] args)
    {
        int? wsPort = null;
        string? token = null;
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
        }

        if (wsPort == null || string.IsNullOrEmpty(token))
        {
            return null;
        }

        return new CliOptions(wsPort.Value, token);
    }
}
