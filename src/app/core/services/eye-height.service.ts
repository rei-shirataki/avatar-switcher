import { DestroyRef, Injectable, inject, signal } from '@angular/core';
import { listen, UnlistenFn } from '@tauri-apps/api/event';
import { TauriService } from './tauri.service';

const STORAGE_KEY_VALUE = 'avatar-switcher.eyeheight.value';
const STORAGE_KEY_MODE = 'avatar-switcher.eyeheight.mode';

export const EYE_HEIGHT_DEFAULT = 1.6;
export const EYE_HEIGHT_MIN = 0.2;
export const EYE_HEIGHT_MAX = 5.0;

/** 自前の OSC 送信後、この期間内に VRChat から届くエコーは applyExternalValue で無視する。
 *  スムーズ終端で _isSmoothing=false に戻った直後に届く中間値エコーが _target を
 *  巻き戻すレースを潰す。VRChat 側の echo 遅延 (~50–200ms) と SMOOTH_DURATION_MS の
 *  どちらも余裕で覆える値にしてある。 */
const ECHO_SUPPRESS_MS = 800;

/** 送信モード。どちらも公式 `/avatar/eyeheight` に Float(m) を送る。
 *  - `instant`: 目標値を 1 発で送る。VRChat 側で即座に反映。
 *  - `smooth` : 現在値→目標値を ~500ms でイージング補間し、~33ms 毎に小刻みに
 *               送信して見た目をスムーズに変化させる。 */
export type EyeHeightMode = 'instant' | 'smooth';
const DEFAULT_MODE: EyeHeightMode = 'instant';
const SMOOTH_DURATION_MS = 500;
const SMOOTH_STEP_MS = 33;

/** スムーズモードのイージング曲線。easeInOutCubic 相当の対称カーブ。
 *  立ち上がりはゆっくり → 中盤で加速 → 終端で減速、の S 字を描く。 */
const SMOOTH_EASE = cubicBezier(0.65, 0.0, 0.35, 1.0);

/** CSS の cubic-bezier(x1,y1,x2,y2) と同じ曲線を返すイージング関数を生成する。
 *  返り値は (t: 0〜1) → (eased: 0〜1)。
 *  x(u) = t の u を Newton 法で解いて y(u) を返す標準実装。 */
function cubicBezier(x1: number, y1: number, x2: number, y2: number): (t: number) => number {
  const cx = 3 * x1;
  const bx = 3 * (x2 - x1) - cx;
  const ax = 1 - cx - bx;
  const cy = 3 * y1;
  const by = 3 * (y2 - y1) - cy;
  const ay = 1 - cy - by;
  const sampleX = (u: number) => ((ax * u + bx) * u + cx) * u;
  const sampleY = (u: number) => ((ay * u + by) * u + cy) * u;
  const derivX = (u: number) => (3 * ax * u + 2 * bx) * u + cx;
  return (t: number) => {
    if (t <= 0) return 0;
    if (t >= 1) return 1;
    let u = t;
    for (let i = 0; i < 8; i++) {
      const dx = sampleX(u) - t;
      if (Math.abs(dx) < 1e-6) break;
      const slope = derivX(u);
      if (Math.abs(slope) < 1e-6) break;
      u -= dx / slope;
    }
    return sampleY(u);
  };
}

@Injectable({ providedIn: 'root' })
export class EyeHeightService {
  private readonly _value = signal<number>(EYE_HEIGHT_DEFAULT);
  /** 直近の正規化済み目標値。adjust() の起点・スムーズ中の表示に使う。
   *  スムージング中はまだ補間途上で _value() < target なので、起点を _value()
   *  にすると連打時の +δ が壊れるためここで別途保持する。 */
  private readonly _target = signal<number>(EYE_HEIGHT_DEFAULT);
  /** スムーズ補間が進行中か。UI 表示の出し分けに使う signal。 */
  private readonly _isSmoothing = signal<boolean>(false);
  private readonly _mode = signal<EyeHeightMode>(DEFAULT_MODE);
  private readonly _lastError = signal<string | null>(null);

  readonly value = this._value.asReadonly();
  readonly target = this._target.asReadonly();
  readonly isSmoothing = this._isSmoothing.asReadonly();
  readonly mode = this._mode.asReadonly();
  readonly lastError = this._lastError.asReadonly();

  private unlisten: UnlistenFn | null = null;
  /** 進行中のスムージング interval ハンドル。null なら未進行。 */
  private smoothTimer: ReturnType<typeof setInterval> | null = null;
  /** 直近で sendOsc を呼んだ performance.now() 時刻。エコー抑止の起点。 */
  private _lastSelfSendAt = 0;

  constructor(private tauri: TauriService) {
    const saved = parseFloat(localStorage.getItem(STORAGE_KEY_VALUE) ?? '');
    if (Number.isFinite(saved)) {
      this._value.set(this.normalize(saved));
    }
    this._target.set(this._value());
    const savedMode = localStorage.getItem(STORAGE_KEY_MODE);
    if (savedMode === 'instant' || savedMode === 'smooth') {
      this._mode.set(savedMode);
    }
    // VRChat からの現在アイハイト通知を購読する。
    // サービスは providedIn: 'root' で常駐するため、別ビューにいてアバター切替された
    // 場合でも EyeHeightAsMeters を取りこぼさずに最新値を保持できる。
    listen<number>('osc:eye-height', e => this.applyExternalValue(e.payload))
      .then(un => { this.unlisten = un; })
      .catch(err => {
        this._lastError.set(`OSC受信購読失敗: ${String(err)}`);
      });

    // dev HMR 等でサービスが再構築される際にリスナーと interval を確実に解放する。
    // providedIn:'root' でも DestroyRef は機能する。
    inject(DestroyRef).onDestroy(() => {
      this.cancelSmooth();
      this.unlisten?.();
      this.unlisten = null;
    });
  }

