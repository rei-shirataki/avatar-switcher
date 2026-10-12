using System.Runtime.InteropServices;
using AvatarSwitcher.OverlaySidecar.Utils;
using CefSharp;
using CefSharp.Enums;
using CefSharp.OffScreen;
using CefSharp.Structs;
using SharpDX.Direct3D11;

namespace AvatarSwitcher.OverlaySidecar.Rendering;

/// <summary>
/// CefSharp をウィンドウレスモードで動かし、ペイント結果を D3D11 テクスチャへ書き込む。
/// GPU共有テクスチャは使わず、<see cref="IRenderHandler.OnPaint"/> で受け取る
/// CPU側のBGRAピクセルバッファをそのまま <see cref="Texture2D"/> にマップ&コピーする
/// （非アクセラレーション経路。M1のスコープ判断）。
///
/// 参照実装: OyasumiVR src-overlay-sidecar/Utils/Browsers/OffscreenBrowser.cs,
/// NonAcceleratedOffscreenBrowser.cs（同リポジトリはさらに
/// vrcx-team/VRCX の OffScreenBrowserLegacy.cs を参照元として明記している）
/// </summary>
internal sealed class OffscreenBrowser : ChromiumWebBrowser, IRenderHandler
{
    private readonly ReaderWriterLockSlim _paintBufferLock = new();
    private GCHandle _paintBuffer;
    private int _width;
    private int _height;
    private Texture2D? _renderTarget;

    /// <summary>直近でペイントが来た時刻（Unixミリ秒）。フレーム更新の要否判定に使う。</summary>
    public long LastPaintAt { get; private set; }

    public OffscreenBrowser(string address, uint width, uint height)
        : base(address, automaticallyCreateBrowser: false)
    {
        var windowInfo = new WindowInfo();
        windowInfo.SetAsWindowless(IntPtr.Zero);
        windowInfo.WindowlessRenderingEnabled = true;
        windowInfo.SharedTextureEnabled = false;
        windowInfo.Width = (int)width;
        windowInfo.Height = (int)height;

        var browserSettings = new BrowserSettings
        {
            WindowlessFrameRate = 60,
            DefaultEncoding = "UTF-8",
        };

        CreateBrowser(windowInfo, browserSettings);

        Size = new System.Drawing.Size((int)width, (int)height);
        RenderHandler = this;
    }

    public void SetTextureTarget(Texture2D? renderTarget)
    {
        _renderTarget = renderTarget;
    }

    /// <summary>直近のペイントバッファをテクスチャへ書き込む。フレーム更新ループから毎ティック呼ぶ。</summary>
    public void WriteToTexture()
    {
        if (_renderTarget == null) return;

        _paintBufferLock.EnterReadLock();
        try
        {
            if (_width <= 0 || _height <= 0) return;

            // CEF のペイントサイズとテクスチャサイズは通常どちらも同じ解像度だが、
            // 食い違った場合（Size 変更・デバイススケール等）にマップ領域を越えて
            // 書き込まないよう、重なる範囲だけコピーする。
            var desc = _renderTarget.Description;
            var copyWidth = Math.Min(_width, desc.Width);
            var copyHeight = Math.Min(_height, desc.Height);

            var context = _renderTarget.Device.ImmediateContext;
            var dataBox = context.MapSubresource(_renderTarget, 0, MapMode.WriteDiscard, MapFlags.None);
            if (!dataBox.IsEmpty)
            {
                var sourcePtr = _paintBuffer.AddrOfPinnedObject();
                var destinationPtr = dataBox.DataPointer;
                var pitch = _width * 4;
                var rowPitch = dataBox.RowPitch;
                if (pitch == rowPitch && copyWidth == _width && copyHeight == _height)
                {
                    WinApi.RtlCopyMemory(destinationPtr, sourcePtr, (uint)(_width * _height * 4));
                }
                else
                {
                    // テクスチャの実メモリ行幅(rowPitch)がピクセル幅(pitch)と一致しない
                    // (ドライバのアライメント都合)場合やサイズが異なる場合は1行ずつコピーする。
                    var copyBytes = (uint)(copyWidth * 4);
                    for (var y = 0; y < copyHeight; y++)
                    {
                        WinApi.RtlCopyMemory(destinationPtr, sourcePtr, copyBytes);
                        sourcePtr += pitch;
                        destinationPtr += rowPitch;
                    }
                }
            }

            context.UnmapSubresource(_renderTarget, 0);
        }
        finally
        {
            _paintBufferLock.ExitReadLock();
        }
    }

