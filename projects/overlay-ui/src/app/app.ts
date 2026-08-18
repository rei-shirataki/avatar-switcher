import { Component, OnInit, inject } from '@angular/core';
import { OverlayBridgeService } from './overlay-bridge.service';
import { OverlayAvatarService } from './overlay-avatar.service';
import { AvatarGridComponent } from './avatar-grid/avatar-grid.component';

@Component({
  selector: 'app-root',
  imports: [AvatarGridComponent],
  templateUrl: './app.html',
  styleUrl: './app.scss',
})
export class App implements OnInit {
  private readonly bridge = inject(OverlayBridgeService);
  protected readonly avatarService = inject(OverlayAvatarService);

  ngOnInit(): void {
    this.bridge.connect();
    this.avatarService.refresh().catch((e) => console.error('[overlay-ui] アバター一覧取得に失敗:', e));
  }

  onSelect(avatarId: string): void {
    this.avatarService.switchAvatar(avatarId).catch((e) => console.error('[overlay-ui] アバター切替に失敗:', e));
  }
}
