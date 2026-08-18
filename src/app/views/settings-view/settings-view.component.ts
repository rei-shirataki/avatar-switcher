import { Component, DestroyRef, OnInit, inject, signal } from '@angular/core';
import { getVersion } from '@tauri-apps/api/app';
import { VRChatAuthService } from '../../core/services/vrchat-auth.service';
import { TauriService } from '../../core/services/tauri.service';
import { Router } from '@angular/router';
import { VRCUser } from '../../core/models/auth.model';

interface OscStatus {
  port: number;
  vrchat_detected: boolean;
}

type OverlayPlacementMode = 'hand' | 'space';

interface OverlaySettings {
  placementMode: OverlayPlacementMode;
}

/// VRChat 起動状態のポーリング間隔。VRChat 後起動時に「未接続」表示のまま
/// 取り残されないよう、設定画面表示中だけ短めに polling する。
const OSC_POLL_INTERVAL_MS = 3000;

@Component({
  selector: 'app-settings-view',
  standalone: true,
  imports: [],
  template: `
    <div class="settings-view">
      <h2 class="page-title">設定</h2>

      @if (auth.user(); as user) {
        <div class="settings-section">
          <h3 class="section-title">アカウント</h3>
          <div class="account-card">
            @if (avatarUrl(user); as url) {
              <img
                class="account-avatar"
                [src]="url"
                alt=""
                (error)="onAvatarError($event)"
              />
            } @else {
              <div class="account-avatar account-avatar--placeholder"></div>
            }
            <div class="account-info">
              <div class="account-name">{{ user.displayName }}</div>
            </div>
            <button class="btn-logout" (click)="logout()">ログアウト</button>
          </div>
        </div>
      }

      <div class="settings-section">
        <h3 class="section-title">OSC</h3>
        <div class="settings-card">
          <div class="setting-row">
            <span class="setting-label">VRChat 接続</span>
            @if (oscStatus(); as s) {
              <span class="status-badge" [class.status-badge--on]="s.vrchat_detected">
                {{ s.vrchat_detected ? '接続中' : '未接続' }}
              </span>
            } @else {
              <span class="status-badge">確認中…</span>
            }
          </div>
          @if (oscStatus()?.vrchat_detected) {
            <div class="setting-row">
              <span class="setting-label">ポート</span>
              <span class="setting-value">{{ oscStatus()?.port }}</span>
            </div>
          }
        </div>
      </div>

      <div class="settings-section">
        <h3 class="section-title">SteamVRオーバーレイ</h3>
        <div class="settings-card">
          <div class="setting-row setting-row--column">
            <span class="setting-label">パネルの配置</span>
            <div class="placement-toggle">
              <button
                class="toggle-btn"
                [class.toggle-btn--on]="placementMode() === 'hand'"
                (click)="setPlacementMode('hand')"
              >
                手に追従
              </button>
              <button
                class="toggle-btn"
                [class.toggle-btn--on]="placementMode() === 'space'"
                (click)="setPlacementMode('space')"
              >
                空間に固定
              </button>
            </div>
            <p class="setting-hint">変更はオーバーレイの再起動後に反映されます（アプリの再起動が必要です）</p>
          </div>
        </div>
      </div>

      <div class="settings-section">
        <h3 class="section-title">このアプリについて</h3>
        <div class="settings-card">
          <div class="setting-row">
            <span class="setting-label">バージョン</span>
            <span class="setting-value">{{ version() }}</span>
          </div>
        </div>
      </div>
    </div>
  `,
  styleUrl: './settings-view.component.scss',
})
export class SettingsViewComponent implements OnInit {
  readonly version = signal<string>('...');
  readonly oscStatus = signal<OscStatus | null>(null);
  readonly placementMode = signal<OverlayPlacementMode>('hand');

  private pollHandle: ReturnType<typeof setInterval> | null = null;

  constructor(
    public auth: VRChatAuthService,
    private tauri: TauriService,
    private router: Router,
  ) {
    // 設定画面破棄時にポーリングを止める。
    inject(DestroyRef).onDestroy(() => {
      if (this.pollHandle !== null) {
        clearInterval(this.pollHandle);
        this.pollHandle = null;
      }
    });
  }

  async ngOnInit(): Promise<void> {
    // バージョン取得と OSC ステータス取得・オーバーレイ設定取得はそれぞれ独立。
    // 1つの失敗で他の表示が消えないよう個別 catch する。
    const [ver, status, overlaySettings] = await Promise.all([
      getVersion().catch(() => '?'),
      this.fetchOscStatus(),
      this.tauri.invoke<OverlaySettings>('overlay_settings_get').catch(() => null),
    ]);
    this.version.set(ver);
    this.oscStatus.set(status);
    if (overlaySettings) {
      this.placementMode.set(overlaySettings.placementMode);
    }

    // VRChat の後起動を検知するため、設定画面表示中だけ短い間隔で polling する。
    this.pollHandle = setInterval(async () => {
      const s = await this.fetchOscStatus();
      // 値が変化したときだけ signal を更新（不要な再レンダリングを避ける）。
      const cur = this.oscStatus();
      if (!cur || !s || cur.vrchat_detected !== s.vrchat_detected || cur.port !== s.port) {
        this.oscStatus.set(s);
      }
    }, OSC_POLL_INTERVAL_MS);
  }

  private async fetchOscStatus(): Promise<OscStatus | null> {
    return this.tauri.invoke<OscStatus>('osc_get_status').catch(() => null);
  }

  /**
   * オーバーレイパネルの配置方式を切り替える(#28)。overlay-sidecarはCLI引数
   * (`--placement-mode`)でしか受け取れないため、設定は保存するのみでその場では
   * 反映されない。次回サイドカー起動（=アプリ再起動）時に適用される。
   */
  async setPlacementMode(mode: OverlayPlacementMode): Promise<void> {
    if (this.placementMode() === mode) return;
    this.placementMode.set(mode);
    try {
      await this.tauri.invoke('overlay_settings_set', { settings: { placementMode: mode } });
    } catch (e) {
      console.error('[settings] オーバーレイ配置設定の保存に失敗:', e);
    }
  }

  avatarUrl(user: VRCUser): string {
    // profilePicOverride > userIcon > currentAvatarImageUrl の優先順で
    // フォールバックする（一部ユーザーは userIcon が未設定）。
    return user.profilePicOverride || user.userIcon || user.currentAvatarImageUrl || '';
  }

  /** 画像読み込み失敗時はプレースホルダー表示に切り替える（broken icon 防止）。 */
  onAvatarError(event: Event): void {
    const el = event.target as HTMLImageElement | null;
    if (!el) return;
    el.classList.add('account-avatar--placeholder');
    el.removeAttribute('src');
  }

  async logout(): Promise<void> {
    await this.auth.logout();
    this.router.navigate(['/login']);
  }
}
