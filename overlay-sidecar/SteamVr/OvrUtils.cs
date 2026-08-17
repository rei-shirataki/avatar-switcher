using Valve.VR;

namespace AvatarSwitcher.OverlaySidecar.SteamVr;

/// <summary>
/// OpenVR呼び出しの小さな共通ヘルパー。
/// 参照実装: OyasumiVR src-overlay-sidecar/Utils/OVRUtils.cs
/// </summary>
internal static class OvrUtils
{
    private static float _refreshRate = 90;
    private static long _refreshRateLastSetAt;

    /// <summary>
    /// 既存のオーバーレイキーがあればそれを、無ければ新規作成して返す。
    /// アプリ再起動をまたいでも同じキーなら同じオーバーレイを握り直せる。
    /// </summary>
    public static EVROverlayError GetOrCreateOverlay(string key, string name, ref ulong handle)
    {
        var overlay = OpenVR.Overlay;
        if (overlay == null) return EVROverlayError.RequestFailed;
        var error = overlay.FindOverlay(key, ref handle);
        if (error == EVROverlayError.None) return EVROverlayError.None;
        return overlay.CreateOverlay(key, name, ref handle);
    }

    public static TrackedDevicePose_t GetHeadPose(TrackedDevicePose_t[] poseBuffer)
    {
        OpenVR.System.GetDeviceToAbsoluteTrackingPose(ETrackingUniverseOrigin.TrackingUniverseStanding, 0, poseBuffer);
        return poseBuffer[0];
    }

    /// <summary>
    /// HMDの実リフレッシュレートを取得する。5秒キャッシュしてSteamVR呼び出しを抑える。
    /// </summary>
    public static float GetRefreshRate()
    {
        var now = DateTimeOffset.UtcNow.ToUnixTimeMilliseconds();
        if (now - _refreshRateLastSetAt <= 5000) return _refreshRate;
        var system = OpenVR.System;
        if (system == null) return _refreshRate;
        var error = ETrackedPropertyError.TrackedProp_Success;
        var frequency = system.GetFloatTrackedDeviceProperty(0, ETrackedDeviceProperty.Prop_DisplayFrequency_Float, ref error);
        if (error != ETrackedPropertyError.TrackedProp_Success) return _refreshRate;
        _refreshRate = frequency;
        _refreshRateLastSetAt = now;
        return _refreshRate;
    }
}
