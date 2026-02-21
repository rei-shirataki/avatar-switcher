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
