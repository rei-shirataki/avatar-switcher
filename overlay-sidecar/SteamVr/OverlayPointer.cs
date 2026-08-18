using System.Numerics;
using AvatarSwitcher.OverlaySidecar.Rendering;
using AvatarSwitcher.OverlaySidecar.Utils;
using CefSharp;
using Valve.VR;
using CefEventFlags = CefSharp.CefEventFlags;
using MouseButtonType = CefSharp.MouseButtonType;

namespace AvatarSwitcher.OverlaySidecar.SteamVr;

/// <summary>
/// コントローラーからのレイキャストで <see cref="AvatarPanelOverlay"/> との交差点を求め、
/// 小さな円形オーバーレイをポインタとして表示しつつ、交点のUV座標をCefSharpの
/// マウス座標へ変換して注入する。
///
/// SteamVRダッシュボードタブ（CreateDashboardOverlay）と違い、通常のオーバーレイ
/// （CreateOverlay）はSteamVRがマウスイベントを合成してくれないため、
/// レイキャストとポインタ描画・入力注入をすべて自前で行う必要がある。
///
/// 参照実装: OyasumiVR src-overlay-sidecar/Overlays/Helpers/OverlayPointer.cs
/// </summary>
internal sealed class OverlayPointer : IDisposable
{
    private const float PointerWidthMeters = 0.02f;
    private const uint PointerSortOrder = 150;

    private readonly PointerState _left = new();
    private readonly PointerState _right = new();
    private AvatarPanelOverlay? _target;
    private bool _disposed;

    public OverlayPointer()
    {
        SetUpPointerOverlay(_left, "com.rei-shirataki.avatar-switcher:PointerLeft", "AvatarSwitcher Left Pointer");
        SetUpPointerOverlay(_right, "com.rei-shirataki.avatar-switcher:PointerRight", "AvatarSwitcher Right Pointer");
    }

    public void SetTarget(AvatarPanelOverlay? overlay)
    {
        _target = overlay;
    }

    private static void SetUpPointerOverlay(PointerState state, string key, string name)
    {
        OvrUtils.GetOrCreateOverlay(key, name, ref state.OverlayHandle);
        OpenVR.Overlay.SetOverlayWidthInMeters(state.OverlayHandle, PointerWidthMeters);
        OpenVR.Overlay.SetOverlaySortOrder(state.OverlayHandle, PointerSortOrder);

        var (pixels, width, height) = BuildPointerDot();
        var handle = System.Runtime.InteropServices.GCHandle.Alloc(pixels, System.Runtime.InteropServices.GCHandleType.Pinned);
        try
        {
            OpenVR.Overlay.SetOverlayRaw(state.OverlayHandle, handle.AddrOfPinnedObject(), width, height, 4);
        }
        finally
        {
            handle.Free();
        }
    }

    /// <summary>
    /// 16x16のBGRA円形ドット画像を生成する（画像アセット無しでポインタを表示するため）。
    /// </summary>
    private static (byte[] pixels, uint width, uint height) BuildPointerDot()
    {
        const int size = 16;
        var pixels = new byte[size * size * 4];
        var center = (size - 1) / 2f;
        var radius = size / 2f - 1f;
        for (var y = 0; y < size; y++)
        {
            for (var x = 0; x < size; x++)
            {
                var dx = x - center;
                var dy = y - center;
                var inside = dx * dx + dy * dy <= radius * radius;
                var i = (y * size + x) * 4;
                if (inside)
                {
                    // BGRA、不透明の白いドット。
                    pixels[i + 0] = 255;
                    pixels[i + 1] = 255;
                    pixels[i + 2] = 255;
                    pixels[i + 3] = 255;
                }
            }
        }

        return (pixels, size, size);
    }

    /// <summary>
    /// 両コントローラーからパネルへレイキャストし、交点にポインタを表示、
    /// UV座標からCEFへマウス移動イベントを注入する。メインループから毎ティック呼ぶ。
    /// </summary>
    public void UpdateRaycast()
    {
        var poseBuffer = new TrackedDevicePose_t[OpenVR.k_unMaxTrackedDeviceCount];
        UpdateForController(ETrackedControllerRole.LeftHand, _left, poseBuffer);
        UpdateForController(ETrackedControllerRole.RightHand, _right, poseBuffer);
    }

