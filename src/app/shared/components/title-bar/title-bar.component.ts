import { Component } from '@angular/core';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { IconComponent } from '../icon/icon.component';

@Component({
  selector: 'app-title-bar',
  standalone: true,
  imports: [IconComponent],
  template: `
    <div class="title-bar" data-tauri-drag-region>
      <div class="title-bar__logo" data-tauri-drag-region>
        <app-icon name="logo" [size]="16" class="title-bar__icon" data-tauri-drag-region/>
        <span class="title-bar__name" data-tauri-drag-region>AvatarSwitcher</span>
      </div>
      <div class="title-bar__controls">
        <button class="ctrl-btn ctrl-btn--min" (click)="minimize()" title="最小化">
          <app-icon name="minimize" [size]="10"/>
        </button>
        <button class="ctrl-btn ctrl-btn--max" (click)="maximize()" title="最大化">
          <app-icon name="maximize" [size]="10"/>
        </button>
        <button class="ctrl-btn ctrl-btn--close" (click)="close()" title="閉じる">
          <app-icon name="close" [size]="10"/>
        </button>
      </div>
    </div>
  `,
  styleUrl: './title-bar.component.scss',
})
export class TitleBarComponent {
  async minimize() { await getCurrentWindow().minimize(); }
  async maximize() { await getCurrentWindow().toggleMaximize(); }
  async close() { await getCurrentWindow().close(); }
}
