export interface VRCUser {
  id: string;
  displayName: string;
  currentAvatarImageUrl: string;
  userIcon: string;
  profilePicOverride: string;
  status: string;
  state: string;
}

export interface LoginResult {
  success: boolean;
  requires2fa: boolean;
  method: string | null;
  user: VRCUser | null;
  error: string | null;
}
