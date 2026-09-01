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
    // 接続完了（helloメッセージ送信済み）を待ってから要求を送る。
    // connect()の完了を待たずに refresh() すると、まだ OPEN 状態でない
    // WebSocket への送信が黙ってスキップされ、応答を待つPromiseが
    // 永遠に解決しないまま「読み込み中…」に固まってしまう。
    this.bridge
      .connect()
      .then(() => {
        // eyeheight.queryはfire-and-forget(応答はonEyeHeightUpdate経由)なので
        // Promise.allには含めない。受動OSCイベント任せだと身長を一度も変更して
        // いない場合に表示が「—」のまま固まるため、接続直後に能動フェッチする。
        this.bridge.queryEyeHeight();
        this.bridge.getEyeHeightSettings();
        return Promise.all([
          this.avatarService.refresh(),
          this.avatarService.refreshFolders(),
          this.avatarService.restoreUiState(),
        ]);
      })
      .catch((e) => console.error('[overlay-ui] 初期化に失敗:', e));
  }

  onSelectFolder(folderId: string | null): void {
    this.avatarService.selectFolder(folderId);
  }

  onSelect(avatarId: string): void {
    this.avatarService.switchAvatar(avatarId).catch((e) => console.error('[overlay-ui] アバター切替に失敗:', e));
  }
}
