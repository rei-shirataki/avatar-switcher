import { Injectable, signal } from '@angular/core';
import { Subject, firstValueFrom, filter, timeout } from 'rxjs';
import { AvatarFolder, VRCAvatar } from './core/models/avatar.model';

/** Rust core からの応答が返ってこない異常系（送信ドロップ等）を無限待ちさせないための上限。 */
const REQUEST_TIMEOUT_MS = 10_000;

/** avatars.list-result の応答。favoriteIds/uploadedIds はお気に入り/アップロード済みタブのフィルタ用。 */
export interface AvatarsListResult {
  avatars: VRCAvatar[];
  favoriteIds: string[];
  uploadedIds: string[];
}

/** ui-state.get-result の応答。ソートモード・選択中タブの永続化状態（詳細は setUiState 参照）。 */
export interface UiState {
  sortMode: string;
  selectedFolderId: string | null;
}

type ServerMessage =
  | ({ type: 'avatars.list-result' } & AvatarsListResult)
  | { type: 'avatars.select-result'; avatarId: string }
  | { type: 'avatar-changed'; avatarId: string }
  | { type: 'folders.list-result'; folders: AvatarFolder[] }
  | { type: 'eyeheight-update'; value: number }
  | ({ type: 'ui-state.get-result' } & UiState)
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

  /**
   * WebSocket接続 + hello送信が完了するまで待つ。呼び出し側 (app.ts) は
   * このPromiseの解決を待ってから listAvatars() 等を呼ぶこと。
   * onopen直後は非同期でOPEN状態になるだけで即座に送信可能になるとは限らない
   * ため、接続前に listAvatars() 等を呼ぶとメッセージが送信スキップされ、
   * 応答を待つPromiseが解決しないまま止まってしまう。
   */
  connect(): Promise<void> {
    const params = new URLSearchParams(window.location.search);
    const port = params.get('port');
    const token = params.get('token');
    if (!port || !token) {
      console.error('[overlay-bridge] URLクエリに port/token がありません');
      return Promise.reject(new Error('port/token missing'));
    }
    return this.openSocket(port, token);
  }

  private openSocket(port: string, token: string): Promise<void> {
    return new Promise((resolve) => {
      const ws = new WebSocket(`ws://127.0.0.1:${port}`);
      this.ws = ws;
      let resolved = false;

      ws.onopen = () => {
        this.connected.set(true);
        // hello はトークン認証を兼ねるため、Rust側は接続後最初のメッセージが
        // hello でないと即切断する。他の送信より必ず先に届く必要がある。
        ws.send(JSON.stringify({ type: 'sidecar.hello', pid: 0, token }));
        if (!resolved) {
          resolved = true;
          resolve();
        }
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
    });
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

  async listAvatars(): Promise<AvatarsListResult> {
    this.send({ type: 'avatars.list' });
    const reply = await this.waitForOneOf(['avatars.list-result', 'error']);
    if (reply.type === 'error') throw new Error(reply.message);
    return reply;
  }

  onEyeHeightUpdate(handler: (value: number) => void): void {
    this.messages$
      .pipe(filter((m): m is Extract<ServerMessage, { type: 'eyeheight-update' }> => m.type === 'eyeheight-update'))
      .subscribe((m) => handler(m.value));
  }

  /** #27: fire-and-forgetのため応答を待たない。反映値はonEyeHeightUpdateで受け取る。 */
  setEyeHeight(value: number): void {
    this.send({ type: 'eyeheight.set', value });
  }

  /**
   * アイハイトをアバター本来のプレハブ身長にリセットする。計算はRust側
   * （`oscquery::compute_prefab_height`）で行うため、ここではリクエストを送るだけ。
   * fire-and-forgetで、結果はonEyeHeightUpdateで受け取る。
   */
  resetEyeHeight(): void {
    this.send({ type: 'eyeheight.reset' });
  }

  /**
   * 接続直後の初回同期用。受動OSCイベント任せだと、アバターロード時の
   * ダンプを取りこぼした場合に身長変更が一度も起きるまで表示が「—」の
   * まま固まる。fire-and-forgetで、応答はonEyeHeightUpdate経由で届く
   * （現在値が取得できなかった場合は何も届かない＝表示は「—」のまま）。
   */
  queryEyeHeight(): void {
    this.send({ type: 'eyeheight.query' });
  }

  /**
   * 接続直後の初回同期用。overlay-sidecarはCEFプロセスごとに新しいキャッシュ
   * ディレクトリを使う(`Program.cs::InitCef`)ため、ブラウザのlocalStorageは
   * 再起動をまたいで永続化できない。Rust側に保存された値を取得する。
   */
  async getUiState(): Promise<UiState> {
    this.send({ type: 'ui-state.get' });
    const reply = await this.waitForOneOf(['ui-state.get-result', 'error']);
    if (reply.type === 'error') throw new Error(reply.message);
    return reply;
  }

  /** ソート/タブ切り替えのたびに送る。fire-and-forgetのため応答を待たない（setEyeHeightと同じ方針）。 */
  setUiState(state: UiState): void {
    this.send({ type: 'ui-state.set', ...state });
  }

  async listFolders(): Promise<AvatarFolder[]> {
    this.send({ type: 'folders.list' });
    const reply = await this.waitForOneOf(['folders.list-result', 'error']);
    if (reply.type === 'error') throw new Error(reply.message);
    return reply.folders;
  }

  async selectAvatar(avatarId: string): Promise<void> {
    this.send({ type: 'avatars.select', avatarId });
    const reply = await this.waitForOneOf(['avatars.select-result', 'error']);
    if (reply.type === 'error') throw new Error(reply.message);
  }

  private waitForOneOf<T extends ServerMessage['type']>(
    types: T[],
  ): Promise<Extract<ServerMessage, { type: T }>> {
    return firstValueFrom(
      this.messages$.pipe(
        filter((m): m is Extract<ServerMessage, { type: T }> => (types as string[]).includes(m.type)),
        // 送信がドロップされた等で応答が永遠に来ない異常系を、無限に読み込み中
        // のまま固まる代わりにエラーとして表面化させる。
        timeout(REQUEST_TIMEOUT_MS),
      ),
    );
  }
}
