import { Component, OnInit } from '@angular/core';
import { Router, RouterOutlet } from '@angular/router';
import { VRChatAuthService } from './core/services/vrchat-auth.service';
import { TitleBarComponent } from './shared/components/title-bar/title-bar.component';
import { SidebarComponent } from './shared/components/sidebar/sidebar.component';
import { CommonModule } from '@angular/common';

@Component({
  selector: 'app-root',
  standalone: true,
  imports: [RouterOutlet, TitleBarComponent, SidebarComponent, CommonModule],
  template: `
    <div class="app-window" (contextmenu)="$event.preventDefault()">
      <app-title-bar />
      <div class="app-body">
        @if (auth.isLoggedIn()) {
          <app-sidebar />
        }
        <main class="app-content">
          <router-outlet />
        </main>
      </div>
    </div>
  `,
  styleUrl: './app.component.scss',
})
export class AppComponent implements OnInit {
  constructor(public auth: VRChatAuthService, private router: Router) {}

  async ngOnInit() {
    await this.auth.ensureInitialized();
    if (!this.auth.isLoggedIn()) {
      this.router.navigate(['/login']);
    }
  }
}
