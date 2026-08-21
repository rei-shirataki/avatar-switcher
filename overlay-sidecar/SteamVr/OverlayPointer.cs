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
    // #23フォローアップ: OyasumiVR自体も0.02mだが、ユーザー要望により一回り小さくしている。
    private const float PointerWidthMeters = 0.015f;
    private const uint PointerSortOrder = 150;

    private readonly PointerState _left = new();
    private readonly PointerState _right = new();
    private AvatarPanelOverlay? _target;
    private bool _disposed;

    public OverlayPointer()
    {
        var (pixels, width, height) = LoadPointerImage();
        SetUpPointerOverlay(_left, "com.rei-shirataki.avatar-switcher:PointerLeft", "AvatarSwitcher Left Pointer", pixels, width, height);
        SetUpPointerOverlay(_right, "com.rei-shirataki.avatar-switcher:PointerRight", "AvatarSwitcher Right Pointer", pixels, width, height);
    }

    public void SetTarget(AvatarPanelOverlay? overlay)
    {
        _target = overlay;
    }

    private static void SetUpPointerOverlay(PointerState state, string key, string name, byte[] pixels, uint width, uint height)
    {
        OvrUtils.GetOrCreateOverlay(key, name, ref state.OverlayHandle);
        OpenVR.Overlay.SetOverlayWidthInMeters(state.OverlayHandle, PointerWidthMeters);
        OpenVR.Overlay.SetOverlaySortOrder(state.OverlayHandle, PointerSortOrder);

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
    /// ポインタ画像を埋め込みリソース(Resources/pointer.png、OyasumiVR由来。
    /// Resources/VENDORING.md参照)から読み込み、<c>SetOverlayRaw</c>へそのまま渡せる
    /// BGRAバイト列に変換する。System.Drawing.Bitmapの<c>Format32bppArgb</c>は
    /// Windows(GDI+)上ではメモリ上で既にB,G,R,Aの順に並ぶため、追加のチャンネル
    /// 入れ替えは不要（以前の手描き円形ドットを置き換えた）。
    /// </summary>
    private static (byte[] pixels, uint width, uint height) LoadPointerImage()
    {
        var assembly = System.Reflection.Assembly.GetExecutingAssembly();
        using var stream = assembly.GetManifestResourceStream("AvatarSwitcher.OverlaySidecar.Resources.pointer.png")
            ?? throw new InvalidOperationException("pointer.png の埋め込みリソースが見つかりません");
        using var bitmap = new System.Drawing.Bitmap(stream);
        var width = bitmap.Width;
        var height = bitmap.Height;
        var rect = new System.Drawing.Rectangle(0, 0, width, height);
        var bmpData = bitmap.LockBits(rect, System.Drawing.Imaging.ImageLockMode.ReadOnly,
            System.Drawing.Imaging.PixelFormat.Format32bppArgb);
        try
        {
            var pixels = new byte[width * height * 4];
            System.Runtime.InteropServices.Marshal.Copy(bmpData.Scan0, pixels, 0, pixels.Length);
            return (pixels, (uint)width, (uint)height);
        }
        finally
        {
            bitmap.UnlockBits(bmpData);
        }
    }

    /// <summary>
    /// 両コントローラーからパネルへレイキャストし、交点にポインタを表示、
    /// UV座標からCEFへマウス移動イベントを注入する。
    ///
    /// RenderLoop（HMDリフレッシュレート、90〜144Hz程度）から毎ティック呼ぶ。
    /// 以前はMainLoop側の32ms固定ティックから呼んでいたが、クリック検知
    /// (<see cref="SetPressed"/>、こちらは別スレッドのMainLoopで32ms間隔のまま)
    /// が参照する座標が常に最大32ms古くなり、パネルから距離が離れるほど
    /// 顕著な「クリック不成立」の原因になっていた。参照実装OyasumiVRの
    /// OverlayPointer.Startも同様にHMDリフレッシュレートの専用スレッドで
    /// 座標を更新しており、それに合わせた。二つのスレッドから同じ
    /// <see cref="PointerState"/> に触れるため、フィールドアクセスは
    /// pointerインスタンス自体をロックオブジェクトとして保護する
    /// （OyasumiVRも同じ流儀でPointerDataインスタンスをロックしている）。
    /// </summary>
    public void UpdateRaycast()
    {
        var poseBuffer = new TrackedDevicePose_t[OpenVR.k_unMaxTrackedDeviceCount];
        UpdateForController(ETrackedControllerRole.LeftHand, _left, poseBuffer);
        UpdateForController(ETrackedControllerRole.RightHand, _right, poseBuffer);
    }

    private void UpdateForController(ETrackedControllerRole role, PointerState pointer, TrackedDevicePose_t[] poseBuffer)
    {
        if (_target == null || _target.OverlayHandle == 0 || !_target.IsShown)
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

        // RenderLoopスレッドからの書き込みとMainLoopスレッド(SetPressed)からの
        // 読み書きが競合するため、pointerインスタンス自体をロックする。
        lock (pointer)
        {
            pointer.LastUv = uv;
            SendMouseMove(pointer);
        }
    }

    private void Hide(PointerState pointer)
    {
        OpenVR.Overlay.HideOverlay(pointer.OverlayHandle);
        lock (pointer)
        {
            // 交点を失った瞬間のCEF側マウス状態をクリアする(離脱通知)。
            if (pointer.LastUv.HasValue)
            {
                SendMouseMove(pointer, mouseLeave: true);
            }

            pointer.LastUv = null;
            pointer.Pressed = false;
        }
    }

    /// <summary>呼び出し元が既にpointerをlockしている前提（UpdateForController/Hide経由のみ）。</summary>
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

    /// <summary>
    /// SteamVR Inputの `OverlayInteract` アクション状態変化をポインタのクリックに変換する。
    /// 押した瞬間・離した瞬間それぞれで、その時点のLastUvを使ってmousedown/mouseupを
    /// 送る（参照実装OyasumiVRのOnInteractPress/OnInteractReleaseと同じパターン）。
    /// UpdateRaycastがHMDリフレッシュレートの専用スレッドで座標を常に新鮮に
    /// 保っている前提なので、press/releaseで別座標になっても実用上問題ない
    /// （このパターン自体は不具合の原因ではなく、座標の鮮度がMainLoop側の
    /// 32ms固定ティックで1ティック分古かったことが原因だった）。
    /// </summary>
    public void SetPressed(ETrackedControllerRole role, bool pressed)
    {
        var pointer = role switch
        {
            ETrackedControllerRole.LeftHand => _left,
            ETrackedControllerRole.RightHand => _right,
            _ => null,
        };
        if (pointer == null) return;

        Vector2? uv;
        lock (pointer)
        {
            if (pointer.Pressed == pressed) return;
            pointer.Pressed = pressed;
            uv = pointer.LastUv;
        }

        var browser = _target?.Browser;
        if (browser == null || uv == null || _target?.IsShown != true)
        {
            // #37フォローアップ: LastUvはRenderLoopスレッドが次ティックでクリアするまで
            // 残るため、Dashboardが開いた直後の数msはIsShownを見ないと隠れたパネルへ
            // クリックが漏れる（UpdateForController側のIsShownチェックだけでは防げない）。
            Console.WriteLine($"[steamvr] クリックを無視: browser={(browser == null ? "null" : "ok")} LastUv={(uv == null ? "null(パネルに当たっていない)" : "ok")} IsShown={_target?.IsShown}");
            return;
        }
        var (x, y) = ToBrowserPixels(uv.Value, browser);
        Console.WriteLine($"[steamvr] クリック送信: role={role} pressed={pressed} x={x} y={y}");
        browser.GetBrowser().GetHost().SendMouseClickEvent(x, y, MouseButtonType.Left, mouseUp: !pressed, clickCount: 1, CefEventFlags.None);
    }

    /// <summary>
    /// #22: スティック/トラックパッドのY軸値を、現在ポインタが指しているUV座標位置への
    /// CEFホイールイベントに変換する。<see cref="ScrollPixelsPerUnit"/>は初期値の見込みで、
    /// 実機での操作感次第で調整が必要。スティック上方向(y&gt;0)を「上へスクロール」
    /// （コンテンツが下に動き、上側が見える）に対応させている。逆に感じる場合は
    /// 符号を反転する。
    ///
    /// スクロール自体の入力は右スティック固定だが、狙っている手（左右どちらでパネルを
    /// 指しているか）は問わない。ユーザー要望により、左手でパネルを狙いながら右スティックで
    /// スクロールする操作も成立させたいため、_right→_leftの順でLastUvが有効な方を使う。
    /// </summary>
    private const int ScrollPixelsPerUnit = 60;

    public void Scroll(float deltaY)
    {
        Vector2? uv;
        lock (_right)
        {
            uv = _right.LastUv;
        }
        if (uv == null)
        {
            lock (_left)
            {
                uv = _left.LastUv;
            }
        }

        var browser = _target?.Browser;
        if (browser == null || uv == null || _target?.IsShown != true) return;

        var (x, y) = ToBrowserPixels(uv.Value, browser);
        var wheelDeltaY = (int)(deltaY * ScrollPixelsPerUnit);
        browser.GetBrowser().GetHost().SendMouseWheelEvent(x, y, 0, wheelDeltaY, CefEventFlags.None);
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
