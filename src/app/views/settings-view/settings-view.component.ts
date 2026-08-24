import { Component, DestroyRef, OnInit, inject, signal } from '@angular/core';
import { NgClass } from '@angular/common';
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

interface LicenseEntry {
  name: string;
  license: string;
  text: string;
}

/// VRChat 起動状態のポーリング間隔。VRChat 後起動時に「未接続」表示のまま
/// 取り残されないよう、設定画面表示中だけ短めに polling する。
const OSC_POLL_INTERVAL_MS = 3000;

// リポジトリ直下の LICENSE / overlay-sidecar 配下の LICENSE.* と内容を同期させること。
// Tauri の fs リソースバンドル経由で実ファイルを読む案もあったが、静的な同梱テキスト3件のために
// 新規capability/リソース設定を増やすほどではないため、定数として直接埋め込んでいる。
const LICENSES: LicenseEntry[] = [
  {
    name: 'avatar-switcher',
    license: 'MIT License',
    text: `MIT License

Copyright (c) 2026 rei-shirataki

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.`,
  },
  {
    name: 'OyasumiVR（overlay-sidecar/Resources/pointer.png）',
    license: 'MIT License',
    text: `MIT License

Copyright (c) 2022 Raphiiko

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.`,
  },
  {
    name: 'OpenVR（overlay-sidecar/OpenVR/openvr_api）',
    license: 'BSD 3-Clause License',
    text: `Copyright (c) 2015, Valve Corporation
All rights reserved.

Redistribution and use in source and binary forms, with or without modification,
are permitted provided that the following conditions are met:

1. Redistributions of source code must retain the above copyright notice, this
list of conditions and the following disclaimer.

2. Redistributions in binary form must reproduce the above copyright notice,
this list of conditions and the following disclaimer in the documentation and/or
other materials provided with the distribution.

3. Neither the name of the copyright holder nor the names of its contributors
may be used to endorse or promote products derived from this software without
specific prior written permission.

THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS" AND
ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE IMPLIED
WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE
DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE LIABLE FOR
ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES
(INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES;
LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON
ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT
(INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE OF THIS
SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.`,
  },
];

