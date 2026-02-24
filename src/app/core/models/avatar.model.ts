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

export interface AvatarFolder {
  id: string;
  name: string;
  avatarIds: string[];
  color: string | null;
  order: number;
}

/** ローカル保存のアバター表示オーバーライド（名前・サムネイル） */
export interface AvatarOverride {
  avatarId: string;
  customName?: string;
  /** base64 data URL */
  customThumbnail?: string;
}