  /** モードを切り替えて永続化する。進行中のスムージングはキャンセル。 */
  setMode(mode: EyeHeightMode): void {
    this.cancelSmooth();
    this._mode.set(mode);
    localStorage.setItem(STORAGE_KEY_MODE, mode);
  }

  /** ユーザ操作で値をセットし VRChat へ送信する。
   *  smooth モード時は現在値→目標値を補間しながら連続送信する。
   *  途中で新しい setValue が来たらその時点の現在値から再補間。 */
  async setValue(v: number): Promise<void> {
    if (!Number.isFinite(v)) return;
    const target = this.normalize(v);
    this.cancelSmooth();
    this._target.set(target);
    if (this._mode() === 'smooth') {
      this.startSmoothTo(target);
    } else {
      await this.commit(target);
    }
  }

  /** スムージング中なら停止する。新しい目標が来たとき・モード変更時に呼ぶ。 */
  private cancelSmooth(): void {
    if (this.smoothTimer !== null) {
      clearInterval(this.smoothTimer);
      this.smoothTimer = null;
    }
    this._isSmoothing.set(false);
  }

  /** 現在値から target まで SMOOTH_DURATION_MS 掛けてイージング補間しながら送信。
   *  途中の step は **丸めない生 Float** を OSC で送ることで VRChat 側の見た目を
   *  なめらかに連続変化させる。表示は displayValue() の toFixed(2) で別途整える。
   *  終端だけ正規化済み target に着地させて 0.01 単位できれいに止める。 */
  private startSmoothTo(target: number): void {
    const start = this._value();
    // 差分が 0.01 未満なら補間せず即時送信して終了。
    if (Math.abs(target - start) < 0.005) {
      void this.commit(target);
      return;
    }
    const startTime = performance.now();
    this._isSmoothing.set(true);
    this.smoothTimer = setInterval(() => {
      const elapsed = performance.now() - startTime;
      const t = Math.min(1, elapsed / SMOOTH_DURATION_MS);
      if (t >= 1) {
        // 終端：正規化済み target で完全停止し、永続化もここで。
        this.cancelSmooth();
        this._value.set(target);
        void this.sendOsc(target);
        this.persistValue(target);
        return;
      }
      // 途中：生 Float のまま流す（clamp だけかける、丸めない）。
      const eased = SMOOTH_EASE(t);
      const raw = this.clamp(start + (target - start) * eased);
      this._value.set(raw);
      void this.sendOsc(raw);
    }, SMOOTH_STEP_MS);
  }


  /** 即時送信＋永続化。instant モードと smooth 終端で使う共通パス。 */
  private async commit(value: number): Promise<void> {
    this._value.set(value);
    this.persistValue(value);
    await this.sendOsc(value);
  }

  private async sendOsc(value: number): Promise<void> {
    // エコー抑止ウィンドウを送信ごとに更新する。Promise resolve を待たず、
    // 「発火した瞬間」から VRChat エコーが戻り得るのでここで打刻する。
    this._lastSelfSendAt = performance.now();
    try {
      await this.tauri.invoke('osc_set_avatar_eye_height', { value });
      this._lastError.set(null);
    } catch (e) {
      this._lastError.set(`OSC 送信失敗: ${String(e)}`);
    }
  }

  private persistValue(value: number): void {
    localStorage.setItem(STORAGE_KEY_VALUE, String(value));
  }

  /** 増減ボタン用：起点は補間中の中間値ではなく _target（直近の目標値）。
   *  これでスムーズモードで連打しても期待通りの累計分だけ動く。 */
  async adjust(delta: number): Promise<void> {
    await this.setValue(this._target() + delta);
  }

  /** VRChat から受け取った値を再送なしで反映する。
   *  生の float（例: 1.6000001）が来るので、ボタン操作と同じ 0.01 桁に
   *  正規化して格納し、表示と次回 adjust() の起点が常にクリーンに保たれるようにする。
   *  スムージング進行中、および直近の自前送信から ECHO_SUPPRESS_MS 以内のエコーは
   *  「自分が投げた値が戻ってきただけ」とみなして無視する。スムーズ終端直後の
   *  遅延エコーで _target が中間値に巻き戻る race を防ぐ。 */
  private applyExternalValue(raw: unknown): void {
    if (this._isSmoothing()) return;
    if (performance.now() - this._lastSelfSendAt < ECHO_SUPPRESS_MS) return;
    const v = typeof raw === 'number' ? raw : parseFloat(String(raw));
    if (!Number.isFinite(v)) return;
    const normalized = this.normalize(v);
    this._value.set(normalized);
    // VRC 側で身長を変えた直後にユーザがボタンを押した時、その新しい値を
    // 起点にして増減できるよう _target も同期しておく。
    this._target.set(normalized);
    localStorage.setItem(STORAGE_KEY_VALUE, String(normalized));
  }

  /** 範囲クランプ＋0.01 桁丸めを 1 箇所に集約。
   *  すべての書き込みパスはこれを通すこと。 */
  private normalize(v: number): number {
    return Math.round(this.clamp(v) * 100) / 100;
  }

  private clamp(v: number): number {
    return Math.max(EYE_HEIGHT_MIN, Math.min(EYE_HEIGHT_MAX, v));
  }
}
