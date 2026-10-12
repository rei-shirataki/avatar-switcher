using System.Diagnostics;

namespace AvatarSwitcher.OverlaySidecar.Utils;

/// <summary>
/// HMDのリフレッシュレートに合わせてループの周期を一定に保つための簡易タイマー。
/// 固定値ではなくHMDの実リフレッシュレートを都度取得するのは、
/// 90Hz/120Hz等ヘッドセットによって異なるため。
///
/// 経過時間は壁時計（DateTimeOffset.UtcNow）ではなく Stopwatch で測る。壁時計はミリ秒単位で、
/// 時刻同期で巻き戻ると戻った分だけ眠り続けてしまう。また Windows の既定のタイマー分解能
/// （約15.6ms）では Thread.Sleep が粗すぎて 90Hz 以上でも約64Hz しか回らないため、
/// 生存中は分解能を 1ms に上げる。
/// </summary>
internal sealed class RefreshRateTimer : IDisposable
{
    private const uint TimerResolutionMs = 1;

    private readonly Stopwatch _stopwatch = Stopwatch.StartNew();
    private readonly bool _resolutionRaised;
    private TimeSpan _lastTick;

    public RefreshRateTimer()
    {
        try
        {
            _resolutionRaised = WinApi.TimeBeginPeriod(TimerResolutionMs) == 0;
        }
        catch (Exception e) when (e is DllNotFoundException or EntryPointNotFoundException)
        {
            _resolutionRaised = false;
        }
    }

    public void TickStart()
    {
        _lastTick = _stopwatch.Elapsed;
    }

    private double TimeUntilNextTickMs(float minRefreshRate = 30, float maxRefreshRate = 144)
    {
        var refreshRate = Math.Clamp(SteamVr.OvrUtils.GetRefreshRate(), minRefreshRate, maxRefreshRate);
        var elapsed = (_stopwatch.Elapsed - _lastTick).TotalMilliseconds;
        return 1000.0 / refreshRate - elapsed;
    }

    public void SleepUntilNextTick()
    {
        var ms = Math.Max(0, TimeUntilNextTickMs());
        Thread.Sleep((int)Math.Floor(ms));
    }

    public void Dispose()
    {
        if (_resolutionRaised) WinApi.TimeEndPeriod(TimerResolutionMs);
    }
}
