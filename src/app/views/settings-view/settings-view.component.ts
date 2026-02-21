import { Component, OnInit, OnDestroy, signal } from '@angular/core';
import { FormsModule } from '@angular/forms';
import { VRChatAuthService } from '../../core/services/vrchat-auth.service';
import { TauriService } from '../../core/services/tauri.service';
import { Router } from '@angular/router';

interface OverlayStatus {
  enabled: boolean;
  steamVrRunning: boolean;
  overlayActive: boolean;
}

@Component({
  selector: 'app-settings-view',
  standalone: true,
  imports: [FormsModule],
  template: `
    <div class="settings-view">
      <h2 class="page-title">設定</h2>

      @if (auth.user(); as user) {
        <div class="settings-section">
          <h3 class="section-title">アカウント</h3>
          <div class="account-card">
            <div class="account-info">
              <div class="account-name">{{ user.displayName }}</div>
              <div class="account-id">{{ user.id }}</div>
              <div class="account-status status-{{ user.status }}">
                {{ statusLabel(user.status) }}
              </div>
            </div>
            <button class="btn-logout" (click)="logout()">ログアウト</button>
          </div>
        </div>
      }

      <div class="settings-section">
        <h3 class="section-title">SteamVRオーバーレイ</h3>

        <div class="settings-card">
          <div class="setting-row">
            <span class="setting-label">SteamVR</span>
            <span class="status-badge" [class.status-badge--on]="overlayStatus()?.steamVrRunning">
              {{ overlayStatus()?.steamVrRunning ? '実行中' : '未起動' }}
            </span>
          </div>
          <div class="setting-row">
            <span class="setting-label">オーバーレイ</span>
            <button
              class="toggle-btn"
              [class.toggle-btn--on]="overlayStatus()?.enabled"
              (click)="toggleOverlay()"
            >
              {{ overlayStatus()?.enabled ? 'ON' : 'OFF' }}
            </button>
          </div>
          <div class="setting-row">
            <span class="setting-label">表示状態</span>
            <span class="status-badge" [class.status-badge--on]="overlayStatus()?.overlayActive">
              {{ overlayStatus()?.overlayActive ? 'アクティブ' : '非アクティブ' }}
            </span>
          </div>
        </div>

        <div class="settings-card">
          <div class="setting-row setting-row--column">
            <label class="setting-label">
              オーバーレイ幅
              <span class="setting-value">{{ widthMeters().toFixed(1) }} m</span>
            </label>
            <input
              class="slider"
              type="range"
              min="0.5"
              max="3.0"
              step="0.1"
              [ngModel]="widthMeters()"
              (ngModelChange)="onWidthChange($event)"
            />
          </div>
          <p class="setting-hint">HMD正面1.5mに固定表示されます</p>
        </div>

        @if (!overlayStatus()?.steamVrRunning) {
          <div class="info-box">
            SteamVRが起動していません。SteamVRを起動してからオーバーレイを有効にしてください。
          </div>
        }
      </div>
    </div>
  `,
  styleUrl: './settings-view.component.scss',
})
export class SettingsViewComponent implements OnInit, OnDestroy {
  overlayStatus = signal<OverlayStatus | null>(null);
  widthMeters = signal(1.5);

  private pollInterval: ReturnType<typeof setInterval> | null = null;

  constructor(
    public auth: VRChatAuthService,
    private tauri: TauriService,
    private router: Router,
  ) {}

  async ngOnInit() {
    await this.refreshOverlayStatus();
    this.pollInterval = setInterval(() => this.refreshOverlayStatus(), 3000);
  }

  ngOnDestroy() {
    if (this.pollInterval) clearInterval(this.pollInterval);
  }

  async refreshOverlayStatus() {
    const s = await this.tauri.invoke<OverlayStatus>('overlay_get_status');
    this.overlayStatus.set(s);
  }

  async toggleOverlay() {
    const current = this.overlayStatus()?.enabled ?? false;
    await this.tauri.invoke('overlay_set_enabled', { enabled: !current });
    await this.refreshOverlayStatus();
  }

  async onWidthChange(value: number) {
    this.widthMeters.set(Number(value));
    await this.tauri.invoke('overlay_set_width', { widthMeters: Number(value) });
  }

  statusLabel(status: string): string {
    const map: Record<string, string> = {
      'join me': '参加募集中',
      'active': 'アクティブ',
      'ask me': '声かけOK',
      'busy': 'ビジー',
    };
    return map[status] ?? status;
  }

  async logout() {
    await this.auth.logout();
    this.router.navigate(['/login']);
  }
}
