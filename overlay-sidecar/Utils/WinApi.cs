using System.Runtime.InteropServices;

namespace AvatarSwitcher.OverlaySidecar.Utils;

/// <summary>
/// CEFのペイントバッファをD3D11テクスチャへコピーする際に使うネイティブメモリコピー。
/// 参照実装: OyasumiVR (github.com/Raphiiko/OyasumiVR) src-overlay-sidecar/Utils/WinApi.cs
/// </summary>
internal static class WinApi
{
    [DllImport("kernel32.dll", SetLastError = false)]
    public static extern void RtlCopyMemory(IntPtr destination, IntPtr source, uint length);

    /// <summary>Thread.Sleep 等のタイマー分解能を上げる（既定は約15.6ms）。
    /// Windows 10 2004 以降はこのプロセスにだけ効く。必ず同じ値で timeEndPeriod と対にする。</summary>
    [DllImport("winmm.dll", EntryPoint = "timeBeginPeriod")]
    public static extern uint TimeBeginPeriod(uint periodMs);

    [DllImport("winmm.dll", EntryPoint = "timeEndPeriod")]
    public static extern uint TimeEndPeriod(uint periodMs);
}
