import { Component, OnInit, signal, computed, HostListener } from '@angular/core';
import { FormsModule } from '@angular/forms';
import { CommonModule } from '@angular/common';
import { AvatarService } from '../../core/services/avatar.service';
import { TauriService } from '../../core/services/tauri.service';
import { VRChatAuthService } from '../../core/services/vrchat-auth.service';
import { AvatarCardComponent, CardContextMenuEvent } from '../../shared/components/avatar-card/avatar-card.component';
import { IconComponent } from '../../shared/components/icon/icon.component';
import { VRCAvatar, AvatarFolder } from '../../core/models/avatar.model';

@Component({
  selector: 'app-avatars-view',
  standalone: true,
  imports: [FormsModule, CommonModule, AvatarCardComponent, IconComponent],
  templateUrl: './avatars-view.component.html',
  styleUrl: './avatars-view.component.scss',
})
export class AvatarsViewComponent implements OnInit {
  searchQuery = signal('');
  selectedFolderId = signal<string | null>(null);
  showNewFolderInput = signal(false);
  newFolderName = '';
  successMessage = signal<string | null>(null);
  // 詳細パネルの表示対象を ID で管理し、computed でオーバーライド適用済みデータを参照
  // → 保存後に detailAvatar が自動更新される
  private readonly _detailAvatarId = signal<string | null>(null);
  readonly detailAvatar = computed(() => {
    const id = this._detailAvatarId();
    if (!id) return null;
    return this.avatarService.allAvatarsWithOverrides().find(a => a.id === id) ?? null;
  });
  detailHighResLoaded = signal(false);
  contextMenuFolder = signal<AvatarFolder | null>(null);
  contextMenuPos = signal({ x: 0, y: 0 });
  cardMenuAvatar = signal<VRCAvatar | null>(null);
  cardMenuPos = signal({ x: 0, y: 0 });
  renamingFolderId = signal<string | null>(null);
  renamingFolderName = '';
  selectionMode = signal(false);
  selectedAvatarIds = signal<string[]>([]);
  bulkFolderId = '';

  /** 詳細表示中のアバターが自分のアップロードかどうか */
  readonly isDetailAvatarOwned = computed(() => {
    const av = this.detailAvatar();
    const uid = this.authService.user()?.id;
    return !!av && !!uid && av.authorId === uid;
  });

  // ── 編集モード ──────────────────────────────────────────
  detailEditMode = signal(false);
  detailEditName = '';
  /**
   * null    = 変更なし（既存値をそのまま使う）
   * 'reset' = カスタムサムネイルを削除してVRChatのURLに戻す
   * string  = 新しく選択された base64 data URL
   */
  detailEditThumbnail = signal<string | null>(null);
  detailEditSaving = signal(false);
  detailEditError = signal<string | null>(null);

  readonly FAVORITES_TAB = '__favorites__';
  readonly UPLOADED_TAB = '__uploaded__';

  readonly SORT_OPTIONS = [
    { value: 'updated-desc', label: '更新日時 (新しい順)' },
    { value: 'updated-asc',  label: '更新日時 (古い順)' },
    { value: 'name-asc',     label: '名前 (A → Z)' },
    { value: 'name-desc',    label: '名前 (Z → A)' },
    { value: 'author-asc',   label: '制作者名順' },
  ] as const;

  sortMode = signal<string>('updated-desc');

  filteredAvatars = computed(() => {
    const query = this.searchQuery().toLowerCase();
    const folderId = this.selectedFolderId();
    const all = this.avatarService.allAvatarsWithOverrides();
    let avatars: VRCAvatar[];

    if (folderId === this.FAVORITES_TAB) {
      const favIds = new Set(this.avatarService.favorites().map(a => a.id));
      avatars = all.filter(a => favIds.has(a.id));
    } else if (folderId === this.UPLOADED_TAB) {
      const upIds = new Set(this.avatarService.avatars().map(a => a.id));
      avatars = all.filter(a => upIds.has(a.id));
    } else if (folderId) {
      const folder = this.avatarService.folders().find(f => f.id === folderId);
      avatars = folder
        ? all.filter(a => folder.avatarIds.includes(a.id))
        : [];
    } else {
      avatars = all;
    }

    if (query) {
      avatars = avatars.filter(a =>
        a.name.toLowerCase().includes(query) ||
        a.authorName.toLowerCase().includes(query)
      );
    }

    return this.sortAvatars(avatars, this.sortMode());
  });

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

  constructor(
    public avatarService: AvatarService,
    public authService: VRChatAuthService,
    private tauri: TauriService,
  ) {}

  async ngOnInit() {
    await Promise.all([
      this.avatarService.loadAvatars(),
      this.avatarService.loadFavorites(),
      this.avatarService.loadFolders(),
      this.avatarService.loadOverrides(),
    ]);
  }

