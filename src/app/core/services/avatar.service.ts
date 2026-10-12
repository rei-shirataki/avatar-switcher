import { DestroyRef, Injectable, computed, effect, inject, signal, untracked } from '@angular/core';
import { listen, UnlistenFn } from '@tauri-apps/api/event';
import { TauriService } from './tauri.service';
import { VRChatAuthService } from './vrchat-auth.service';
import { VRCAvatar, AvatarFolder, AvatarOverride } from '../models/avatar.model';

/// アバター・お気に入り一覧のメモリキャッシュ有効期限（ミリ秒）。
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
  /** 直近で初期化が完了した時刻 (ms)。0 は「まだ一度も完了していない」。 */
  private _loadedAt = 0;
  /** 初期化が in-flight 中かどうか。TTL 判定が in-flight 中に走ると
   *  Date.now() - 0 が常に TTL 超過扱いになり二重ロードしてしまうので、
   *  明示フラグでガードする。 */
  private _inFlight = false;
  /** ログアウト・アカウント切替のたびに増える世代。取得中に変わったら結果を捨てる。 */
  private _generation = 0;
  /** 一覧取得の通し番号。より新しい取得が始まっていれば古い結果で上書きしない。 */
  private _avatarsReq = 0;
  private _favoritesReq = 0;
  /** 直近に観測したログインユーザーID（アカウント切替の検知用）。 */
  private _userId: string | null = null;
  private readonly unlisteners: UnlistenFn[] = [];
  /** onDestroy 済みか。listen() の Promise 解決が onDestroy より後になった場合、
   *  unlisteners への push ではなく即座に unlisten してリークを防ぐために参照する。 */
  private destroyed = false;

  constructor(private tauri: TauriService, private auth: VRChatAuthService) {
    // VRChat 側でアバターが切り替わったら（クイックメニュー操作・別端末経由など）
    // サイドバーの装着中アバター画像を即時更新する。avatars / favorites リストに
    // 未ロードの ID が来た場合は何もしない（次回 refresh で同期される）。
    this.registerListener(
      listen<string>('osc:avatar-change', e => this.applyExternalAvatarChange(e.payload)),
      err => console.warn('osc:avatar-change の購読に失敗しました:', err),
    );

    // ログアウト・別アカウントでのログイン時に、前のアカウントのアバター一覧を
    // メモリに残さない（10分の TTL 内だと次のアカウントにそのまま表示されていた）。
    effect(() => {
      const id = this.auth.user()?.id ?? null;
      untracked(() => {
        if (this._userId !== null && this._userId !== id) this.resetForAccountChange();
        this._userId = id;
      });
    });

    inject(DestroyRef).onDestroy(() => {
      this.destroyed = true;
      for (const un of this.unlisteners.splice(0)) un();
    });
  }

  /** listen() の Promise を登録し、解決した unlisten を保持する。
   *  onDestroy が Promise 解決より先に発火した場合（HMR 等）は push せず即座に
   *  unlisten を呼び、リスナーリークを防ぐ。 */
  private registerListener(pending: Promise<UnlistenFn>, onError?: (err: unknown) => void): void {
    pending
      .then(un => {
        if (this.destroyed) {
          un();
          return;
        }
        this.unlisteners.push(un);
      })
      .catch(err => onError?.(err));
  }

  private applyExternalAvatarChange(avatarId: unknown): void {
    if (typeof avatarId !== 'string' || !avatarId) return;
    const found = this.allAvatars().find(a => a.id === avatarId);
    if (!found) return;
    const override = this._overrides()[avatarId];
    // Rust 側は欠けた URL を null ではなく空文字列で返すため、?? ではなく || で次へ回す。
    const imageUrl = override?.customThumbnail || found.thumbnailImageUrl || found.imageUrl;
    if (imageUrl) {
      this.auth.updateCurrentAvatarImage(imageUrl);
    }
  }

  /**
   * 初回起動時はディスクキャッシュ（SWR）→ fresh フェッチで即時表示を実現する。
   * セッション内 TTL 経過後の自動再フェッチではディスクキャッシュ表示はスキップし、
   * fresh フェッチのみ実行する（メモリキャッシュが既にあるので二度手間を避ける）。
   *
   * エラー時は _initPromise をリセットして次回再試行可能にする。
   */
  ensureLoaded(): Promise<void> {
    if (this._initPromise) {
      // in-flight 中は TTL 判定をスキップ（二重ロード回避）。
      if (this._inFlight) return this._initPromise;
      const expired = Date.now() - this._loadedAt > CACHE_TTL_MS;
      if (!expired) return this._initPromise;
      this._initPromise = null;
    }

    // セッション内で既に一度ロード済み（=_loadedAt > 0）なら fresh のみ。
    // 初回（_loadedAt === 0）は SWR（ディスクキャッシュ即読み → 並列再フェッチ）。
    const isFirstLoad = this._loadedAt === 0;
    return this.startLoad(() => (isFirstLoad ? this._initWithSwr() : this._initFresh()));
  }

  /**
   * ロード処理を開始して `_initPromise` に登録する。完了・失敗時の状態更新は、
   * その時点でも自分が `_initPromise` である場合に限る。refresh() やアカウント切替で
   * 置き換えられた古いロードが後から失敗しても、新しいロードの状態を壊さない。
   */
  private startLoad(loader: () => Promise<unknown>): Promise<void> {
    this._inFlight = true;
    const p: Promise<void> = loader()
      .then(() => {
        if (this._initPromise === p) this._loadedAt = Date.now();
      })
      .catch((e) => {
        if (this._initPromise === p) this._initPromise = null;
        throw e;
      })
      .finally(() => {
        if (this._initPromise === p || this._initPromise === null) this._inFlight = false;
      });
    this._initPromise = p;
    return p;
  }

  /** アカウントが変わったときに、アカウント固有のメモリ状態を捨てる。 */
  private resetForAccountChange(): void {
    this._generation++;
    this._avatars.set([]);
    this._favorites.set([]);
    this._switching.set(null);
    this._initPromise = null;
    this._loadedAt = 0;
    this._inFlight = false;
  }

  /**
   * SWR で初期化する（初回起動・全リセット時）：
   * 1. ディスクキャッシュを即時ロード（あればスピナーを出さず即表示）。
   * 2. API への fresh フェッチを並列実行し、成功したらシグナルを更新。
   * フォルダ・オーバーライドはローカルのみなので素直に load する。
   */
  private async _initWithSwr(): Promise<void> {
    // ─ stale 部分（ディスクキャッシュ即読み） ─
    const gen = this._generation;
    const [cachedAv, cachedFav] = await Promise.all([
      this.tauri.invoke<VRCAvatar[]>('vrchat_get_cached_avatars').catch(() => [] as VRCAvatar[]),
      this.tauri.invoke<VRCAvatar[]>('vrchat_get_cached_favorites').catch(() => [] as VRCAvatar[]),
    ]);
    if (gen !== this._generation) return;
    // 自前 / お気に入りそれぞれ独立にキャッシュ有無を判定する。
    // 片方だけある状態（例: 初回お気に入り取得失敗）でも、ある方は stale 表示
    // して即時 UX を維持し、無い方だけスピナー付きで loadXxx() する。
    const hadAvCache = cachedAv.length > 0;
    const hadFavCache = cachedFav.length > 0;
    if (hadAvCache) this._avatars.set(cachedAv);
    if (hadFavCache) this._favorites.set(cachedFav);

    // ─ revalidate 部分（並列フェッチ） ─
    await Promise.all([
      hadAvCache ? this._fetchAvatarsBg() : this.loadAvatars(),
      hadFavCache ? this._fetchFavoritesBg() : this.loadFavorites(),
      this.loadFolders(),
      this.loadOverrides(),
    ]);
  }

  /**
   * TTL 経過時の自動再フェッチ用。ディスクキャッシュは読まず、API への fresh フェッチのみ。
   * メモリ上の avatars/favorites は表示維持され、フェッチ完了後に上書きされる。
   */
  private async _initFresh(): Promise<void> {
    await Promise.all([
      this._fetchAvatarsBg(),
      this._fetchFavoritesBg(),
      this.loadFolders(),
      this.loadOverrides(),
    ]);
  }

  /** バックグラウンド再フェッチ。失敗しても stale データを残し、ユーザー操作を妨げない。 */
  private async _fetchAvatarsBg(): Promise<void> {
    try {
      await this.fetchAvatars();
    } catch (e) {
      console.warn('自前アバターの再フェッチに失敗（キャッシュ表示を維持）:', e);
    }
  }

  private async _fetchFavoritesBg(): Promise<void> {
    try {
      await this.fetchFavorites();
    } catch (e) {
      console.warn('お気に入りの再フェッチに失敗（キャッシュ表示を維持）:', e);
    }
  }

  /**
   * データを強制的に再フェッチする（手動更新ボタン用）。
   * SWR の stale キャッシュ表示はバイパスし、loading フラグを立てて fresh フェッチする
   * （ユーザー操作の即時フィードバックのため）。
   */
  async refresh(): Promise<void> {
    this._loadedAt = 0;
    await this.startLoad(() =>
      Promise.all([this.loadAvatars(), this.loadFavorites(), this.loadFolders(), this.loadOverrides()]),
    );
  }

  /**
   * 一覧を取得して反映する。取得中により新しい取得が始まった場合や、
   * ログアウト・アカウント切替があった場合は、古い結果で上書きしない。
   */
  private async fetchAvatars(): Promise<void> {
    const req = ++this._avatarsReq;
    const gen = this._generation;
    const avatars = await this.tauri.invoke<VRCAvatar[]>('vrchat_get_my_avatars');
    if (req === this._avatarsReq && gen === this._generation) this._avatars.set(avatars);
  }

  private async fetchFavorites(): Promise<void> {
    const req = ++this._favoritesReq;
    const gen = this._generation;
    const favs = await this.tauri.invoke<VRCAvatar[]>('vrchat_get_favorite_avatars');
    if (req === this._favoritesReq && gen === this._generation) this._favorites.set(favs);
  }

  async loadFavorites(): Promise<void> {
    this._loadingFavorites.set(true);
    try {
      await this.fetchFavorites();
    } finally {
      this._loadingFavorites.set(false);
    }
  }

  /**
   * 自前アバターを全件取得する。
   * バックエンド側でページングを投機的並列化しているため、フロントは 1 回呼ぶだけ。
   */
  async loadAvatars(): Promise<void> {
    this._loading.set(true);
    try {
      await this.fetchAvatars();
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

  /**
   * オーバーライドを指定値で置換する。null は「そのフィールドをクリア」で、両方 null なら
   * エントリごと削除される。バックエンドで 1 回の原子的な更新になるため、delete→set の
   * 途中失敗で残すべき値まで失われることがない。
   */
  private async _replaceOverride(
    avatarId: string,
    customName: string | null,
    customThumbnail: string | null,
  ): Promise<void> {
    const saved = await this.tauri.invoke<AvatarOverride | null>('avatar_overrides_replace', {
      avatarId, customName, customThumbnail,
    });
    this._overrides.update(o => {
      const next = { ...o };
      if (saved) next[avatarId] = saved;
      else delete next[avatarId];
      return next;
    });
  }

  private async _clearThumbnailOverride(avatarId: string): Promise<void> {
    const ov = this._overrides()[avatarId];
    if (!ov?.customThumbnail) return;
    await this._replaceOverride(avatarId, ov.customName ?? null, null);
  }

  private async _clearNameOverride(avatarId: string): Promise<void> {
    const ov = this._overrides()[avatarId];
    if (!ov?.customName) return; // 名前オーバーライドなし
    await this._replaceOverride(avatarId, null, ov.customThumbnail ?? null);
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
    const existing = this._overrides()[avatarId];
    const nameArg = customName !== undefined ? (customName || null) : (existing?.customName ?? null);
    const thumbnailArg = customThumbnail === 'reset'
      ? null
      : (customThumbnail ?? existing?.customThumbnail ?? null);
    await this._replaceOverride(avatarId, nameArg, thumbnailArg);
  }

  async switchAvatar(avatarId: string): Promise<void> {
    this._switching.set(avatarId);
    try {
      // REST + OSC を並行送信 (OSC は localhost UDP なので即時、REST は ~1秒かかる)。
      // VRChatの装着APIは「更新後のアバター情報」ではなくユーザープロフィールを
      // 返す実装のため、レスポンスは使わず既にローカルに持っているアバター
      // データ（一覧取得時にキャッシュ済み）からサムネイルを引く。
      await Promise.all([
        this.tauri.invoke('vrchat_select_avatar', { avatarId }),
        this.tauri.invoke('osc_change_avatar', { avatarId }).catch(() => {}),
      ]);
      // 装着結果でサイドバーの「現在のアバター」画像を即時更新する。
      // オーバーライドがあればそちらを優先（VRChat に未反映のカスタムサムネ）。
      const override = this._overrides()[avatarId];
      const found = this.allAvatars().find(a => a.id === avatarId);
      const imageUrl = override?.customThumbnail || found?.thumbnailImageUrl || found?.imageUrl;
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
    const renamed = await this.tauri.invoke<AvatarFolder>('folders_rename', { folderId, name });
    // 名前だけを反映する。avatarIds まで置き換えると、並行して完了した追加・削除を巻き戻す。
    this._folders.update(fs => fs.map(f => (f.id === folderId ? { ...f, name: renamed.name } : f)));
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
