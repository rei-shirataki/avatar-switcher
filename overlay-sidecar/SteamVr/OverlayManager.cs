using System.Runtime.InteropServices;
using AvatarSwitcher.OverlaySidecar.Bridge;
using AvatarSwitcher.OverlaySidecar.Rendering;
using AvatarSwitcher.OverlaySidecar.Utils;
using Valve.VR;

namespace AvatarSwitcher.OverlaySidecar.SteamVr;

/// <summary>
/// SteamVR (OpenVR) 全体のライフサイクルを管理する。
///
/// - `VRApplication_Background` で初期化する: `VRApplication_Overlay` と違い、
///   このアプリの起動をトリガーにSteamVR自体を自動起動させない
///   （ヘッドセットを被っていないのに毎回SteamVRが立ち上がるのは望ましくない）。
/// - SteamVR未起動なら数秒おきにリトライし続け、起動中は `VREvent_Quit` を検知して
///   後片付けし待機状態へ戻る。
/// - アクティブ中はイベント/入力検知ループ(_mainThread)とテクスチャ更新ループ
///   (_renderThread)を別スレッドで回す。参照実装(OyasumiVR OvrManager.cs)と
///   同じ2スレッド構成。
/// </summary>
internal sealed class OverlayManager
{
    private static readonly TimeSpan RetryInterval = TimeSpan.FromSeconds(3);

    private readonly WsBridgeClient _bridge;
    private readonly int? _uiPort;
    private readonly CancellationToken _shutdownToken;

    private volatile bool _active;
    private D3D11Context? _d3D;
    private AvatarPanelOverlay? _panel;
    private OverlayPointer? _pointer;
    private ulong _actionSetHandle;
    private ulong _overlayInteractActionHandle;

    public OverlayManager(WsBridgeClient bridge, int? uiPort, CancellationToken shutdownToken)
    {
        _bridge = bridge;
        _uiPort = uiPort;
        _shutdownToken = shutdownToken;
    }

    public void Run()
    {
        new Thread(MainLoop) { IsBackground = true }.Start();
        new Thread(RenderLoop) { IsBackground = true }.Start();
    }

    private void MainLoop()
    {
        var e = new VREvent_t();

        while (!_shutdownToken.IsCancellationRequested)
        {
            Thread.Sleep(32);

            if (!_active)
            {
                if (!TryActivate())
                {
                    Thread.Sleep(RetryInterval);
                    continue;
                }

                _active = true;
                _ = _bridge.SendSteamVrStatusAsync(true, _shutdownToken);
            }

            DetectOverlayInteract();
            _pointer?.UpdateRaycast();

            while (OpenVR.System.PollNextEvent(ref e, (uint)Marshal.SizeOf(e)))
            {
                if ((EVREventType)e.eventType == EVREventType.VREvent_Quit)
                {
                    Console.WriteLine("[steamvr] SteamVRからQuitイベントを受信、後片付けします");
                    Deactivate();
                    _ = _bridge.SendSteamVrStatusAsync(false, _shutdownToken);
                    break;
                }
            }
        }

        Deactivate();
    }

    private bool _warnedVrInitFailure;

    private bool TryActivate()
    {
        try
        {
            var error = EVRInitError.None;
            var system = OpenVR.Init(ref error, EVRApplicationType.VRApplication_Background);
            if (error != EVRInitError.None || system == null)
            {
                // SteamVR未起動時は常に失敗し続けるため、ログを1回だけ出す
                // （3秒間隔で無限に出続けるとログファイルが埋もれるため）。
                if (!_warnedVrInitFailure)
                {
                    Console.WriteLine($"[steamvr] SteamVRに接続できません（{error}）。SteamVR起動を待機します。");
                    _warnedVrInitFailure = true;
                }
                return false;
            }

            _warnedVrInitFailure = false;

            var input = OpenVR.Input;
            if (input == null || OpenVR.Overlay == null)
            {
                OpenVR.Shutdown();
                return false;
            }

            // アプリを .vrmanifest でSteamVRに登録する。これが無いと、SteamVRの
            // コントローラーバインディング画面がこのプロセスを「登録済みアプリ」として
            // 認識できず、表示名が実行ファイル名にフォールバックする上、
            // バインディングの保存先が定まらず編集・保存ができない
            // （実機確認で判明。参照実装: OyasumiVR src-core/src/openvr/mod.rs
            // が同様に起動時 add_application_manifest を呼んでいる）。
            var vrManifestPath = Path.Combine(AppDomain.CurrentDomain.BaseDirectory, "manifest.vrmanifest");
            var appManifestError = OpenVR.Applications.AddApplicationManifest(vrManifestPath, false);
            if (appManifestError != EVRApplicationError.None)
            {
                Console.Error.WriteLine($"[steamvr] vrmanifest の登録に失敗: {appManifestError} ({vrManifestPath})");
            }

            var manifestPath = Path.Combine(AppDomain.CurrentDomain.BaseDirectory, "input", "action_manifest.json");
            var manifestError = input.SetActionManifestPath(manifestPath);
            if (manifestError != EVRInputError.None)
            {
                Console.Error.WriteLine($"[steamvr] action manifest の設定に失敗: {manifestError} ({manifestPath})");
                OpenVR.Shutdown();
                return false;
            }

            ulong actionSetHandle = 0;
            input.GetActionSetHandle("/actions/hidden", ref actionSetHandle);
            ulong actionHandle = 0;
            input.GetActionHandle("/actions/hidden/in/OverlayInteract", ref actionHandle);
            if (actionSetHandle == 0 || actionHandle == 0)
            {
                Console.Error.WriteLine("[steamvr] action handle の取得に失敗しました");
                OpenVR.Shutdown();
                return false;
            }

            _actionSetHandle = actionSetHandle;
            _overlayInteractActionHandle = actionHandle;

            _d3D = new D3D11Context();
            _d3D.Initialize();
            _pointer = new OverlayPointer();
            _panel = new AvatarPanelOverlay(_d3D, _bridge.WsPort, _bridge.Token, _uiPort);
            _panel.OpenAsync().GetAwaiter().GetResult();
            _pointer.SetTarget(_panel);

            Console.WriteLine("[steamvr] OverlayManager を起動しました");
            return true;
        }
        catch (Exception ex)
        {
            Console.Error.WriteLine($"[steamvr] 初期化に失敗しました: {ex.Message}");
            return false;
        }
    }

