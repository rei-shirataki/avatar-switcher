using System.Numerics;
using AvatarSwitcher.OverlaySidecar.Rendering;
using AvatarSwitcher.OverlaySidecar.Utils;
using CefSharp;
using SharpDX.Direct3D11;
using Valve.VR;

namespace AvatarSwitcher.OverlaySidecar.SteamVr;

/// <summary>
/// アバター切替パネル本体のオーバーレイ。
///
/// M1時点では OyasumiVR の DashboardOverlay がハンド追従時に使っている
/// フォールバック分岐（頭部前方に固定配置）だけを実装し、手/頭姿勢を毎フレーム
/// 追従させる処理はまだ入れていない（次段階の作業）。開閉トグルもM2以降で
/// SteamVR Inputの `OpenOverlay` アクションと組み合わせて追加する予定。
///
/// 参照実装: OyasumiVR src-overlay-sidecar/Overlays/BaseWebOverlay.cs, DashboardOverlay.cs
/// </summary>
internal sealed class AvatarPanelOverlay : IDisposable
{
    private const string OverlayKey = "com.rei-shirataki.avatar-switcher:AvatarPanel";
    private const string OverlayName = "AvatarSwitcher Panel";
    private const uint Resolution = 1024;
    private const float WidthMeters = 0.45f;
    /// <summary>頭部前方のオフセット（m）。DashboardOverlayのフォールバック配置に合わせた値。</summary>
    private const float ForwardOffsetMeters = 0.55f;

    private readonly D3D11Context _d3D;
    private ulong _overlayHandle;
    private OffscreenBrowser? _browser;
    private Texture2D? _texture;
    private EVROverlayError _lastTextureError = EVROverlayError.None;
    private bool _disposed;

    public ulong OverlayHandle => _overlayHandle;
    public OffscreenBrowser? Browser => _browser;

    public AvatarPanelOverlay(D3D11Context d3D)
    {
        _d3D = d3D;
    }

    public async Task OpenAsync()
    {
        var err = OvrUtils.GetOrCreateOverlay(OverlayKey, OverlayName, ref _overlayHandle);
        if (err != EVROverlayError.None)
        {
            Console.Error.WriteLine($"[steamvr] オーバーレイの作成に失敗しました: {err}");
            return;
        }

        OpenVR.Overlay.SetOverlayWidthInMeters(_overlayHandle, WidthMeters);

        _texture = await _d3D.CreateCpuWritableTextureAsync(Resolution);
        _browser = new OffscreenBrowser("about:blank", Resolution, Resolution);
        LogBrowserEvents(_browser);
        _browser.SetTextureTarget(_texture);
        _browser.LoadHtml(TestPage.Html);

        PlaceInFrontOfHead();
        OpenVR.Overlay.ShowOverlay(_overlayHandle);
    }

    /// <summary>
    /// 頭部の前方 <see cref="ForwardOffsetMeters"/> m に固定配置する。
    /// DashboardOverlay.GetTargetTransform のハンドトラッキング非対応時フォールバックと同じ式。
    /// </summary>
    private void PlaceInFrontOfHead()
    {
        var poseBuffer = new TrackedDevicePose_t[OpenVR.k_unMaxTrackedDeviceCount];
        var headPose = OvrUtils.GetHeadPose(poseBuffer);
        if (headPose.eTrackingResult != ETrackingResult.Running_OK) return;

        var headMatrix = headPose.mDeviceToAbsoluteTracking.ToMatrix4X4();
        var offset = Matrix4x4.CreateTranslation(0, 0, -ForwardOffsetMeters);
        var transform = (offset * headMatrix).ToHmdMatrix34T();
        OpenVR.Overlay.SetOverlayTransformAbsolute(_overlayHandle, ETrackingUniverseOrigin.TrackingUniverseStanding,
            ref transform);
    }

    /// <summary>
    /// 直近のCEFペイントをオーバーレイテクスチャへ反映する。フレーム更新ループから毎ティック呼ぶ。
    /// 1秒以上ペイントが無い場合は無駄なテクスチャ送信を避けてスキップする。
    /// </summary>
    public void UpdateFrame()
    {
        if (_disposed || _browser == null || _texture == null) return;
        if (DateTimeOffset.UtcNow.ToUnixTimeMilliseconds() - _browser.LastPaintAt >= 1000) return;

        _browser.WriteToTexture();

        var texture = new Texture_t { handle = _texture.NativePointer };
        var err = OpenVR.Overlay.SetOverlayTexture(_overlayHandle, ref texture);
        if (err != EVROverlayError.None && err != _lastTextureError)
        {
            Console.Error.WriteLine($"[steamvr] SetOverlayTexture 失敗: {err}");
        }

        _lastTextureError = err;
    }

    private static void LogBrowserEvents(OffscreenBrowser browser)
    {
        browser.ConsoleMessage += (_, e) =>
            Console.WriteLine($"[overlay-ui console] {e.Level} {e.Message} ({e.Source}:{e.Line})");
        browser.LoadError += (_, e) =>
            Console.Error.WriteLine($"[overlay-ui] load error: {e.FailedUrl} {e.ErrorCode} {e.ErrorText}");
    }

    public void Dispose()
    {
        if (_disposed) return;
        _disposed = true;
        if (_overlayHandle != 0) OpenVR.Overlay.DestroyOverlay(_overlayHandle);
        _browser?.SetTextureTarget(null);
        _browser?.Dispose();
        _browser = null;
        _texture?.Dispose();
        _texture = null;
    }
}
