import { Injectable, signal } from '@angular/core';
import { Subject, firstValueFrom, filter } from 'rxjs';
import { VRCAvatar } from './core/models/avatar.model';

type ServerMessage =
  | { type: 'avatars.list-result'; avatars: VRCAvatar[] }
  | { type: 'avatars.select-result'; avatar: VRCAvatar }
  | { type: 'avatar-changed'; avatarId: string }
  | { type: 'error'; message: string };

/**
 * Rust core (src-tauri/src/steamvr/bridge.rs) への WebSocket クライアント。
 * tauri.service.ts の代替。overlay-sidecar が CEF に読み込ませる際、URL クエリ
 * `?port=<port>&token=<token>` で接続先を渡す（サイドカー起動時に Rust から
 * 動的に決まるポート番号・トークンを埋め込む必要があるため）。
 */
@Injectable({ providedIn: 'root' })
export class OverlayBridgeService {
  private ws: WebSocket | null = null;
  private readonly messages$ = new Subject<ServerMessage>();
  readonly connected = signal(false);

  connect(): void {
    const params = new URLSearchParams(window.location.search);
    const port = params.get('port');
    const token = params.get('token');
    if (!port || !token) {
      console.error('[overlay-bridge] URLクエリに port/token がありません');
      return;
    }
    this.openSocket(port, token);
  }

  private openSocket(port: string, token: string): void {
    const ws = new WebSocket(`ws://127.0.0.1:${port}`);
    this.ws = ws;

    ws.onopen = () => {
      this.connected.set(true);
      ws.send(JSON.stringify({ type: 'sidecar.hello', pid: 0, token }));
    };
    ws.onmessage = (ev) => {
      try {
        this.messages$.next(JSON.parse(ev.data) as ServerMessage);
      } catch (e) {
        console.warn('[overlay-bridge] 不正なメッセージを受信:', ev.data, e);
      }
    };
    ws.onclose = () => {
      this.connected.set(false);
      // サイドカー再起動直後の一時的な切断を想定した簡易リトライ。
      // 恒久的にサイドカーが消えた場合はページごと作り直されるため無限リトライで問題ない。
      setTimeout(() => this.openSocket(port, token), 2000);
    };
    ws.onerror = () => ws.close();
  }

  private send(message: Record<string, unknown>): void {
    if (this.ws?.readyState !== WebSocket.OPEN) {
      console.warn('[overlay-bridge] 未接続のため送信をスキップ:', message);
      return;
    }
    this.ws.send(JSON.stringify(message));
  }

  onAvatarChanged(handler: (avatarId: string) => void): void {
    this.messages$
      .pipe(filter((m): m is Extract<ServerMessage, { type: 'avatar-changed' }> => m.type === 'avatar-changed'))
      .subscribe((m) => handler(m.avatarId));
  }

  async listAvatars(): Promise<VRCAvatar[]> {
    this.send({ type: 'avatars.list' });
    const reply = await this.waitForOneOf(['avatars.list-result', 'error']);
    if (reply.type === 'error') throw new Error(reply.message);
    return reply.avatars;
  }

  async selectAvatar(avatarId: string): Promise<VRCAvatar> {
    this.send({ type: 'avatars.select', avatarId });
    const reply = await this.waitForOneOf(['avatars.select-result', 'error']);
    if (reply.type === 'error') throw new Error(reply.message);
    return reply.avatar;
  }

  private waitForOneOf<T extends ServerMessage['type']>(
    types: T[],
  ): Promise<Extract<ServerMessage, { type: T }>> {
    return firstValueFrom(
      this.messages$.pipe(
        filter((m): m is Extract<ServerMessage, { type: T }> => (types as string[]).includes(m.type)),
      ),
    );
  }
}