@Component({
  selector: 'app-settings-view',
  standalone: true,
  imports: [NgClass],
  template: `
    <div class="p-6 h-full overflow-y-auto">
      <h2 class="m-0 mb-6 text-lg font-bold text-text">設定</h2>

      @if (auth.user(); as user) {
        <div class="mb-6">
          <h3 class="m-0 mb-3 text-[11px] font-semibold uppercase tracking-[0.08em] text-text-tertiary">アカウント</h3>
          <div class="bg-background border border-[var(--charcoal-color-container-secondary-default)] rounded-l p-4 flex items-center justify-between gap-4">
            @if (avatarUrl(user); as url) {
              <img
                class="w-10 h-10 rounded-full object-cover flex-shrink-0 bg-container-secondary"
                [src]="url"
                alt=""
                (error)="onAvatarError($event)"
              />
            } @else {
              <div class="w-10 h-10 rounded-full flex-shrink-0 bg-container-secondary"></div>
            }
            <div class="flex-1 min-w-0">
              <div class="text-[15px] font-semibold text-text">{{ user.displayName }}</div>
            </div>
            <button class="py-2 px-4 bg-[rgba(245,108,108,0.1)] border border-[rgba(245,108,108,0.3)] rounded-m text-text-negative text-xs font-[var(--font-sans)] cursor-pointer transition-colors duration-150 whitespace-nowrap flex-shrink-0 hover:bg-[rgba(245,108,108,0.2)]" (click)="logout()">ログアウト</button>
          </div>
        </div>
      }

      <div class="mb-6">
        <h3 class="m-0 mb-3 text-[11px] font-semibold uppercase tracking-[0.08em] text-text-tertiary">OSC</h3>
        <div class="bg-background border border-[var(--charcoal-color-container-secondary-default)] rounded-l p-4 flex flex-col gap-3 mb-3">
          <div class="flex items-center justify-between">
            <span class="text-[13px] text-text-secondary">VRChat 接続</span>
            @if (oscStatus(); as s) {
              <span
                class="text-[11px] py-0.5 px-2.5 rounded-s"
                [ngClass]="s.vrchat_detected ? 'bg-[rgba(86,201,110,0.1)] text-text-positive' : 'bg-[rgba(136,136,170,0.1)] text-text-tertiary'"
              >
                {{ s.vrchat_detected ? '接続中' : '未接続' }}
              </span>
            } @else {
              <span class="text-[11px] py-0.5 px-2.5 rounded-s bg-[rgba(136,136,170,0.1)] text-text-tertiary">確認中…</span>
            }
          </div>
          @if (oscStatus()?.vrchat_detected) {
            <div class="flex items-center justify-between">
              <span class="text-[13px] text-text-secondary">ポート</span>
              <span class="text-primary font-semibold">{{ oscStatus()?.port }}</span>
            </div>
          }
        </div>
      </div>

      <div class="mb-6">
        <h3 class="m-0 mb-3 text-[11px] font-semibold uppercase tracking-[0.08em] text-text-tertiary">SteamVRオーバーレイ</h3>
        <div class="bg-background border border-[var(--charcoal-color-container-secondary-default)] rounded-l p-4 flex flex-col gap-3 mb-3">
          <div class="flex flex-col items-stretch gap-2">
            <span class="text-[13px] text-text-secondary">パネルの配置</span>
            <div class="flex gap-2">
              <button
                class="flex-1 py-1 px-5 rounded-m text-xs font-semibold font-[var(--font-sans)] cursor-pointer transition-all duration-150"
                [ngClass]="placementMode() === 'hand' ? 'bg-primary-dim border border-primary text-primary' : 'bg-container-secondary border border-[var(--charcoal-color-container-secondary-default)] text-text-tertiary'"
                (click)="setPlacementMode('hand')"
              >
                手に追従
              </button>
              <button
                class="flex-1 py-1 px-5 rounded-m text-xs font-semibold font-[var(--font-sans)] cursor-pointer transition-all duration-150"
                [ngClass]="placementMode() === 'space' ? 'bg-primary-dim border border-primary text-primary' : 'bg-container-secondary border border-[var(--charcoal-color-container-secondary-default)] text-text-tertiary'"
                (click)="setPlacementMode('space')"
              >
                空間に固定
              </button>
            </div>
            <p class="m-0 text-[11px] text-text-placeholder">変更はオーバーレイの再起動後に反映されます（アプリの再起動が必要です）</p>
          </div>
        </div>
      </div>

      <div class="mb-6">
        <h3 class="m-0 mb-3 text-[11px] font-semibold uppercase tracking-[0.08em] text-text-tertiary">このアプリについて</h3>
        <div class="bg-background border border-[var(--charcoal-color-container-secondary-default)] rounded-l p-4 flex flex-col gap-3 mb-3">
          <div class="flex items-center justify-between">
            <span class="text-[13px] text-text-secondary">バージョン</span>
            <span class="text-primary font-semibold">{{ version() }}</span>
          </div>
        </div>
      </div>

      <div class="mb-6">
        <h3 class="m-0 mb-3 text-[11px] font-semibold uppercase tracking-[0.08em] text-text-tertiary">ライセンス</h3>
        <div class="bg-background border border-[var(--charcoal-color-container-secondary-default)] rounded-l p-4 flex flex-col gap-3 mb-3">
          @for (entry of licenses; track entry.name; let i = $index) {
            <div class="flex items-center justify-between">
              <div class="flex flex-col gap-0.5">
                <span class="text-[13px] text-text-secondary">{{ entry.name }}</span>
                <span class="text-[11px] text-text-placeholder">{{ entry.license }}</span>
              </div>
              <button class="flex-shrink-0 py-1 px-4 rounded-m border border-[var(--charcoal-color-container-secondary-default)] bg-container-secondary text-text-tertiary text-xs font-semibold font-[var(--font-sans)] cursor-pointer transition-colors duration-150 hover:bg-primary-dim" (click)="toggleLicense(i)">
                {{ expandedLicense() === i ? '閉じる' : '表示' }}
              </button>
            </div>
            @if (expandedLicense() === i) {
              <pre class="m-0 p-3 bg-[var(--charcoal-color-dark-neutral--5)] border border-[var(--charcoal-color-container-secondary-default)] rounded-m font-[var(--font-mono,monospace)] text-[11px] leading-normal text-text-tertiary whitespace-pre-wrap break-words">{{ entry.text }}</pre>
            }
          }
        </div>
      </div>
    </div>
  `,
})
export class SettingsViewComponent implements OnInit {
  readonly version = signal<string>('...');
  readonly oscStatus = signal<OscStatus | null>(null);
  readonly placementMode = signal<OverlayPlacementMode>('hand');
  readonly licenses = LICENSES;
  readonly expandedLicense = signal<number | null>(null);

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

  toggleLicense(index: number): void {
    this.expandedLicense.set(this.expandedLicense() === index ? null : index);
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
    el.style.objectFit = 'initial';
    el.removeAttribute('src');
  }

  async logout(): Promise<void> {
    await this.auth.logout();
    this.router.navigate(['/login']);
  }
}
