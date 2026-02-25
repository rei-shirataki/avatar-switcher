import { Injectable, signal, computed } from '@angular/core';
import { TauriService } from './tauri.service';
import { VRCAvatar, AvatarFolder, AvatarOverride } from '../models/avatar.model';

@Injectable({ providedIn: 'root' })
export class AvatarService {
  private readonly _avatars = signal<VRCAvatar[]>([]);
  private readonly _favorites = signal<VRCAvatar[]>([]);
  private readonly _folders = signal<AvatarFolder[]>([]);
  private readonly _loading = signal(false);
  private readonly _loadingFavorites = signal(false);
  private readonly _switching = signal<string | null>(null);
  private readonly _overrides = signal<Record<string, AvatarOverride>>({});

  readonly avatars = this._avatars.asReadonly();
  readonly favorites = this._favorites.asReadonly();
  readonly folders = this._folders.asReadonly();
  readonly isLoading = computed(() => this._loading() || this._loadingFavorites());
  readonly switching = this._switching.asReadonly();
  readonly overrides = this._overrides.asReadonly();

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

  // オーバーライド適用済みの全アバター（カード表示・検索に使用）
  readonly allAvatarsWithOverrides = computed(() => {
    const ov = this._overrides();
    if (Object.keys(ov).length === 0) return this.allAvatars();
    return this.allAvatars().map(a => {
      const o = ov[a.id];
      if (!o) return a;
      return {
        ...a,
        name: o.customName ?? a.name,
        thumbnailImageUrl: o.customThumbnail ?? a.thumbnailImageUrl,
      };
    });
  });

  private _initPromise: Promise<void> | null = null;

  constructor(private tauri: TauriService) {}

  /**
   * 初回のみ全データをフェッチし、2回目以降はキャッシュ済みの Promise を返す。
   * タブ切り替え時に不要な再フェッチを防ぐ。
   * エラー時は _initPromise をリセットし、次回再試行できるようにする。
   */
  ensureLoaded(): Promise<void> {
    if (this._initPromise) return this._initPromise;
    this._initPromise = Promise.all([
      this.loadAvatars(),
      this.loadFavorites(),
      this.loadFolders(),
      this.loadOverrides(),
    ]).then(() => {}).catch((e) => {
      this._initPromise = null;
      throw e;
    });
    return this._initPromise;
  }

