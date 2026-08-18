import { Injectable, inject, signal } from '@angular/core';
import { OverlayBridgeService } from './overlay-bridge.service';
import { VRCAvatar } from './core/models/avatar.model';

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
  private readonly _currentAvatarId = signal<string | null>(null);
  private readonly _switching = signal<string | null>(null);
  private readonly _loading = signal(false);

  readonly avatars = this._avatars.asReadonly();
  readonly currentAvatarId = this._currentAvatarId.asReadonly();
  readonly switching = this._switching.asReadonly();
  readonly loading = this._loading.asReadonly();

  constructor() {
    this.bridge.onAvatarChanged((id) => this._currentAvatarId.set(id));
  }

  async refresh(): Promise<void> {
    this._loading.set(true);
    try {
      this._avatars.set(await this.bridge.listAvatars());
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
