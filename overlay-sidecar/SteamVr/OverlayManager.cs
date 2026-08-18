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
    private ulong _toggleActionSetHandle;
    private ulong _overlayInteractActionHandle;
    private ulong _openOverlayActionHandle;
    private ulong _scrollActionHandle;
    private DateTime _lastOpenOverlayPress = DateTime.MinValue;
    private static readonly TimeSpan OpenOverlayDoublePressWindow = TimeSpan.FromMilliseconds(400);
    private const float ScrollDeadzone = 0.15f;

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

            UpdateActionSets();
            DetectOverlayInteract();
            DetectOpenOverlayToggle();
            DetectScroll();

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
            input.GetActionSetHandle("/actions/main", ref actionSetHandle);
            ulong actionHandle = 0;
            input.GetActionHandle("/actions/main/in/OverlayInteract", ref actionHandle);
            if (actionSetHandle == 0 || actionHandle == 0)
            {
                Console.Error.WriteLine("[steamvr] action handle の取得に失敗しました");
                OpenVR.Shutdown();
                return false;
            }

            _actionSetHandle = actionSetHandle;
            _overlayInteractActionHandle = actionHandle;

            // #22: 右スティック(Vive Wandはトラックパッド)上下でのスクロール。トリガーと同じく
            // /actions/main(hidden)に属するアナログアクションで、ユーザーはバインド変更不可。
            // requirement: suggestedのため未バインドでもハンドル自体は取得できる想定。
            ulong scrollActionHandle = 0;
            input.GetActionHandle("/actions/main/in/Scroll", ref scrollActionHandle);
            if (scrollActionHandle == 0)
            {
                Console.Error.WriteLine("[steamvr] Scroll action handle の取得に失敗しました（スクロールは無効化されます）");
            }
            _scrollActionHandle = scrollActionHandle;

            // トリガーの/actions/mainとは別に、表示切替は/actions/toggleというusage: leftrightの
            // アクションセットに分けている（#21フォローアップ）。usageはアクション単位ではなく
            // アクションセット単位の設定のため、「トリガーはhidden(自動適用・編集不可)のまま、
            // 表示切替だけユーザーがSteamVRのバインディング編集画面で変更できるように」という
            // 要望を満たすには別セットに分割する必要があった。
            // OpenOverlayは requirement: suggested のため未バインドでもハンドル自体は
            // 取得できる想定。取れなければ機能を諦めてログのみ出し、致命的エラーとしては扱わない。
            ulong toggleActionSetHandle = 0;
            input.GetActionSetHandle("/actions/toggle", ref toggleActionSetHandle);
            ulong openOverlayActionHandle = 0;
            input.GetActionHandle("/actions/toggle/in/OpenOverlay", ref openOverlayActionHandle);
            if (toggleActionSetHandle == 0 || openOverlayActionHandle == 0)
            {
                Console.Error.WriteLine("[steamvr] OpenOverlay action handle の取得に失敗しました（表示切替は無効化されます）");
                toggleActionSetHandle = 0;
                openOverlayActionHandle = 0;
            }
            _toggleActionSetHandle = toggleActionSetHandle;
            _openOverlayActionHandle = openOverlayActionHandle;

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

    /// <summary>
    /// /actions/main（トリガー、hidden）と /actions/toggle（表示切替、leftright）の
    /// 両方を毎ティックアクティブ化する。トグル用セットが未解決（ハンドル取得失敗）の
    /// 場合は/actions/mainのみで呼び、致命的エラーにはしない。
    /// </summary>
    private void UpdateActionSets()
    {
        if (_actionSetHandle == 0) return;

        var count = _toggleActionSetHandle != 0 ? 2 : 1;
        var activeSets = new VRActiveActionSet_t[count];
        activeSets[0] = new VRActiveActionSet_t
        {
            ulActionSet = _actionSetHandle,
            ulRestrictedToDevice = OpenVR.k_ulInvalidInputValueHandle,
            ulSecondaryActionSet = 0,
            nPriority = 0,
            unPadding = 0,
        };
        if (_toggleActionSetHandle != 0)
        {
            activeSets[1] = new VRActiveActionSet_t
            {
                ulActionSet = _toggleActionSetHandle,
                ulRestrictedToDevice = OpenVR.k_ulInvalidInputValueHandle,
                ulSecondaryActionSet = 0,
                nPriority = 0,
                unPadding = 0,
            };
        }

        var updateError = OpenVR.Input.UpdateActionState(activeSets, (uint)Marshal.SizeOf<VRActiveActionSet_t>());
        if (updateError != EVRInputError.None)
        {
            LogInteractDiag($"UpdateActionState 失敗: {updateError}");
        }
    }

    private void DetectOverlayInteract()
    {
        if (_actionSetHandle == 0) return;

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

    /// <summary>
    /// #21: `/actions/toggle/in/OpenOverlay` の立ち上がりエッジを検知し、
    /// <see cref="OpenOverlayDoublePressWindow"/> 以内に2回目が来たらパネルの表示/非表示を切り替える。
    ///
    /// `/actions/toggle` は`/actions/main`と同じく常時アクティブなアクションセットで、
    /// 既定バインディングのA/Xボタン等はVRChat本体側の操作（ジャンプ等）にも使われている
    /// 可能性があるため、単押しでトグルすると誤爆しうる。ダブルプレス限定にすることで
    /// 意図的な操作とそうでないものを区別する。左右どちらのコントローラーでの押下も
    /// 区別せず合算して見る（`DetectOverlayInteract` と同じく `GetDigitalActionData` に
    /// `k_ulInvalidInputValueHandle` を渡し、両手の論理ORを取る）。
    /// </summary>
    private void DetectOpenOverlayToggle()
    {
        if (_openOverlayActionHandle == 0) return;

        var actionData = new InputDigitalActionData_t();
        var dataError = OpenVR.Input.GetDigitalActionData(_openOverlayActionHandle, ref actionData,
            (uint)Marshal.SizeOf<InputDigitalActionData_t>(), OpenVR.k_ulInvalidInputValueHandle);
        if (dataError != EVRInputError.None) return;
        if (!actionData.bChanged || !actionData.bState) return;

        var now = DateTime.UtcNow;
        if (now - _lastOpenOverlayPress <= OpenOverlayDoublePressWindow)
        {
            _lastOpenOverlayPress = DateTime.MinValue;
            var visible = !(_panel?.IsVisible ?? false);
            Console.WriteLine($"[steamvr] OpenOverlay ダブルプレス検知: visible={visible}");
            _panel?.SetVisible(visible);
        }
        else
        {
            _lastOpenOverlayPress = now;
        }
    }

    /// <summary>
    /// #22: `/actions/main/in/Scroll`（右スティック/トラックパッドのY軸）を読み取り、
    /// デッドゾーン超過分を<see cref="OverlayPointer.Scroll"/>経由でCEFのホイールイベントに変換する。
    /// 右手にのみバインドしているため、role固定でよい（左手分の判定は不要）。
    /// </summary>
    private bool _warnedScrollDiag;

    private void DetectScroll()
    {
        if (_scrollActionHandle == 0) return;

        var analogData = new InputAnalogActionData_t();
        var dataError = OpenVR.Input.GetAnalogActionData(_scrollActionHandle, ref analogData,
            (uint)Marshal.SizeOf<InputAnalogActionData_t>(), OpenVR.k_ulInvalidInputValueHandle);
        if (dataError != EVRInputError.None)
        {
            if (!_warnedScrollDiag)
            {
                _warnedScrollDiag = true;
                Console.Error.WriteLine($"[steamvr] Scroll GetAnalogActionData 失敗: {dataError}");
            }
            return;
        }
        if (!analogData.bActive)
        {
            if (!_warnedScrollDiag)
            {
                _warnedScrollDiag = true;
                Console.WriteLine("[steamvr] Scroll action が非アクティブです（バインド未反映の可能性。SteamVR再起動を試してください）");
            }
            return;
        }
        if (Math.Abs(analogData.y) < ScrollDeadzone) return;

        Console.WriteLine($"[steamvr] Scroll検知: y={analogData.y:F2}");
        _pointer?.Scroll(analogData.y);
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
                    //
                    // レイキャスト/ポインタ座標の更新(UpdateRaycast)もここに置く。
                    // 以前はMainLoop側の32ms固定ティックでDetectOverlayInteract()の
                    // 「後」に呼んでいたため、クリック検知時に使う座標が常に最大32ms
                    // (1ティック分)古い状態だった。パネルから距離が離れるほど同じ
                    // 角度のブレでもレイ先端の座標変化が大きくなるため、これが
                    // 「距離が離れると反応しない」「2回目でようやく成立する」の
                    // 原因だったと判明（参照実装OyasumiVRはOverlayPointer.Startを
                    // HMDリフレッシュレートの専用スレッドで回し、座標を常に新鮮に
                    // 保っている。同じ構成に合わせた）。
                    _pointer?.UpdateRaycast();
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