    private void DetectOverlayInteract()
    {
        if (_actionSetHandle == 0) return;

        var activeSets = new[]
        {
            new VRActiveActionSet_t
            {
                ulActionSet = _actionSetHandle,
                ulRestrictedToDevice = OpenVR.k_ulInvalidInputValueHandle,
                ulSecondaryActionSet = 0,
                nPriority = 0,
                unPadding = 0,
            },
        };
        var updateError = OpenVR.Input.UpdateActionState(activeSets, (uint)Marshal.SizeOf<VRActiveActionSet_t>());
        if (updateError != EVRInputError.None)
        {
            LogInteractDiag($"UpdateActionState 失敗: {updateError}");
            return;
        }

        var actionData = new InputDigitalActionData_t();
        var dataError = OpenVR.Input.GetDigitalActionData(_overlayInteractActionHandle, ref actionData,
            (uint)Marshal.SizeOf<InputDigitalActionData_t>(), OpenVR.k_ulInvalidInputValueHandle);
        if (dataError != EVRInputError.None)
        {
            LogInteractDiag($"GetDigitalActionData 失敗: {dataError}");
            return;
        }
        if (!actionData.bChanged) return;

        var originInfo = new InputOriginInfo_t();
        var originError = OpenVR.Input.GetOriginTrackedDeviceInfo(actionData.activeOrigin, ref originInfo,
            (uint)Marshal.SizeOf<InputOriginInfo_t>());
        if (originError != EVRInputError.None)
        {
            LogInteractDiag($"GetOriginTrackedDeviceInfo 失敗: {originError}");
            return;
        }

        var role = OpenVR.System.GetControllerRoleForTrackedDeviceIndex(originInfo.trackedDeviceIndex);
        Console.WriteLine($"[steamvr] OverlayInteract 変化: role={role} bState={actionData.bState}");
        _pointer?.SetPressed(role, actionData.bState);
    }

    /// <summary>アクション取得系の失敗はバインド未設定など頻発しうるので1回目だけ出す。</summary>
    private bool _warnedInteractDiag;

    private void LogInteractDiag(string message)
    {
        if (_warnedInteractDiag) return;
        _warnedInteractDiag = true;
        Console.Error.WriteLine($"[steamvr] {message}");
    }

    private void RenderLoop()
    {
        var timer = new RefreshRateTimer();
        while (!_shutdownToken.IsCancellationRequested)
        {
            if (_active)
            {
                timer.TickStart();
                try
                {
                    // MainLoop 側の Deactivate()（OpenVR.Shutdown 呼び出し）と競合し、
                    // このスレッドが解放済みの OpenVR インターフェースに触れる可能性が
                    // ある（参照実装 OyasumiVR も同じ構造で同じレースを許容している）。
                    // クラッシュさせず次ティックへ回復させるだけの安全網。
                    _panel?.UpdateFrame();
                }
                catch (Exception ex)
                {
                    Console.Error.WriteLine($"[steamvr] フレーム更新中に例外: {ex.Message}");
                }

                timer.SleepUntilNextTick();
            }
            else
            {
                Thread.Sleep(100);
            }
        }
    }

    private void Deactivate()
    {
        if (!_active) return;
        _active = false;
        _pointer?.Dispose();
        _pointer = null;
        _panel?.Dispose();
        _panel = null;
        _d3D?.Dispose();
        _d3D = null;
        OpenVR.Shutdown();
        Console.WriteLine("[steamvr] OverlayManager を停止しました");
    }
}
