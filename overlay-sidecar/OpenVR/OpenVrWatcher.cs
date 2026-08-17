using System;
using System.Threading;
using System.Threading.Tasks;
using AvatarSwitcher.OverlaySidecar.Bridge;
using Valve.VR;

// 物理フォルダは OpenVR/ に置くが、Valve.VR 名前空間のクラス `OpenVR` との
// 衝突を避けるため C# の名前空間はあえて SteamVr にしている。
namespace AvatarSwitcher.OverlaySidecar.SteamVr;

/// <summary>
/// SteamVR (OpenVR) の起動を検知し続ける。
///
/// M0 時点では `VR_Init` の成否を一定間隔でリトライし、状態が変化した
/// タイミングだけ Rust コアへ `sidecar.steamvr-status` を通知する。
/// ダッシュボードオーバーレイの生成 (`CreateDashboardOverlay`) と
/// `PollNextOverlayEvent` によるイベントループは M1 で追加する。
/// </summary>
internal sealed class OpenVrWatcher
{
    private static readonly TimeSpan PollInterval = TimeSpan.FromSeconds(3);

    private readonly WsBridgeClient _bridge;

    public OpenVrWatcher(WsBridgeClient bridge)
    {
        _bridge = bridge;
    }

    public async Task RunAsync(CancellationToken ct)
    {
        var running = false;
        while (!ct.IsCancellationRequested)
        {
            if (!running)
            {
                running = TryInit();
                if (running)
                {
                    await _bridge.SendSteamVrStatusAsync(true, ct);
                }
            }
            else if (!OpenVR.IsHmdPresent())
            {
                // SteamVR 側が先に終了した。M1 で PollNextOverlayEvent の
                // VREvent_Quit 検知に置き換えるまでの簡易チェック。
                OpenVR.Shutdown();
                running = false;
                await _bridge.SendSteamVrStatusAsync(false, ct);
            }

            await Task.Delay(PollInterval, ct);
        }
    }

    private static bool TryInit()
    {
        try
        {
            var error = EVRInitError.None;
            var system = OpenVR.Init(ref error, EVRApplicationType.VRApplication_Overlay);
            return error == EVRInitError.None && system != null;
        }
        catch (Exception e)
        {
            // openvr_api.dll 未検出 (DllNotFoundException) を含む、あらゆる
            // 初期化失敗はここで握りつぶしリトライに委ねる。
            // SteamVR未インストール環境でもサイドカー自体はクラッシュせず待機し続ける。
            Console.Error.WriteLine($"[steamvr] VR_Init failed: {e.Message}");
            return false;
        }
    }
}