  /** データを強制的に再フェッチする（手動更新ボタン用）。 */
  async refresh(): Promise<void> {
    this._initPromise = null;
    await this.ensureLoaded();
  }

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
    // 先にクリアして古いデータをリセット
    this._avatars.set([]);
    try {
      let offset = 0;
      while (true) {
        const page = await this.tauri.invoke<VRCAvatar[]>('vrchat_get_my_avatars', { offset });
        // ページ単位で即時反映し、最初の 100 件をすぐに表示する
        this._avatars.update(current => [...current, ...page]);
        if (page.length < 100) break;
        offset += 100;
      }
    } finally {
      this._loading.set(false);
    }
  }

  async loadFolders(): Promise<void> {
    const folders = await this.tauri.invoke<AvatarFolder[]>('folders_get_all');
    this._folders.set(folders);
  }

  async loadOverrides(): Promise<void> {
    const list = await this.tauri.invoke<AvatarOverride[]>('avatar_overrides_get_all');
    const map: Record<string, AvatarOverride> = {};
    for (const o of list) map[o.avatarId] = o;
    this._overrides.set(map);
  }

  /**
   * VRChat API でアバター名を更新する。
   * 成功後、ローカルの avatar リストとローカル名前オーバーライドを同期する。
   * 自分が作成したアバターのみ更新可能（他人のアバターは 403 エラー）。
   */
  async updateAvatarName(avatarId: string, name: string): Promise<VRCAvatar> {
    const updated = await this.tauri.invoke<VRCAvatar>('vrchat_update_avatar', { avatarId, name });
    // ローカルの avatar リストを更新
    this._avatars.update(list => list.map(a => a.id === avatarId ? updated : a));
    this._favorites.update(list => list.map(a => a.id === avatarId ? updated : a));
    // 名前のローカルオーバーライドをクリア（サムネイルオーバーライドは保持）
    await this._clearNameOverride(avatarId);
    return updated;
  }

  /**
   * アバター画像を VRChat サーバーにアップロードし、アバターの imageUrl を更新する。
   * 成功後、ローカルシグナルを同期し、ローカルサムネイルオーバーライドをクリアする。
   */
  async updateAvatarImage(avatarId: string, dataUrl: string): Promise<VRCAvatar> {
    const updated = await this.tauri.invoke<VRCAvatar>('vrchat_update_avatar_image', {
      avatarId,
      dataUrl,
    });
    this._avatars.update(list => list.map(a => a.id === avatarId ? updated : a));
    this._favorites.update(list => list.map(a => a.id === avatarId ? updated : a));
    await this._clearThumbnailOverride(avatarId);
    return updated;
  }

  private async _clearThumbnailOverride(avatarId: string): Promise<void> {
    const ov = this._overrides()[avatarId];
    if (!ov?.customThumbnail) return;
    await this.tauri.invoke('avatar_overrides_delete', { avatarId });
    this._overrides.update(o => { const next = { ...o }; delete next[avatarId]; return next; });
    // 名前オーバーライドが残る場合は再保存
    if (ov.customName) {
      const saved = await this.tauri.invoke<AvatarOverride>('avatar_overrides_set', {
        avatarId, customName: ov.customName, customThumbnail: null,
      });
      this._overrides.update(o => ({ ...o, [avatarId]: saved }));
    }
  }

  private async _clearNameOverride(avatarId: string): Promise<void> {
    const ov = this._overrides()[avatarId];
    if (!ov?.customName) return; // 名前オーバーライドなし
    // 既存オーバーライドを削除
    await this.tauri.invoke('avatar_overrides_delete', { avatarId });
    this._overrides.update(o => { const next = { ...o }; delete next[avatarId]; return next; });
    // サムネイルオーバーライドがあれば名前なしで再保存
    if (ov.customThumbnail) {
      const saved = await this.tauri.invoke<AvatarOverride>('avatar_overrides_set', {
        avatarId, customName: null, customThumbnail: ov.customThumbnail,
      });
      this._overrides.update(o => ({ ...o, [avatarId]: saved }));
    }
  }

  /**
   * アバターのオーバーライドを保存する。
   * customThumbnail に 'reset' を渡すとカスタムサムネイルを削除する。
   */
  async saveOverride(
    avatarId: string,
    customName: string | undefined,
    customThumbnail: string | undefined | 'reset',
  ): Promise<void> {
    const isThumbnailReset = customThumbnail === 'reset';
    const existing = this._overrides()[avatarId];

    if (isThumbnailReset) {
      // カスタムサムネイルを削除。名前オーバーライドが残る場合は再セット。
      await this.tauri.invoke('avatar_overrides_delete', { avatarId });
      this._overrides.update(ov => {
        const next = { ...ov };
        delete next[avatarId];
        return next;
      });
      const nameToKeep = customName !== undefined ? (customName || null) : (existing?.customName ?? null);
      if (nameToKeep) {
        const updated = await this.tauri.invoke<AvatarOverride>('avatar_overrides_set', {
          avatarId,
          customName: nameToKeep,
          customThumbnail: null,
        });
        this._overrides.update(ov => ({ ...ov, [avatarId]: updated }));
      }
    } else {
      const thumbnailArg = customThumbnail ?? existing?.customThumbnail ?? null;
      const nameArg = customName !== undefined ? (customName || null) : (existing?.customName ?? null);

      if (!nameArg && !thumbnailArg) {
        // 両方 null → オーバーライド不要なので削除
        await this.tauri.invoke('avatar_overrides_delete', { avatarId });
        this._overrides.update(ov => {
          const next = { ...ov };
          delete next[avatarId];
          return next;
        });
      } else {
        const updated = await this.tauri.invoke<AvatarOverride>('avatar_overrides_set', {
          avatarId,
          customName: nameArg,
          customThumbnail: thumbnailArg,
        });
        this._overrides.update(ov => ({ ...ov, [avatarId]: updated }));
      }
    }
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
