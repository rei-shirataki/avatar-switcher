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
    // 接続完了（helloメッセージ送信済み）を待ってから要求を送る。
    // connect()の完了を待たずに refresh() すると、まだ OPEN 状態でない
    // WebSocket への送信が黙ってスキップされ、応答を待つPromiseが
    // 永遠に解決しないまま「読み込み中…」に固まってしまう。
    this.bridge
      .connect()
      .then(() => this.avatarService.refresh())
      .catch((e) => console.error('[overlay-ui] 初期化に失敗:', e));

    // デバッグ用: 「クリックできない」報告の切り分け。CEFから注入される
    // マウスクリックがDOMまで届いているか自体を、Angularのイベント
    // バインディングを経由せず直接確認する（Angular側の(click)束縛が
    // 発火しない場合と、そもそも座標がカードに当たっていない場合を
    // 区別するため）。原因判明後に削除する。
    document.addEventListener('click', (e) => {
      const target = e.target as HTMLElement | null;
      console.log(
        `[overlay-ui debug] raw click x=${e.clientX} y=${e.clientY} target=${target?.tagName}.${target?.className}`,
      );
    });
  }

  onSelect(avatarId: string): void {
    this.avatarService.switchAvatar(avatarId).catch((e) => console.error('[overlay-ui] アバター切替に失敗:', e));
  }
}
