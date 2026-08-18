using SharpDX;
using SharpDX.Direct3D11;
using SharpDX.DXGI;
using Valve.VR;
using Device = SharpDX.Direct3D11.Device;

namespace AvatarSwitcher.OverlaySidecar.Rendering;

/// <summary>
/// CEFのオフスクリーン描画結果をアップロードするための D3D11 デバイスとテクスチャを管理する。
/// GPU共有テクスチャ（SharedTextureEnabled）は使わず、CPU側でピクセルをコピーする
/// 非アクセラレーション経路のみに絞っている（M1のスコープ判断。詳細はPR説明を参照）。
///
/// 参照実装: OyasumiVR src-overlay-sidecar/Managers/OvrDXDeviceHander.cs, Utils/Utils.cs (InitTexture2D)
/// </summary>
internal sealed class D3D11Context : IDisposable
{
    public Device Device { get; private set; } = null!;

    public void Initialize()
    {
        // SteamVRが描画に使っているアダプタと異なるアダプタで作ったテクスチャは
        // EVROverlayError.InvalidTexture で拒否されるため、必ず同じアダプタを選ぶ。
        var adapterIndex = -1;
        OpenVR.System.GetDXGIOutputInfo(ref adapterIndex);
        if (adapterIndex < 0) adapterIndex = 0;

        using var factory = new Factory1();
        Device = new Device(factory.GetAdapter(adapterIndex), DeviceCreationFlags.BgraSupport);
    }

    public void Dispose()
    {
        Device?.Dispose();
        Device = null!;
    }

    /// <summary>
    /// CPUから書き込み可能な正方形テクスチャを作成する。
    /// SteamVR起動直後などデバイスが一時的に不安定な場合に備え、数回リトライする。
    /// </summary>
    public async Task<Texture2D> CreateCpuWritableTextureAsync(uint resolution)
    {
        int[] retryDelaysMs = { 16, 100, 200, 500, 1000 };
        for (var attempt = 0; ; attempt++)
        {
            try
            {
                var description = new Texture2DDescription
                {
                    Width = (int)resolution,
                    Height = (int)resolution,
                    MipLevels = 1,
                    ArraySize = 1,
                    Format = Format.B8G8R8A8_UNorm,
                    SampleDescription = new SampleDescription(1, 0),
                    BindFlags = BindFlags.ShaderResource,
                    Usage = ResourceUsage.Dynamic,
                    CpuAccessFlags = CpuAccessFlags.Write,
                };
                return new Texture2D(Device, description);
            }
            catch (SharpDXException) when (attempt < retryDelaysMs.Length)
            {
                await Task.Delay(retryDelaysMs[attempt]);
            }
        }
    }
}