    private void UpdateForController(ETrackedControllerRole role, PointerState pointer, TrackedDevicePose_t[] poseBuffer)
    {
        if (_target == null || _target.OverlayHandle == 0)
        {
            Hide(pointer);
            return;
        }

        var index = OpenVR.System.GetTrackedDeviceIndexForControllerRole(role);
        if (index is < 1 or >= OpenVR.k_unMaxTrackedDeviceCount)
        {
            Hide(pointer);
            return;
        }

        OpenVR.System.GetDeviceToAbsoluteTrackingPose(ETrackingUniverseOrigin.TrackingUniverseStanding, 0, poseBuffer);
        var controllerPose = poseBuffer[index];
        if (!controllerPose.bPoseIsValid || !controllerPose.bDeviceIsConnected)
        {
            Hide(pointer);
            return;
        }

        // コントローラー実機の持ち方とレイの狙う先にずれがあるため一定角度だけ傾ける。
        // この角度はOyasumiVRのポインタ実装からそのまま踏襲した値で、変更すると
        // 狙点がずれるため調整しない。
        var controllerTransform = Matrix4x4.CreateRotationX(345f) *
                                   controllerPose.mDeviceToAbsoluteTracking.ToMatrix4X4();

        var intersectionParams = new VROverlayIntersectionParams_t
        {
            eOrigin = ETrackingUniverseOrigin.TrackingUniverseStanding,
            vSource = controllerTransform.Translation.ToHmdVector3T(),
            vDirection = controllerTransform.GetDirectionNormal().ToHmdVector3T(),
        };
        var intersection = new VROverlayIntersectionResults_t();
        if (!OpenVR.Overlay.ComputeOverlayIntersection(_target.OverlayHandle, ref intersectionParams, ref intersection))
        {
            Hide(pointer);
            return;
        }

        var uv = intersection.vUVs.ToVector2();
        if (uv.X is < 0 or > 1 || uv.Y is < 0 or > 1)
        {
            Hide(pointer);
            return;
        }

        var headPose = OvrUtils.GetHeadPose(poseBuffer);
        var headRotation = Matrix4x4.CreateFromQuaternion(Quaternion.CreateFromRotationMatrix(
            headPose.mDeviceToAbsoluteTracking.ToMatrix4X4()));
        var position = intersection.vPoint.ToVector3();
        var transform = (headRotation * Matrix4x4.CreateTranslation(position)).ToHmdMatrix34T();
        OpenVR.Overlay.SetOverlayTransformAbsolute(pointer.OverlayHandle, ETrackingUniverseOrigin.TrackingUniverseStanding,
            ref transform);
        OpenVR.Overlay.ShowOverlay(pointer.OverlayHandle);

        pointer.LastUv = uv;
        SendMouseMove(pointer);
    }

    private void Hide(PointerState pointer)
    {
        OpenVR.Overlay.HideOverlay(pointer.OverlayHandle);
        // 交点を失った瞬間のCEF側マウス状態をクリアする(離脱通知)。
        if (pointer.LastUv.HasValue)
        {
            SendMouseMove(pointer, mouseLeave: true);
        }

        pointer.LastUv = null;
        pointer.Pressed = false;
    }

    private void SendMouseMove(PointerState pointer, bool mouseLeave = false)
    {
        var browser = _target?.Browser;
        if (browser == null || pointer.LastUv == null) return;
        var (x, y) = ToBrowserPixels(pointer.LastUv.Value, browser);
        browser.GetBrowser().GetHost().SendMouseMoveEvent(x, y, mouseLeave,
            pointer.Pressed ? CefEventFlags.LeftMouseButton : CefEventFlags.None);
    }

    /// <summary>UV原点(左下)とCEFのピクセル原点(左上)でY軸が逆になるため反転する。</summary>
    private static (int x, int y) ToBrowserPixels(Vector2 uv, OffscreenBrowser browser)
    {
        var x = (int)(uv.X * browser.Size.Width);
        var y = (int)((1.0f - uv.Y) * browser.Size.Height);
        return (x, y);
    }

    /// <summary>SteamVR Inputの `OverlayInteract` アクション状態変化をポインタのクリックに変換する。</summary>
    public void SetPressed(ETrackedControllerRole role, bool pressed)
    {
        var pointer = role switch
        {
            ETrackedControllerRole.LeftHand => _left,
            ETrackedControllerRole.RightHand => _right,
            _ => null,
        };
        if (pointer == null || pointer.Pressed == pressed) return;
        pointer.Pressed = pressed;

        var browser = _target?.Browser;
        if (browser == null || pointer.LastUv == null)
        {
            Console.WriteLine($"[steamvr] クリックを無視: browser={(browser == null ? "null" : "ok")} LastUv={(pointer.LastUv == null ? "null(パネルに当たっていない)" : "ok")}");
            return;
        }
        var (x, y) = ToBrowserPixels(pointer.LastUv.Value, browser);
        Console.WriteLine($"[steamvr] クリック送信: role={role} pressed={pressed} x={x} y={y}");
        browser.GetBrowser().GetHost().SendMouseClickEvent(x, y, MouseButtonType.Left, !pressed, 1, CefEventFlags.None);
    }

    public void Dispose()
    {
        if (_disposed) return;
        _disposed = true;
        if (_left.OverlayHandle != 0) OpenVR.Overlay.DestroyOverlay(_left.OverlayHandle);
        if (_right.OverlayHandle != 0) OpenVR.Overlay.DestroyOverlay(_right.OverlayHandle);
    }

    private sealed class PointerState
    {
        public ulong OverlayHandle;
        public Vector2? LastUv;
        public bool Pressed;
    }
}
