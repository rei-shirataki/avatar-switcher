import { Component, CUSTOM_ELEMENTS_SCHEMA } from '@angular/core';
import { NgClass } from '@angular/common';
import { RouterLink, RouterLinkActive } from '@angular/router';
import { VRChatAuthService } from '../../../core/services/vrchat-auth.service';
import { Router } from '@angular/router';
import { IconComponent } from '../icon/icon.component';

interface NavItem {
  route: string;
  iconName: string;
  label: string;
}

@Component({
  selector: 'app-sidebar',
  standalone: true,
  imports: [RouterLink, RouterLinkActive, IconComponent, NgClass],
  host: { class: 'flex self-stretch' },
  template: `
    <nav class="w-[72px] flex flex-col bg-[var(--charcoal-color-dark-neutral--5)] border-r border-[var(--charcoal-color-container-secondary-default)] flex-shrink-0 flex-1 pt-2">
      <div class="flex-1 flex flex-col gap-1 p-2">
        @for (item of navItems; track item.route) {
          <a
            #rla="routerLinkActive"
            routerLinkActive
            class="flex flex-col items-center justify-center gap-[5px] py-[14px] px-1 rounded-m no-underline transition-colors duration-150 cursor-pointer relative"
            [ngClass]="rla.isActive ? 'bg-primary-dim text-primary' : 'text-text-tertiary hover:bg-background hover:text-text-secondary'"
            [routerLink]="item.route"
            [title]="item.label"
          >
            <app-icon [name]="item.iconName" [size]="20"/>
            <span class="text-[10px] font-medium tracking-[0.02em]">{{ item.label }}</span>
            @if (rla.isActive) {
              <span class="absolute -left-2 top-1/2 -translate-y-1/2 w-[3px] h-5 bg-primary rounded-r-[2px]"></span>
            }
          </a>
        }
      </div>
      <div class="flex flex-col items-center gap-1 p-2 border-t border-[var(--charcoal-color-container-secondary-default)] mt-1">
        @if (auth.user(); as user) {
          <div class="w-full flex items-center justify-center py-[10px] px-1">
            @if (user.profilePicOverride || user.userIcon || user.currentAvatarImageUrl; as img) {
              <img class="w-9 h-9 rounded-full object-cover border-2 border-primary-dim" [src]="img" alt="user" />
            } @else {
              <div class="w-9 h-9 rounded-full border-2 border-primary-dim flex items-center justify-center bg-container-secondary text-text-secondary text-sm font-semibold">
                {{ user.displayName.charAt(0) }}
              </div>
            }
          </div>
        }
        <button
          class="w-full border-0 bg-transparent text-text-tertiary rounded-m cursor-pointer flex flex-col items-center justify-center gap-[5px] py-[14px] px-1 transition-colors duration-150 hover:bg-[rgba(245,108,108,0.12)] hover:text-text-negative"
          (click)="logout()"
          title="ログアウト"
        >
          <pixiv-icon name="24/Logout" fixed-size="16" style="--charcoal-icon-size: 16px"></pixiv-icon>
          <span class="text-[10px] font-medium tracking-[0.02em]">ログアウト</span>
        </button>
      </div>
    </nav>
  `,
  schemas: [CUSTOM_ELEMENTS_SCHEMA],
})
export class SidebarComponent {
  navItems: NavItem[] = [
    { route: '/avatars', iconName: 'avatars', label: 'アバター' },
    { route: '/height', iconName: 'height', label: '身長' },
    { route: '/settings', iconName: 'settings', label: '設定' },
  ];

  constructor(public auth: VRChatAuthService, private router: Router) {}

  async logout() {
    await this.auth.logout();
    this.router.navigate(['/login']);
  }
}
