/** メインアプリとSteamVRオーバーレイUI(projects/overlay-ui)で共有するアイハイト定数。
 *  Rust側(src-tauri/src/osc/mod.rs の EYE_HEIGHT_MIN/MAX/SAFE_MIN/SAFE_MAX と
 *  osc/oscquery.rs の EYE_HEIGHT_DEFAULT)にも同値があるため、変更時は合わせて更新すること。 */

export const EYE_HEIGHT_DEFAULT = 1.6;
export const EYE_HEIGHT_MIN = 0.2;
export const EYE_HEIGHT_MAX = 5.0;

/** 設定で上限適用をオフにした場合でも残す最小限のセーフガード。
 *  VRChat の OSC 書き込みは EYE_HEIGHT_MIN/MAX の制限を受けないため、意図的に
 *  範囲外の身長を試したいユーザーのためにオフを許可するが、0 以下や桁外れの値は
 *  UI 表示崩壊・意図しない暴走の原因になるため下限だけは死守する。上限は
 *  「実質無制限」とみなせる程度に広く取っている。 */
export const EYE_HEIGHT_SAFE_MIN = 0.01;
export const EYE_HEIGHT_SAFE_MAX = 100;
