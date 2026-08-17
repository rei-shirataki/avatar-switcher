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
}
