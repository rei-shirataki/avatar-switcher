import { Component, OnInit } from '@angular/core';
import { Router, RouterOutlet } from '@angular/router';
import { VRChatAuthService } from './core/services/vrchat-auth.service';
import { EyeHeightService } from './core/services/eye-height.service';
import { TitleBarComponent } from './shared/components/title-bar/title-bar.component';
import { SidebarComponent } from './shared/components/sidebar/sidebar.component';
import { CommonModule } from '@angular/common';

@Component({
  selector: 'app-root',
  standalone: true,
  imports: [RouterOutlet, TitleBarComponent, SidebarComponent, CommonModule],
  template: `
    <div
      class="flex flex-col w-screen h-screen bg-[var(--charcoal-color-dark-neutral--10)] rounded-l overflow-hidden border border-[var(--charcoal-color-container-secondary-default)]"
      (contextmenu)="$event.preventDefault()"
    >
      <app-title-bar />
      <div class="flex flex-1 overflow-hidden">
        @if (auth.isLoggedIn()) {
          <app-sidebar />
        }
        <main class="flex-1 overflow-hidden flex flex-col">
          <router-outlet />
        </main>
      </div>
    </div>
  `,
})
export class AppComponent implements OnInit {
  constructor(
    public auth: VRChatAuthService,
    private router: Router,
    // EyeHeightService をここでインジェクトして起動時に常駐させ、
    // 別ビュー閲覧中でも OSC からの EyeHeightAsMeters を取り逃さないようにする。
    private _eyeHeight: EyeHeightService,
  ) {}

  async ngOnInit() {
    await this.auth.ensureInitialized();
    if (!this.auth.isLoggedIn()) {
      this.router.navigate(['/login']);
    }
  }
}
