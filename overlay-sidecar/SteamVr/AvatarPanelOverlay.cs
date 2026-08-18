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
/// 配置方式は2種類（#28、ユーザー設定で選択、既定はHand）:
/// - Hand: 表示ONの瞬間にダブルプレスした手のコントローラーへ
///   <see cref="OpenVR.Overlay.SetOverlayTransformTrackedDeviceRelative"/> でアタッチする。
///   一度アタッチすればSteamVR側が継続的に追従させるため、毎フレームの座標計算は不要
///   （コントローラーの再接続でデバイスindexが変わりうるため、表示ONのたびに取り直す）。
/// - Space: OyasumiVR DashboardOverlay.GetTargetTransform と同じ式で表示ONの瞬間だけ
///   手元付近に配置し、以降は空間に静止させる。ハンドトラッキングが無効な場合は
///   頭部前方固定にフォールバックする。
///
/// 開閉トグル(#21)は SteamVR Inputの `OpenOverlay` アクションと組み合わせて実装済み(<see cref="SetVisible"/>)。
///
/// 参照実装: OyasumiVR src-overlay-sidecar/Overlays/BaseWebOverlay.cs, DashboardOverlay.cs
/// </summary>
internal sealed class AvatarPanelOverlay : IDisposable
{
    private const string OverlayKey = "com.rei-shirataki.avatar-switcher:AvatarPanel";
    private const string OverlayName = "AvatarSwitcher Panel";
    private const uint Resolution = 1024;
    // パネルは1024x1024pxの単一テクスチャとしてレンダリングされ、この幅(m)で
    // 物理サイズへスケールするだけ（文字サイズ等のCSSはこのテクスチャ内に固定）。
    // そのためWidthMetersを大きくすると文字・ボタンを含む全要素が一括で拡大される。
    // 実機確認(#28)で「文字・ボタンが小さい、特にHandモードが顕著」と判明したため、
    // 初期値(Space: 0.45f, Hand: 0.25f)から引き上げた。
    private const float WidthMeters = 0.55f;
    /// <summary>Handモード時の幅（m）。腕に付けるため通常より小さくするが、実機確認で
    /// 0.25fは読みにくいレベルだったため引き上げた（さらなる要調整の可能性あり）。</summary>
    private const float HandWidthMeters = 0.4f;
    /// <summary>頭部前方のオフセット（m）。DashboardOverlayのフォールバック配置に合わせた値。</summary>
    private const float ForwardOffsetMeters = 0.55f;
    /// <summary>ボタン長押し(<see cref="ToggleSize"/>)で拡大する際の倍率。</summary>
    private const float EnlargedScale = 1.4f;
    /// <summary>Space方式（OyasumiVR式）の手元オフセット。座標系はワールド、頭部の向きのみで回転する。</summary>
    private static readonly Vector3 NearHandOffset = new(0, 0.15f, -0.2f);
    /// <summary>Handモードのコントローラー相対オフセット（m）。手前上方に置き、見下ろす角度に傾ける。</summary>
    private static readonly Vector3 HandRelativeOffset = new(0, 0.05f, -0.08f);
    /// <summary>Handモードのパネル傾き（度）。腕時計のように持ち上げて見下ろす想定。</summary>
    private const float HandTiltDegrees = -60f;

    private readonly D3D11Context _d3D;
    private readonly int _wsPort;
    private readonly string _wsToken;
    private readonly int? _uiPort;
    private readonly PlacementMode _placementMode;
    private ulong _overlayHandle;
    private OffscreenBrowser? _browser;
    private Texture2D? _texture;
    private EVROverlayError _lastTextureError = EVROverlayError.None;
    private bool _disposed;
    // #24: 起動直後はオーバーレイを表示しない。ユーザーが必要な時だけ
    // #21のダブルプレスで呼び出す想定。
    private bool _visible;
    /// <summary>ボタン長押しで拡大された状態か。表示/非表示や配置とは独立に、
    /// アプリ終了までセッション内で保持する（永続化はしない）。</summary>
    private bool _enlarged;

    public ulong OverlayHandle => _overlayHandle;
    public OffscreenBrowser? Browser => _browser;
    public bool IsVisible => _visible;

    public AvatarPanelOverlay(D3D11Context d3D, int wsPort, string wsToken, int? uiPort, PlacementMode placementMode)
    {
        _d3D = d3D;
        _wsPort = wsPort;
        _wsToken = wsToken;
        _uiPort = uiPort;
        _placementMode = placementMode;
    }

