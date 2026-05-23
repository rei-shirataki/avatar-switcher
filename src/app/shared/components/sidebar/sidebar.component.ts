import { Component } from '@angular/core';
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
  imports: [RouterLink, RouterLinkActive, IconComponent],
  template: `
    <nav class="sidebar">
      <div class="sidebar__nav">
        @for (item of navItems; track item.route) {
          <a
            class="sidebar__item"
            [routerLink]="item.route"
            routerLinkActive="sidebar__item--active"
            [title]="item.label"
          >
            <app-icon [name]="item.iconName" [size]="20"/>
            <span class="sidebar__label">{{ item.label }}</span>
          </a>
        }
      </div>
      <div class="sidebar__bottom">
        @if (auth.user(); as user) {
          <div class="sidebar__user">
            @if (user.profilePicOverride || user.userIcon || user.currentAvatarImageUrl; as img) {
              <img class="sidebar__avatar" [src]="img" alt="user" />
            } @else {
              <div class="sidebar__avatar sidebar__avatar--placeholder">
                {{ user.displayName.charAt(0) }}
              </div>
            }
          </div>
        }
        <button class="sidebar__logout" (click)="logout()" title="ログアウト">
          <app-icon name="log-out" [size]="16"/>
          <span class="sidebar__logout-label">ログアウト</span>
        </button>
      </div>
    </nav>
  `,
  styleUrl: './sidebar.component.scss',
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
