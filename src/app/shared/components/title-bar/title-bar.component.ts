import { Component } from '@angular/core';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { IconComponent } from '../icon/icon.component';

@Component({
  selector: 'app-title-bar',
  standalone: true,
  imports: [IconComponent],
  template: `
    <div class="flex items-center justify-between h-9 py-0 pr-2 pl-3 border-b border-[var(--charcoal-color-container-secondary-default)] flex-shrink-0 select-none bg-[var(--charcoal-color-dark-neutral--5)]" data-tauri-drag-region>
      <div class="flex items-center gap-1.5 cursor-default" data-tauri-drag-region>
        <app-icon name="logo" [size]="16" class="text-primary" data-tauri-drag-region/>
        <span class="text-xs font-semibold tracking-[0.03em] text-text-secondary" data-tauri-drag-region>AvatarSwitcher</span>
      </div>
      <div class="flex gap-0.5">
        <button class="w-8 h-7 border-0 cursor-pointer flex items-center justify-center transition-colors duration-150 p-0 bg-transparent rounded-s text-text-tertiary hover:bg-container-secondary hover:text-text" (click)="minimize()" title="最小化">
          <app-icon name="minimize" [size]="10"/>
        </button>
        <button class="w-8 h-7 border-0 cursor-pointer flex items-center justify-center transition-colors duration-150 p-0 bg-transparent rounded-s text-text-tertiary hover:bg-container-secondary hover:text-text" (click)="maximize()" title="最大化">
          <app-icon name="maximize" [size]="10"/>
        </button>
        <button class="w-8 h-7 border-0 cursor-pointer flex items-center justify-center transition-colors duration-150 p-0 bg-transparent rounded-s text-text-tertiary hover:bg-[#c42b1c] hover:text-white" (click)="close()" title="閉じる">
          <app-icon name="close" [size]="10"/>
        </button>
      </div>
    </div>
  `,
})
export class TitleBarComponent {
  async minimize() { await getCurrentWindow().minimize(); }
  async maximize() { await getCurrentWindow().toggleMaximize(); }
  async close() { await getCurrentWindow().close(); }
}