    public async Task OpenAsync()
    {
        var err = OvrUtils.GetOrCreateOverlay(OverlayKey, OverlayName, ref _overlayHandle);
        if (err != EVROverlayError.None)
        {
            Console.Error.WriteLine($"[steamvr] オーバーレイの作成に失敗しました: {err}");
            return;
        }

        ApplyWidth();

        _texture = await _d3D.CreateCpuWritableTextureAsync(Resolution);

        // ブラウザ生成後に別途 LoadHtml() を呼ぶ実装だと、CefSharp側のネイティブ
        // ブラウザ初期化が非同期のため LoadHtml が初期化完了前に呼ばれて静かに
        // 無視されるレースになる（実機確認で発生：当たり判定は効くのに
        // ページ内容が一切表示されない）。参照実装(OyasumiVR BrowserManager.GetBrowser)
        // に倣い、コンストラクタの address に直接コンテンツを渡して最初のナビゲーション
        // として読み込ませる。
        _browser = new OffscreenBrowser(ResolveOverlayUiUrl(), Resolution, Resolution);
        LogBrowserEvents(_browser);
        _browser.SetTextureTarget(_texture);

        // #24: 起動直後は表示しない。位置だけ先に頭部前方へ合わせておき、
        // 最初にSetVisible(true)が呼ばれた瞬間に正しい位置で出るようにする。
        PlaceInFrontOfHead();
        OpenVR.Overlay.HideOverlay(_overlayHandle);
    }

    /// <summary>
    /// overlay-ui (Angular) の読み込み先URLを決める。優先順位:
    /// 1. 開発用URL環境変数 (`ng serve --project overlay-ui` を別途起動して指す。
    ///    sidecar.rs::resolve_sidecar_path と同じく「開発用オーバーライドを
    ///    最優先」の流儀に揃えている)
    /// 2. Rust core が起動する overlay-ui 静的ファイルサーバー（`--ui-port` で
    ///    渡される。本番相当。file:// 直読みは Angular esbuild が出力する
    ///    &lt;script type="module"&gt; がES moduleの仕様上 file:// (origin: null) からの
    ///    読み込みを常にCORSでブロックするため実機で機能しなかった
    ///    （Access to script at 'file:///...' ... blocked by CORS policy を確認）。
    ///    http://127.0.0.1 経由にすることで回避する
    /// 3. どちらも無ければ M1 のテストページにフォールバック（overlay-ui未セットアップでも
    ///    オーバーレイ描画パイプライン自体の疎通確認は引き続きできるようにする）
    /// いずれも Rust core の WS ブリッジへ接続するための port/token をクエリで渡す。
    /// </summary>
    private string ResolveOverlayUiUrl()
    {
        var query = $"?port={_wsPort}&token={Uri.EscapeDataString(_wsToken)}";

        var devUrl = Environment.GetEnvironmentVariable("AVATAR_SWITCHER_OVERLAY_UI_DEV_URL");
        if (!string.IsNullOrEmpty(devUrl))
        {
            return devUrl.TrimEnd('/') + "/" + query;
        }

        if (_uiPort is { } uiPort)
        {
            return $"http://127.0.0.1:{uiPort}/index.html{query}";
        }

        Console.WriteLine("[steamvr] overlay-ui 静的ファイルサーバーが起動していないため M1 テストページで代替します" +
            "（dist/overlay-ui/browser をビルドするか AVATAR_SWITCHER_OVERLAY_UI_DEV_URL を設定してください）");
        return "data:text/html;base64," + Convert.ToBase64String(System.Text.Encoding.UTF8.GetBytes(TestPage.Html));
    }

    /// <summary>
    /// 配置方式(Hand/Space)ごとの基準幅に、拡大状態なら<see cref="EnlargedScale"/>を
    /// 掛けて反映する。<see cref="OpenVR.Overlay.SetOverlayTransformTrackedDeviceRelative"/>
    /// / <see cref="OpenVR.Overlay.SetOverlayTransformAbsolute"/>で設定した基準点(アンカー)は
    /// 幅を変えても動かないため、位置の再計算は不要。
    /// </summary>
    private void ApplyWidth()
    {
        var baseWidth = _placementMode == PlacementMode.Hand ? HandWidthMeters : WidthMeters;
        OpenVR.Overlay.SetOverlayWidthInMeters(_overlayHandle, _enlarged ? baseWidth * EnlargedScale : baseWidth);
    }

