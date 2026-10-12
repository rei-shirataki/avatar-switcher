namespace AvatarSwitcher.OverlaySidecar.Utils;

/// <summary>
/// 同じ箇所から高頻度に出るエラーログを、一定間隔に1回まで間引いて標準エラーへ出す。
/// サイドカーのログは起動時にしかローテーションされないため、毎ティックの出力が
/// 続くと1セッションで数十MBに達する。間引いた件数は次の出力に添える。
/// </summary>
internal sealed class ThrottledLog
{
    private readonly TimeSpan _interval;
    private readonly object _lock = new();
    private DateTime _last = DateTime.MinValue;
    private int _suppressed;

    public ThrottledLog(TimeSpan interval)
    {
        _interval = interval;
    }

    public void Log(string message)
    {
        int suppressed;
        lock (_lock)
        {
            var now = DateTime.UtcNow;
            if (now - _last < _interval)
            {
                _suppressed++;
                return;
            }
            _last = now;
            suppressed = _suppressed;
            _suppressed = 0;
        }
        Console.Error.WriteLine(suppressed > 0 ? $"{message}（ほか {suppressed} 件を省略）" : message);
    }
}
