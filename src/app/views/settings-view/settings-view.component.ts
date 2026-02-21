import { Component } from '@angular/core';
import { VRChatAuthService } from '../../core/services/vrchat-auth.service';
import { Router } from '@angular/router';

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
    </div>
  `,
  styleUrl: './settings-view.component.scss',
})
export class SettingsViewComponent {
  constructor(
    public auth: VRChatAuthService,
    private router: Router,
  ) {}

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
