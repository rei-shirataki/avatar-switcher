// メインアプリ (src/app/core/models/avatar.model.ts) の VRCAvatar と同一形状。
// overlay-sidecar 側 (Rust bridge.rs) が同じフィールド名(camelCase)でJSONを返すため
// そのまま流用できる。overlay-ui はオーバーライド編集を行わないため、
// AvatarOverride はここでは持たない。
export interface VRCAvatar {
  id: string;
  name: string;
  authorName: string;
  authorId: string;
  thumbnailImageUrl: string;
  imageUrl: string;
  releaseStatus: string;
  tags: string[];
  version: number;
  createdAt: string;
  updatedAt: string;
}

// メインアプリの AvatarFolder と同一形状(#26)。overlay-uiは
// 表示・タブ切り替えのみで作成/編集/削除は行わないため、それ用のフィールドは
// 持たせていない(既に持っているものと同じなので割愛の必要も無い)。
export interface AvatarFolder {
  id: string;
  name: string;
  avatarIds: string[];
  color: string | null;
  order: number;
}