  async onSwitch(avatarId: string) {
    try {
      await this.avatarService.switchAvatar(avatarId);
      this.showSuccess('アバターを切り替えました');
    } catch (e: unknown) {
      console.error(e);
    }
  }

  async createFolder() {
    if (!this.newFolderName.trim()) return;
    await this.avatarService.createFolder(this.newFolderName.trim());
    this.newFolderName = '';
    this.showNewFolderInput.set(false);
  }

  async onAddToFolder(avatarId: string, folderId: string) {
    await this.avatarService.addAvatarToFolder(folderId, avatarId);
  }

  async onRemoveFromFolder(avatarId: string, folderId: string) {
    await this.avatarService.removeAvatarFromFolder(folderId, avatarId);
  }

  async deleteFolder(folder: AvatarFolder) {
    await this.avatarService.deleteFolder(folder.id);
    if (this.selectedFolderId() === folder.id) {
      this.selectedFolderId.set(null);
    }
  }

  onFolderContextMenu(event: MouseEvent, folder: AvatarFolder) {
    event.preventDefault();
    event.stopPropagation();
    this.renamingFolderId.set(null);
    this.cardMenuAvatar.set(null);
    this.contextMenuFolder.set(folder);
    this.contextMenuPos.set(this.clampMenuPos(event.clientX, event.clientY, 160, 100));
  }

  onAvatarCardContextMenu(e: CardContextMenuEvent) {
    this.contextMenuFolder.set(null);
    this.cardMenuAvatar.set(e.avatar);
    this.cardMenuPos.set(this.clampMenuPos(e.x, e.y, 200, 240));
  }

  private clampMenuPos(x: number, y: number, estW: number, estH: number): { x: number; y: number } {
    const margin = 8;
    return {
      x: Math.min(x, window.innerWidth  - estW - margin),
      y: Math.min(y, window.innerHeight - estH - margin),
    };
  }

  async toggleCardMenuFolder(folderId: string) {
    const avatar = this.cardMenuAvatar();
    if (!avatar) return;
    const folder = this.avatarService.folders().find(f => f.id === folderId);
    if (!folder) return;
    if (folder.avatarIds.includes(avatar.id)) {
      await this.avatarService.removeAvatarFromFolder(folderId, avatar.id);
    } else {
      await this.avatarService.addAvatarToFolder(folderId, avatar.id);
    }
    this.cardMenuAvatar.set(null);
  }

  @HostListener('document:click', ['$event'])
  onDocumentClick(event: MouseEvent) {
    const target = event.target as HTMLElement;
    if (!target.closest('.context-menu')) {
      this.contextMenuFolder.set(null);
      this.cardMenuAvatar.set(null);
    }
  }

  @HostListener('document:contextmenu', ['$event'])
  onDocumentContextMenu(event: MouseEvent) {
    const target = event.target as HTMLElement;
    if (!target.closest('.folder-tab-wrap')) {
      this.contextMenuFolder.set(null);
    }
    // card context menu: closed if event reached document (i.e. not from avatar-card which stopPropagates)
    this.cardMenuAvatar.set(null);
  }

  startRenameFolder(folder: AvatarFolder) {
    this.contextMenuFolder.set(null);
    this.renamingFolderName = folder.name;
    this.renamingFolderId.set(folder.id);
  }

  async confirmRenameFolder() {
    const id = this.renamingFolderId();
    if (!id || !this.renamingFolderName.trim()) return;
    await this.avatarService.renameFolder(id, this.renamingFolderName.trim());
    this.renamingFolderId.set(null);
  }

  cancelRenameFolder() {
    this.renamingFolderId.set(null);
  }

  toggleSelectionMode() {
    this.selectionMode.update(v => !v);
    if (!this.selectionMode()) {
      this.selectedAvatarIds.set([]);
      this.bulkFolderId = '';
    }
  }

  toggleAvatarSelection(avatarId: string) {
    this.selectedAvatarIds.update(ids =>
      ids.includes(avatarId) ? ids.filter(id => id !== avatarId) : [...ids, avatarId]
    );
  }

  selectAll() {
    this.selectedAvatarIds.set(this.filteredAvatars().map(a => a.id));
  }

  async bulkAddToFolder() {
    if (!this.bulkFolderId || this.selectedAvatarIds().length === 0) return;
    const ids = this.selectedAvatarIds();
    for (const id of ids) {
      await this.avatarService.addAvatarToFolder(this.bulkFolderId, id);
    }
    this.showSuccess(`${ids.length} 件のアバターをフォルダに追加しました`);
    this.selectedAvatarIds.set([]);
    this.bulkFolderId = '';
  }

  private showSuccess(msg: string) {
    this.successMessage.set(msg);
    setTimeout(() => this.successMessage.set(null), 2500);
  }

