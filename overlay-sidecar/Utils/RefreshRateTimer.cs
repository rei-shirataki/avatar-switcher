namespace AvatarSwitcher.OverlaySidecar.Utils;

/// <summary>
/// HMDのリフレッシュレートに合わせてループの周期を一定に保つための簡易タイマー。
/// 固定値ではなくHMDの実リフレッシュレートを都度取得するのは、
/// 90Hz/120Hz等ヘッドセットによって異なるため。
/// </summary>
internal sealed class RefreshRateTimer
{
    private long _lastTick;

    public void TickStart()
    {
        _lastTick = DateTimeOffset.UtcNow.ToUnixTimeMilliseconds();
    }

    private static float TimeUntilNextTick(long lastTick, float minRefreshRate = 30, float maxRefreshRate = 144)
    {
        var refreshRate = Math.Clamp(SteamVr.OvrUtils.GetRefreshRate(), minRefreshRate, maxRefreshRate);
        var elapsed = (float)(DateTimeOffset.UtcNow.ToUnixTimeMilliseconds() - lastTick);
        return 1000f / refreshRate - elapsed;
    }

    public void SleepUntilNextTick()
    {
        var ms = Math.Max(0, TimeUntilNextTick(_lastTick));
        Thread.Sleep((int)Math.Floor(ms));
    }
}