    /// <summary>
    /// ボタン長押し（新規、`/actions/toggle/in/OpenOverlay`の長押し検知から呼ばれる）で
    /// パネルサイズを拡大/縮小トグルする。パネルが小さく見づらいという実機フィード
    /// バック(#28のWidthMeters引き上げ)を受けて、恒久的な既定値の変更だけでなく、
    /// ユーザーがその場で更に拡大できる手段も用意した。
    /// </summary>
    public void ToggleSize()
    {
        _enlarged = !_enlarged;
        ApplyWidth();
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
    /// 表示/非表示を切り替える。#21: `/actions/toggle/in/OpenOverlay` のダブルプレスから呼ばれる。
    /// <paramref name="role"/> はダブルプレスした手（#28、Hand/Space方式どちらの配置にも使う）。
    /// 取得できなかった場合は <see cref="ETrackedControllerRole.Invalid"/> を渡せば
    /// 各配置メソッドが頭部前方固定へフォールバックする。
    /// </summary>
    public void SetVisible(bool visible, ETrackedControllerRole role = ETrackedControllerRole.Invalid)
    {
        if (_disposed || _visible == visible) return;
        _visible = visible;
        if (visible)
        {
            if (_placementMode == PlacementMode.Hand)
            {
                AttachToHand(role);
            }
            else
            {
                PlaceNearHand(role);
            }
            OpenVR.Overlay.ShowOverlay(_overlayHandle);
        }
        else
        {
            OpenVR.Overlay.HideOverlay(_overlayHandle);
        }
    }

    /// <summary>
    /// Handモード（#28）: <paramref name="role"/> のコントローラーへオーバーレイをアタッチする。
    /// <see cref="OpenVR.Overlay.SetOverlayTransformTrackedDeviceRelative"/> は一度呼べば
    /// SteamVR側が継続的に追従させるため、毎フレームの座標更新は不要。コントローラーの
    /// 再接続でデバイスindexが変わりうるため、表示ONのたびに取り直して呼び直す。
    /// </summary>
    private void AttachToHand(ETrackedControllerRole role)
    {
        var deviceIndex = OpenVR.System.GetTrackedDeviceIndexForControllerRole(role);
        if (deviceIndex == OpenVR.k_unTrackedDeviceIndexInvalid)
        {
            PlaceInFrontOfHead();
            return;
        }

        var tilt = Matrix4x4.CreateRotationX(HandTiltDegrees * MathF.PI / 180f);
        var offset = Matrix4x4.CreateTranslation(HandRelativeOffset);
        var transform = (tilt * offset).ToHmdMatrix34T();
        var err = OpenVR.Overlay.SetOverlayTransformTrackedDeviceRelative(_overlayHandle, deviceIndex, ref transform);
        if (err != EVROverlayError.None)
        {
            Console.Error.WriteLine($"[steamvr] SetOverlayTransformTrackedDeviceRelative 失敗: {err}");
            PlaceInFrontOfHead();
        }
    }

    /// <summary>
    /// Spaceモード（#28）: OyasumiVR DashboardOverlay.GetTargetTransform と同じ式。
    /// <paramref name="role"/> のコントローラー位置＋頭部の向き（位置は含めない）を使って
    /// 手元付近に配置し、以降は空間に静止する（毎フレーム追従はしない）。
    /// ハンドトラッキングが無効な場合は頭部前方固定にフォールバックする。
    /// </summary>
    private void PlaceNearHand(ETrackedControllerRole role)
    {
        var deviceIndex = OpenVR.System.GetTrackedDeviceIndexForControllerRole(role);
        if (deviceIndex == OpenVR.k_unTrackedDeviceIndexInvalid)
        {
            PlaceInFrontOfHead();
            return;
        }

        var poseBuffer = new TrackedDevicePose_t[OpenVR.k_unMaxTrackedDeviceCount];
        var headPose = OvrUtils.GetHeadPose(poseBuffer);
        if (headPose.eTrackingResult != ETrackingResult.Running_OK)
        {
            PlaceInFrontOfHead();
            return;
        }

        var handPose = poseBuffer[deviceIndex];
        if (!handPose.bPoseIsValid || handPose.eTrackingResult != ETrackingResult.Running_OK)
        {
            PlaceInFrontOfHead();
            return;
        }

        var headMatrix = headPose.mDeviceToAbsoluteTracking.ToMatrix4X4();
        var headRotationOnly = Matrix4x4.CreateFromQuaternion(Quaternion.CreateFromRotationMatrix(headMatrix));
        var handMatrix = handPose.mDeviceToAbsoluteTracking.ToMatrix4X4();
        var handPositionOnly = Matrix4x4.CreateTranslation(handMatrix.Translation);

        var posOffset = Matrix4x4.CreateTranslation(NearHandOffset);
        var transform = (posOffset * headRotationOnly * handPositionOnly).ToHmdMatrix34T();
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
            Console.WriteLine($"[overlay-ui console] {e.Level} {e.Message} ({TruncateSource(e.Source)}:{e.Line})");
        browser.LoadError += (_, e) =>
            Console.Error.WriteLine($"[overlay-ui] load error: {TruncateSource(e.FailedUrl)} {e.ErrorCode} {e.ErrorText}");
    }

    /// <summary>M1のテストページはdata URIで読み込むため、そのままログに出すと1行が数KBになる。</summary>
    private static string TruncateSource(string source) =>
        source.Length > 80 ? source[..80] + "…" : source;

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
