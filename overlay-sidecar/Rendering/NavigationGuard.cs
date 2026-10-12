using CefSharp;
using CefSharp.Handler;

namespace AvatarSwitcher.OverlaySidecar.Rendering;

/// <summary>
/// overlay-ui のページ（URL フラグメントに WS トークンを持つ）が、最初に読み込んだ
/// オリジン以外へ遷移したりポップアップを開いたりしないようにする。
/// 表示するのは自前の overlay-ui だけなので、それ以外への遷移は不要。
/// </summary>
internal sealed class NavigationGuard : RequestHandler
{
    private readonly string _allowedOrigin;

    public NavigationGuard(string initialUrl)
    {
        _allowedOrigin = OriginOf(initialUrl);
    }

    /// <summary>scheme://host:port。data: など階層を持たない URL は scheme だけで比較する。</summary>
    internal static string OriginOf(string url) =>
        Uri.TryCreate(url, UriKind.Absolute, out var uri) && !string.IsNullOrEmpty(uri.Host)
            ? $"{uri.Scheme}://{uri.Host}:{uri.Port}"
            : url.Split(':', 2)[0] + ":";

    protected override bool OnBeforeBrowse(IWebBrowser chromiumWebBrowser, IBrowser browser, IFrame frame,
        IRequest request, bool userGesture, bool isRedirect)
    {
        if (OriginOf(request.Url) == _allowedOrigin) return false;
        Console.Error.WriteLine($"[overlay-ui] 許可されていない遷移をブロックしました: {OriginOf(request.Url)}");
        return true; // true = 遷移をキャンセル
    }
}

/// <summary>ポップアップ（window.open や target=_blank）を開かせない。</summary>
internal sealed class NoPopupLifeSpanHandler : LifeSpanHandler
{
    protected override bool OnBeforePopup(IWebBrowser chromiumWebBrowser, IBrowser browser, IFrame frame,
        string targetUrl, string targetFrameName, WindowOpenDisposition targetDisposition, bool userGesture,
        IPopupFeatures popupFeatures, IWindowInfo windowInfo, IBrowserSettings browserSettings,
        ref bool noJavascriptAccess, out IWebBrowser? newBrowser)
    {
        newBrowser = null;
        return true; // true = ポップアップをキャンセル
    }
}
