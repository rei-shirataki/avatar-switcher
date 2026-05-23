import { Injectable, signal, computed } from '@angular/core';
import { TauriService } from './tauri.service';
import { VRChatAuthService } from './vrchat-auth.service';
import { VRCAvatar, AvatarFolder, AvatarOverride } from '../models/avatar.model';

/// アバター取得時の安全上限。1 ページ 100 件 × 100 ページ = 10,000 件まで。
/// 通常ユーザーは数十〜数百件なのでこの上限は実質的に到達しない。
/// API バグや想定外の挙動で無限ループに陥らないための保険。
const MAX_AVATAR_PAGES = 100;

/// アバター・お気に入り一覧のキャッシュ有効期限（ミリ秒）。
/// 長時間起動でも別端末からのアバター追加・削除を反映できるようにする。
const CACHE_TTL_MS = 10 * 60 * 1000; // 10 分

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
  private _loadedAt = 0;

  constructor(private tauri: TauriService, private auth: VRChatAuthService) {}

  /**
   * 初回のみ全データをフェッチし、2回目以降はキャッシュ済みの Promise を返す。
   * タブ切り替え時に不要な再フェッチを防ぐ。
   * 最終ロードから CACHE_TTL_MS 以上経過していたら自動で再フェッチする。
   * エラー時は _initPromise をリセットし、次回再試行できるようにする。
   *
   * `_loadedAt === 0` の間（= in-flight）は TTL を評価せず必ず同じ Promise を返す。
   * 評価すると Date.now()-0 が常に TTL 超過と判定され、二重ロードが発生する。
   */
  ensureLoaded(): Promise<void> {
    if (this._initPromise) {
      // _loadedAt が未設定（=0）の場合は in-flight。TTL 判定をスキップして共有 Promise を返す。
      const expired = this._loadedAt !== 0 && Date.now() - this._loadedAt > CACHE_TTL_MS;
      if (!expired) return this._initPromise;
      this._initPromise = null;
    }

    this._initPromise = Promise.all([
      this.loadAvatars(),
      this.loadFavorites(),
      this.loadFolders(),
      this.loadOverrides(),
    ]).then(() => {
      this._loadedAt = Date.now();
    }).catch((e) => {
      this._initPromise = null;
      throw e;
    });
    return this._initPromise;
  }

  /** データを強制的に再フェッチする（手動更新ボタン用）。 */
  async refresh(): Promise<void> {
    this._initPromise = null;
    this._loadedAt = 0;
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
      // MAX_AVATAR_PAGES を上限とする安全策。API バグで常に 100 件返り続けても
      // UI ハングを起こさず、警告ログを残してループを抜ける。
      for (let page = 0; page < MAX_AVATAR_PAGES; page++) {
        const items = await this.tauri.invoke<VRCAvatar[]>('vrchat_get_my_avatars', { offset });
        // ページ単位で即時反映し、最初の 100 件をすぐに表示する
        this._avatars.update(current => [...current, ...items]);
        if (items.length < 100) return;
        offset += 100;
      }
      console.warn(
        `loadAvatars: ${MAX_AVATAR_PAGES} ページ取得しても終端に達しませんでした。打ち切りました。`,
      );
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
      const [selected] = await Promise.all([
        this.tauri.invoke<VRCAvatar>('vrchat_select_avatar', { avatarId }),
        this.tauri.invoke('osc_change_avatar', { avatarId }).catch(() => {}),
      ]);
      // 装着結果でサイドバーの「現在のアバター」画像を即時更新する。
      // オーバーライドがあればそちらを優先（VRChat に未反映のカスタムサムネ）。
      const override = this._overrides()[avatarId];
      const imageUrl = override?.customThumbnail ?? selected?.thumbnailImageUrl ?? selected?.imageUrl;
      if (imageUrl) {
        this.auth.updateCurrentAvatarImage(imageUrl);
      }
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

  /**
   * 複数アバターを 1 回の Tauri 呼び出しでフォルダに追加する。
   * バックエンドの folders_add_avatars が排他ロック下で 1 トランザクションとして
   * 書き込むため、並列の addAvatarToFolder と違い RMW 競合で消失しない。
   */
  async addAvatarsToFolder(folderId: string, avatarIds: string[]): Promise<void> {
    if (avatarIds.length === 0) return;
    await this.tauri.invoke('folders_add_avatars', { folderId, avatarIds });
    this._folders.update(fs =>
      fs.map(f => {
        if (f.id !== folderId) return f;
        const merged = [...f.avatarIds];
        for (const id of avatarIds) {
          if (!merged.includes(id)) merged.push(id);
        }
        return { ...f, avatarIds: merged };
      })
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
