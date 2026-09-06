import { Component, OnInit, inject } from '@angular/core';
import { OverlayBridgeService } from './overlay-bridge.service';
import { OverlayAvatarService } from './overlay-avatar.service';
import { AvatarGridComponent } from './avatar-grid/avatar-grid.component';
import { FolderTabsComponent } from './folder-tabs/folder-tabs.component';
import { EyeHeightControlComponent } from './eye-height-control/eye-height-control.component';

@Component({
  selector: 'app-root',
  imports: [AvatarGridComponent, FolderTabsComponent, EyeHeightControlComponent],
  templateUrl: './app.html',
  host: { class: 'block w-full h-full' },
})
export class App implements OnInit {
  private readonly bridge = inject(OverlayBridgeService);
  protected readonly avatarService = inject(OverlayAvatarService);

  ngOnInit(): void {
    // onConnectedは初回接続・サイドカー再起動後の自動再接続いずれでもhello送信後に
    // 発火するため、ここに登録した初期同期はサイドカーが一時的に落ちて戻ってきた
    // 場合にもやり直される（接続完了を待たずに送ると、まだOPEN状態でない
    // WebSocketへの送信が黙ってスキップされてしまうため、必ずこの中で行う）。
    this.bridge.onConnected(() => {
      // eyeheight.queryはfire-and-forget(応答はonEyeHeightUpdate経由)なので
      // Promise.allには含めない。受動OSCイベント任せだと身長を一度も変更して
      // いない場合に表示が「—」のまま固まるため、接続直後に能動フェッチする。
      this.bridge.queryEyeHeight();
      this.bridge.getEyeHeightSettings();
      Promise.all([
        this.avatarService.refresh(),
        this.avatarService.refreshFolders(),
        this.avatarService.restoreUiState(),
      ]).catch((e) => console.error('[overlay-ui] 初期化に失敗:', e));
    });
    this.bridge.connect().catch((e) => console.error('[overlay-ui] 初期化に失敗:', e));
  }

  onSelectFolder(folderId: string | null): void {
    this.avatarService.selectFolder(folderId);
  }

  onSelect(avatarId: string): void {
    this.avatarService.switchAvatar(avatarId).catch((e) => console.error('[overlay-ui] アバター切替に失敗:', e));
  }
}
