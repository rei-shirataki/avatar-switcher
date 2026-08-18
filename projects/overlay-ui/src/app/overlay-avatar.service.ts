import { Injectable, inject, signal, computed } from '@angular/core';
import { OverlayBridgeService } from './overlay-bridge.service';
import { AvatarFolder, VRCAvatar } from './core/models/avatar.model';

/**
 * avatars-view.component.ts の SORT_OPTIONS と同じ並び。VRコントローラーでの
 * 操作性を考慮し、ドロップダウンではなく「タップで次のモードへ循環する単一ボタン」
 * で切り替える(#25)ため、選択中モードのラベル表示にそのまま使う。
 */
export const SORT_OPTIONS = [
  { value: 'updated-desc', label: '更新日時 (新しい順)' },
  { value: 'updated-asc', label: '更新日時 (古い順)' },
  { value: 'name-asc', label: '名前 (A → Z)' },
  { value: 'name-desc', label: '名前 (Z → A)' },
  { value: 'author-asc', label: '制作者名順' },
] as const;

/** avatars-view.component.ts の FAVORITES_TAB/UPLOADED_TAB と同じ特殊ID。folderIdの名前空間を
 * 共有するフォルダ選択(selectFolder)経由でお気に入り/アップロード済みタブも切り替える。 */
export const FAVORITES_TAB = '__favorites__';
export const UPLOADED_TAB = '__uploaded__';

/**
 * avatar.service.ts の簡略版。VRChatAuthService や tauri-plugin-store への
 * 依存を切り、WS経由でRust coreからアバター一覧取得/切替のみ行う。
 * フォルダ管理・検索・ローカルオーバーライド編集はM1範囲外(Rust側で適用済みの
 * 結果を受け取るのみ)。
 */
@Injectable({ providedIn: 'root' })
export class OverlayAvatarService {
  private readonly bridge = inject(OverlayBridgeService);

  private readonly _avatars = signal<VRCAvatar[]>([]);
  private readonly _favoriteIds = signal<ReadonlySet<string>>(new Set());
  private readonly _uploadedIds = signal<ReadonlySet<string>>(new Set());
  private readonly _currentAvatarId = signal<string | null>(null);
  private readonly _switching = signal<string | null>(null);
  private readonly _loading = signal(false);
  private readonly _sortMode = signal<string>('updated-desc');
  private readonly _folders = signal<AvatarFolder[]>([]);
  /** null = 「すべて」タブ。FAVORITES_TAB/UPLOADED_TABも同じ名前空間で扱う。
   * #26: 作成/編集/削除はメインアプリ側の役割で、ここは表示・切替のみ。 */
  private readonly _selectedFolderId = signal<string | null>(null);

  readonly avatars = this._avatars.asReadonly();
  readonly currentAvatarId = this._currentAvatarId.asReadonly();
  readonly switching = this._switching.asReadonly();
  readonly loading = this._loading.asReadonly();
  readonly sortMode = this._sortMode.asReadonly();
  readonly folders = this._folders.asReadonly();
  readonly selectedFolderId = this._selectedFolderId.asReadonly();

  readonly sortLabel = computed(
    () => SORT_OPTIONS.find((o) => o.value === this._sortMode())?.label ?? '',
  );

  /** avatars-view.component.ts::filteredAvatars と同じ分岐（FAVORITES_TAB/UPLOADED_TAB/フォルダ/すべて）。 */
  private readonly filteredAvatars = computed(() => {
    const folderId = this._selectedFolderId();
    if (folderId === FAVORITES_TAB) {
      const ids = this._favoriteIds();
      return this._avatars().filter((a) => ids.has(a.id));
    }
    if (folderId === UPLOADED_TAB) {
      const ids = this._uploadedIds();
      return this._avatars().filter((a) => ids.has(a.id));
    }
    if (folderId === null) return this._avatars();
    const folder = this._folders().find((f) => f.id === folderId);
    if (!folder) return this._avatars();
    const ids = new Set(folder.avatarIds);
    return this._avatars().filter((a) => ids.has(a.id));
  });

  readonly sortedAvatars = computed(() => this.sortAvatars(this.filteredAvatars(), this._sortMode()));

  constructor() {
    this.bridge.onAvatarChanged((id) => this._currentAvatarId.set(id));
  }

  selectFolder(folderId: string | null): void {
    this._selectedFolderId.set(folderId);
    this.persistUiState();
  }

  async refreshFolders(): Promise<void> {
    this._folders.set(await this.bridge.listFolders());
  }

  /**
   * 再起動後もソート/タブ選択を復元する。overlay-sidecarはCEFプロセスごとに
   * 新しいキャッシュディレクトリを使いlocalStorageが永続化できないため、
   * Rust側(storage::OverlayUiState)に保存された値を接続直後に取得する。
   * 未知のsortMode値（設定ファイル破損等）はデフォルトにフォールバックする。
   */
  async restoreUiState(): Promise<void> {
    try {
      const state = await this.bridge.getUiState();
      const sortMode = SORT_OPTIONS.some((o) => o.value === state.sortMode)
        ? state.sortMode
        : 'updated-desc';
      this._sortMode.set(sortMode);
      this._selectedFolderId.set(state.selectedFolderId);
    } catch (e) {
      console.warn('[overlay-avatar] UI状態の復元に失敗:', e);
    }
  }

  private persistUiState(): void {
    this.bridge.setUiState({ sortMode: this._sortMode(), selectedFolderId: this._selectedFolderId() });
  }

  /** タップのたびに次のソートモードへ循環する。avatars-view.component.tsのドロップダウン選択に相当。 */
  cycleSortMode(): void {
    const index = SORT_OPTIONS.findIndex((o) => o.value === this._sortMode());
    const next = SORT_OPTIONS[(index + 1) % SORT_OPTIONS.length];
    this._sortMode.set(next.value);
    this.persistUiState();
  }

  /** avatars-view.component.ts::sortAvatars と同一ロジック。 */
  private sortAvatars(avatars: VRCAvatar[], mode: string): VRCAvatar[] {
    const sorted = [...avatars];
    switch (mode) {
      case 'updated-asc':
        return sorted.sort((a, b) => a.updatedAt.localeCompare(b.updatedAt));
      case 'name-asc':
        return sorted.sort((a, b) => a.name.localeCompare(b.name));
      case 'name-desc':
        return sorted.sort((a, b) => b.name.localeCompare(a.name));
      case 'author-asc':
        return sorted.sort((a, b) => a.authorName.localeCompare(b.authorName));
      default: // updated-desc
        return sorted.sort((a, b) => b.updatedAt.localeCompare(a.updatedAt));
    }
  }

  async refresh(): Promise<void> {
    this._loading.set(true);
    try {
      const result = await this.bridge.listAvatars();
      this._avatars.set(result.avatars);
      this._favoriteIds.set(new Set(result.favoriteIds));
      this._uploadedIds.set(new Set(result.uploadedIds));
    } finally {
      this._loading.set(false);
    }
  }

  async switchAvatar(avatarId: string): Promise<void> {
    this._switching.set(avatarId);
    try {
      await this.bridge.selectAvatar(avatarId);
      this._currentAvatarId.set(avatarId);
    } finally {
      this._switching.set(null);
    }
  }
}
