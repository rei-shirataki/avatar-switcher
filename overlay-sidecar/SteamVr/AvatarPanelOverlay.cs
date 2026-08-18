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
    private readonly int _wsPort;
    private readonly string _wsToken;
    private ulong _overlayHandle;
    private OffscreenBrowser? _browser;
    private Texture2D? _texture;
    private EVROverlayError _lastTextureError = EVROverlayError.None;
    private bool _disposed;

    public ulong OverlayHandle => _overlayHandle;
    public OffscreenBrowser? Browser => _browser;

    public AvatarPanelOverlay(D3D11Context d3D, int wsPort, string wsToken)
    {
        _d3D = d3D;
        _wsPort = wsPort;
        _wsToken = wsToken;
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

        // ブラウザ生成後に別途 LoadHtml() を呼ぶ実装だと、CefSharp側のネイティブ
        // ブラウザ初期化が非同期のため LoadHtml が初期化完了前に呼ばれて静かに
        // 無視されるレースになる（実機確認で発生：当たり判定は効くのに
        // ページ内容が一切表示されない）。参照実装(OyasumiVR BrowserManager.GetBrowser)
        // に倣い、コンストラクタの address に直接コンテンツを渡して最初のナビゲーション
        // として読み込ませる。
        _browser = new OffscreenBrowser(ResolveOverlayUiUrl(), Resolution, Resolution);
        LogBrowserEvents(_browser);
        _browser.SetTextureTarget(_texture);

        PlaceInFrontOfHead();
        OpenVR.Overlay.ShowOverlay(_overlayHandle);
    }

    /// <summary>
    /// overlay-ui (Angular) の読み込み先URLを決める。優先順位:
    /// 1. ビルド済み dist が実行ファイル横の overlay-ui/ に配置されていればそれ (本番相当)
    /// 2. 開発用URL環境変数 (`ng serve --project overlay-ui` を別途起動して指す)
    /// 3. どちらも無ければ M1 のテストページにフォールバック（overlay-ui未セットアップでも
    ///    オーバーレイ描画パイプライン自体の疎通確認は引き続きできるようにする）
    /// いずれも Rust core の WS ブリッジへ接続するための port/token をクエリで渡す。
    /// </summary>
    private string ResolveOverlayUiUrl()
    {
        var query = $"?port={_wsPort}&token={Uri.EscapeDataString(_wsToken)}";

        var distIndex = Path.Combine(AppDomain.CurrentDomain.BaseDirectory, "overlay-ui", "index.html");
        if (File.Exists(distIndex))
        {
            return new Uri(distIndex).AbsoluteUri + query;
        }

        var devUrl = Environment.GetEnvironmentVariable("AVATAR_SWITCHER_OVERLAY_UI_DEV_URL");
        if (!string.IsNullOrEmpty(devUrl))
        {
            return devUrl.TrimEnd('/') + "/" + query;
        }

        Console.WriteLine("[steamvr] overlay-ui が見つからないため M1 テストページで代替します" +
            "（dist/overlay-ui/browser を配置するか AVATAR_SWITCHER_OVERLAY_UI_DEV_URL を設定してください）");
        return "data:text/html;base64," + Convert.ToBase64String(System.Text.Encoding.UTF8.GetBytes(TestPage.Html));
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