  openDetailPanel(avatar: VRCAvatar) {
    this._detailAvatarId.set(avatar.id);
    this.detailHighResLoaded.set(false);
    this.detailEditMode.set(false);
    this.detailEditThumbnail.set(null);
  }

  closeDetailPanel() {
    this._detailAvatarId.set(null);
    this.detailEditMode.set(false);
    this.detailEditThumbnail.set(null);
  }

  formatDate(iso: string): string {
    if (!iso) return '—';
    const d = new Date(iso);
    return isNaN(d.getTime()) ? '—' : d.toLocaleDateString('ja-JP', {
      year: 'numeric', month: '2-digit', day: '2-digit',
    });
  }

  displayableTags(tags: string[]): string[] {
    return tags.filter(t =>
      t.length > 0 &&
      !t.startsWith('author_tag_') &&
      !t.startsWith('content_type_') &&
      !t.startsWith('debug_') &&
      !t.startsWith('system_')
    );
  }

  // ── 編集モード ──────────────────────────────────────────

  startEdit() {
    const av = this.detailAvatar();
    if (!av) return;
    // 編集の起点は VRChat 上の実際の名前（ローカルオーバーライドではなく）
    // allAvatarsWithOverrides 経由で来る av.name はオーバーライド後の値なので、
    // 元の VRChat 名は avatarService.avatars() か favorites() から取得する
    const originalAvatar =
      this.avatarService.avatars().find(a => a.id === av.id) ??
      this.avatarService.favorites().find(a => a.id === av.id);
    this.detailEditName = originalAvatar?.name ?? av.name;
    this.detailEditThumbnail.set(null);
    this.detailEditError.set(null);
    this.detailEditMode.set(true);
  }

  cancelEdit() {
    this.detailEditMode.set(false);
    this.detailEditThumbnail.set(null);
    this.detailEditError.set(null);
  }

  onThumbnailFileChange(event: Event) {
    const input = event.target as HTMLInputElement;
    const file = input.files?.[0];
    if (!file) return;
    const reader = new FileReader();
    reader.onload = () => {
      this.detailEditThumbnail.set(reader.result as string);
    };
    reader.readAsDataURL(file);
    // 同じファイルを再選択できるようにリセット
    input.value = '';
  }

  resetThumbnail() {
    this.detailEditThumbnail.set('reset');
  }

  /** 編集プレビュー用に表示すべきサムネイル URL を返す */
  get editPreviewThumb(): string {
    const av = this.detailAvatar()!;
    const newThumb = this.detailEditThumbnail();
    if (newThumb === 'reset') return av.imageUrl ?? av.thumbnailImageUrl;
    if (newThumb) return newThumb;
    // null = 変更なし → 既存オーバーライドまたは VRChat 高解像度 URL
    const ov = this.avatarService.overrides()[av.id];
    return ov?.customThumbnail ?? av.imageUrl ?? av.thumbnailImageUrl;
  }

  /** 現在カスタムサムネイルが設定されているか（リセット前提の表示に使用） */
  get hasCustomThumb(): boolean {
    const av = this.detailAvatar();
    if (!av) return false;
    const newThumb = this.detailEditThumbnail();
    if (newThumb === 'reset') return false;
    if (newThumb) return true;
    return !!(this.avatarService.overrides()[av.id]?.customThumbnail);
  }

  async saveEdit() {
    const av = this.detailAvatar();
    if (!av || this.detailEditSaving()) return;
    this.detailEditSaving.set(true);
    this.detailEditError.set(null);
    try {
      const newName = this.detailEditName.trim();
      const thumbnail = this.detailEditThumbnail() as string | null;

      // VRChat 上の元の名前を取得して変更有無を判定
      const originalAvatar =
        this.avatarService.avatars().find(a => a.id === av.id) ??
        this.avatarService.favorites().find(a => a.id === av.id);
      const vrcName = originalAvatar?.name ?? av.name;

      if (newName && newName !== vrcName) {
        // 名前変更 → VRChat API で反映（自分が作成したアバターのみ可能）
        await this.avatarService.updateAvatarName(av.id, newName);
      }

      // サムネイル変更 → VRChat サーバーにアップロード（自分のアバターのみこのパスに来る）
      if (thumbnail !== null) {
        if (thumbnail !== 'reset') {
          await this.avatarService.updateAvatarImage(av.id, thumbnail);
        } else {
          // 'reset' = ローカルオーバーライドをクリア
          await this.avatarService.saveOverride(av.id, undefined, 'reset');
        }
      }

      this.detailEditMode.set(false);
      this.detailEditThumbnail.set(null);
      this.showSuccess('変更を保存しました');
    } catch (e: unknown) {
      const msg = e instanceof Error ? e.message : String(e);
      this.detailEditError.set(msg || '保存に失敗しました');
    } finally {
      this.detailEditSaving.set(false);
    }
  }
}
