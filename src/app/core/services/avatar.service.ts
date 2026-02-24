import { Injectable, signal, computed } from '@angular/core';
import { TauriService } from './tauri.service';
import { VRCAvatar, AvatarFolder } from '../models/avatar.model';

@Injectable({ providedIn: 'root' })
export class AvatarService {
  private readonly _avatars = signal<VRCAvatar[]>([]);
  private readonly _favorites = signal<VRCAvatar[]>([]);
  private readonly _folders = signal<AvatarFolder[]>([]);
  private readonly _loading = signal(false);
  private readonly _loadingFavorites = signal(false);
  private readonly _switching = signal<string | null>(null);

  readonly avatars = this._avatars.asReadonly();
  readonly favorites = this._favorites.asReadonly();
  readonly folders = this._folders.asReadonly();
  readonly isLoading = computed(() => this._loading() || this._loadingFavorites());
  readonly switching = this._switching.asReadonly();

  // Uploaded + favorites merged, deduplicated by ID so that favorited-only
  // avatars are reachable from "すべて" and folder lookups.
  readonly allAvatars = computed(() => {
    const map = new Map<string, VRCAvatar>();
    for (const a of this._avatars()) map.set(a.id, a);
    for (const a of this._favorites()) {
      if (!map.has(a.id)) map.set(a.id, a);
    }
    return Array.from(map.values());
  });

  constructor(private tauri: TauriService) {}

  async loadFavorites(): Promise<void> {
    this._loadingFavorites.set(true);
    try {
      const favs = await this.tauri.invoke<VRCAvatar[]>('vrchat_get_favorite_avatars');
      this._favorites.set(favs);
    } finally {
      this._loadingFavorites.set(false);
    }
  }

  async loadAvatars(): Promise<void> {
    this._loading.set(true);
    try {
      const results: VRCAvatar[] = [];
      let offset = 0;
      while (true) {
        const page = await this.tauri.invoke<VRCAvatar[]>('vrchat_get_my_avatars', { offset });
        results.push(...page);
        if (page.length < 100) break;
        offset += 100;
      }
      this._avatars.set(results);
    } finally {
      this._loading.set(false);
    }
  }

  async loadFolders(): Promise<void> {
    const folders = await this.tauri.invoke<AvatarFolder[]>('folders_get_all');
    this._folders.set(folders);
  }

  async switchAvatar(avatarId: string): Promise<void> {
    this._switching.set(avatarId);
    try {
      // REST + OSC を並行送信 (OSC は localhost UDP なので即時、REST は ~1秒かかる)
      await Promise.all([
        this.tauri.invoke('vrchat_select_avatar', { avatarId }),
        this.tauri.invoke('osc_change_avatar', { avatarId }).catch(() => {}),
      ]);
    } finally {
      this._switching.set(null);
    }
  }

  async createFolder(name: string): Promise<AvatarFolder> {
    const folder = await this.tauri.invoke<AvatarFolder>('folders_create', { name });
    this._folders.update(fs => [...fs, folder]);
    return folder;
  }

  async renameFolder(folderId: string, name: string): Promise<void> {
    const folder = this._folders().find(f => f.id === folderId);
    if (!folder) return;
    const updated = { ...folder, name };
    await this.tauri.invoke('folders_update', { folder: updated });
    this._folders.update(fs => fs.map(f => f.id === folderId ? updated : f));
  }

  async deleteFolder(folderId: string): Promise<void> {
    await this.tauri.invoke('folders_delete', { folderId });
    this._folders.update(fs => fs.filter(f => f.id !== folderId));
  }

  async addAvatarToFolder(folderId: string, avatarId: string): Promise<void> {
    await this.tauri.invoke('folders_add_avatar', { folderId, avatarId });
    this._folders.update(fs =>
      fs.map(f => f.id === folderId && !f.avatarIds.includes(avatarId)
        ? { ...f, avatarIds: [...f.avatarIds, avatarId] }
        : f
      )
    );
  }

  async removeAvatarFromFolder(folderId: string, avatarId: string): Promise<void> {
    await this.tauri.invoke('folders_remove_avatar', { folderId, avatarId });
    this._folders.update(fs =>
      fs.map(f => f.id === folderId
        ? { ...f, avatarIds: f.avatarIds.filter(id => id !== avatarId) }
        : f
      )
    );
  }
}
