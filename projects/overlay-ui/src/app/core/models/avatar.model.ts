// メインアプリ (src/app/core/models/avatar.model.ts) の VRCAvatar と同一形状。
// overlay-sidecar 側 (Rust bridge.rs) が同じフィールド名(camelCase)でJSONを返すため
// そのまま流用できる。overlay-ui はフォルダ/オーバーライド編集を行わないため、
// AvatarFolder / AvatarOverride はここでは持たない。
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
