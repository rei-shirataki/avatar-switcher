namespace AvatarSwitcher.OverlaySidecar.SteamVr;

/// <summary>
/// パネルオーバーレイの配置方式（#28）。Rust側の設定(`storage::OverlayPlacementMode`)から
/// `--placement-mode` CLI引数経由で渡される。実行中の切替はサポートしない
/// （設定変更はサイドカー再起動後に反映）。
/// </summary>
internal enum PlacementMode
{
    /// <summary>コントローラーに常時追従（腕時計のように）。既定値。</summary>
    Hand,

    /// <summary>表示ON時に手元付近へ出現し、以降は空間に静止（OyasumiVR方式）。</summary>
    Space,
}