    private bool _disposed;

    /// <summary>
    /// 以前は Dispose() を new で隠蔽していたため、IDisposable や基底型経由
    /// （CefSharp 自身の終了時の破棄など）で呼ばれるとピン留めしたペイントバッファと
    /// ロックが解放されなかった。基底の Dispose(bool) をオーバーライドしてどの経路でも解放する。
    /// </summary>
    protected override void Dispose(bool disposing)
    {
        if (_disposed) return;
        _disposed = true;

        if (disposing) RenderHandler = null;
        base.Dispose(disposing);

        // ピン留めは GC から見えないので、ファイナライザ経由（disposing=false）でも解放する。
        _paintBufferLock.EnterWriteLock();
        try
        {
            if (_paintBuffer.IsAllocated) _paintBuffer.Free();
        }
        finally
        {
            _paintBufferLock.ExitWriteLock();
        }

        if (disposing) _paintBufferLock.Dispose();
    }

    // ── IRenderHandler ──────────────────────────────────────────────────

    ScreenInfo? IRenderHandler.GetScreenInfo() => null;

    bool IRenderHandler.GetScreenPoint(int viewX, int viewY, out int screenX, out int screenY)
    {
        screenX = viewX;
        screenY = viewY;
        return false;
    }

    Rect IRenderHandler.GetViewRect() => new(0, 0, Size.Width, Size.Height);

    void IRenderHandler.OnAcceleratedPaint(PaintElementType type, Rect dirtyRect, AcceleratedPaintInfo paintInfo)
    {
        // 非アクセラレーション経路のみサポートするため未使用。
    }

    void IRenderHandler.OnCursorChange(IntPtr cursor, CursorType type, CursorInfo customCursorInfo)
    {
    }

    void IRenderHandler.OnImeCompositionRangeChanged(CefSharp.Structs.Range selectedRange, Rect[] characterBounds)
    {
    }

    void IRenderHandler.OnPaint(PaintElementType type, Rect dirtyRect, IntPtr buffer, int width, int height)
    {
        if (type != PaintElementType.View) return;

        _paintBufferLock.EnterWriteLock();
        try
        {
            if (_width != width || _height != height)
            {
                _width = width;
                _height = height;
                if (_paintBuffer.IsAllocated) _paintBuffer.Free();
                _paintBuffer = GCHandle.Alloc(new byte[_width * _height * 4], GCHandleType.Pinned);
            }

            WinApi.RtlCopyMemory(_paintBuffer.AddrOfPinnedObject(), buffer, (uint)(width * height * 4));
            LastPaintAt = DateTimeOffset.UtcNow.ToUnixTimeMilliseconds();
        }
        finally
        {
            _paintBufferLock.ExitWriteLock();
        }
    }

    void IRenderHandler.OnPopupShow(bool show)
    {
    }

    void IRenderHandler.OnPopupSize(Rect rect)
    {
    }

    void IRenderHandler.OnVirtualKeyboardRequested(IBrowser browser, TextInputMode inputMode)
    {
    }

    bool IRenderHandler.StartDragging(IDragData dragData, DragOperationsMask mask, int x, int y) => false;

    void IRenderHandler.UpdateDragCursor(DragOperationsMask operation)
    {
    }
}
