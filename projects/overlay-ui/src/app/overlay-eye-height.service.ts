import { Injectable, inject, signal } from '@angular/core';
import { OverlayBridgeService } from './overlay-bridge.service';

/** reset()応答(eyeheight-update)が万一届かなかった場合にボタンを永久に無効化
 *  したままにしないための保険。ローカルOSC送信の失敗はまず起きないが、
 *  fire-and-forgetのためRust側のエラー応答は拾っていない。 */
const RESET_TIMEOUT_MS = 3000;

const EYE_HEIGHT_MIN = 0.2;
const EYE_HEIGHT_MAX = 5.0;

/** 送信モード。eye-height.service.ts::EyeHeightMode と同じ意味。
 *  - `instant`: 目標値を1発で送る。
 *  - `smooth` : 現在値→目標値を ~500ms でイージング補間しながら小刻みに送信する。 */
export type EyeHeightMode = 'instant' | 'smooth';
const DEFAULT_MODE: EyeHeightMode = 'instant';
const SMOOTH_DURATION_MS = 500;
const SMOOTH_STEP_MS = 33;

/** スムーズ補間終了直後、補間中に送った中間値のOSCエコーが遅れて届いて
 *  最終値を巻き戻すのを防ぐための抑止ウィンドウ。eye-height.service.ts の
 *  ECHO_SUPPRESS_MS と同じ意図（詳細はそちら参照）。 */
const ECHO_SUPPRESS_MS = 800;

/** eye-height.service.ts と同じ cubic-bezier イージング関数生成。
 *  easeInOutCubic 相当の対称カーブ。 */
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
const SMOOTH_EASE = cubicBezier(0.65, 0.0, 0.35, 1.0);

/**
 * eye-height.service.ts の簡略版(#27, v1スコープ)。現在値表示+ステップボタン+
 * リセット+送信モード切替を対象とし、プリセット・ワールド範囲表示は持たない。
 * デスクトップ側の`EyeHeightService`が持つ EyeHeight/ScaleFactor/ScaleModified の
 * 異時点ペアリングrace対策は、リセット計算自体をRust側
 * (`oscquery::compute_prefab_height`)に寄せて3値を`tokio::join!`で同時取得する
 * ことで不要にしている（受動OSCストリームを継続的に監視するdesktop方式とは異なる設計）。
 */
@Injectable({ providedIn: 'root' })
export class OverlayEyeHeightService {
  private readonly bridge = inject(OverlayBridgeService);

  private readonly _value = signal<number | null>(null);
  /** 直近の目標値。補間中の中間値ではなくこれを起点にstep()することで、
   *  スムーズモード中の連打でも期待通りの累計分だけ動く（desktopの_targetと同じ）。 */
  private readonly _target = signal<number>(0);
  private readonly _isSmoothing = signal(false);
  private readonly _mode = signal<EyeHeightMode>(DEFAULT_MODE);
  private readonly _resetting = signal(false);

  readonly value = this._value.asReadonly();
  readonly isSmoothing = this._isSmoothing.asReadonly();
  readonly mode = this._mode.asReadonly();
  readonly resetting = this._resetting.asReadonly();

  private smoothTimer: ReturnType<typeof setInterval> | null = null;
  /** 直近のスムーズ補間終了時刻(performance.now())。ECHO_SUPPRESS_MSの起点。 */
  private _smoothEndedAt = Number.NEGATIVE_INFINITY;

  constructor() {
    this.bridge.onEyeHeightUpdate((v) => {
      this._resetting.set(false);
      // 補間の実行中・終了直後は自前の値を信頼し、遅れて届く中間値エコーで
      // 巻き戻さない（詳細はECHO_SUPPRESS_MSのコメント参照）。
      if (this._isSmoothing()) return;
      if (performance.now() - this._smoothEndedAt < ECHO_SUPPRESS_MS) return;
      this._value.set(v);
      this._target.set(v);
    });
  }

  /** モードを切り替える。進行中のスムージングはキャンセルする。 */
  setMode(mode: EyeHeightMode): void {
    this.cancelSmooth();
    this._mode.set(mode);
  }

  /**
   * VRコントローラーでのタップ直後に見た目が反応するよう、instantモードでは
   * fire-and-forgetの送信結果を待たず楽観的に即時反映する。smoothモードでは
   * 現在値→目標値を補間しながら連続送信する。
   */
  step(delta: number): void {
    const next = this.clamp(this._target() + delta);
    this._target.set(next);
    this.cancelSmooth();
    if (this._mode() === 'smooth') {
      this.startSmoothTo(next);
    } else {
      this._value.set(next);
      this.bridge.setEyeHeight(next);
    }
  }

  /** スムージング中なら停止する。 */
  private cancelSmooth(): void {
    if (this.smoothTimer !== null) {
      clearInterval(this.smoothTimer);
      this.smoothTimer = null;
    }
    if (this._isSmoothing()) {
      this._isSmoothing.set(false);
      this._smoothEndedAt = performance.now();
    }
  }

  /** eye-height.service.ts::startSmoothTo と同じイージング補間。 */
  private startSmoothTo(target: number): void {
    const start = this._value() ?? target;
    if (Math.abs(target - start) < 0.005) {
      this._value.set(target);
      this.bridge.setEyeHeight(target);
      return;
    }
    const startTime = performance.now();
    this._isSmoothing.set(true);
    this.smoothTimer = setInterval(() => {
      const elapsed = performance.now() - startTime;
      const t = Math.min(1, elapsed / SMOOTH_DURATION_MS);
      if (t >= 1) {
        this.smoothTimer = null;
        this._isSmoothing.set(false);
        this._smoothEndedAt = performance.now();
        this._value.set(target);
        this.bridge.setEyeHeight(target);
        return;
      }
      const eased = SMOOTH_EASE(t);
      const raw = this.clamp(start + (target - start) * eased);
      this._value.set(raw);
      this.bridge.setEyeHeight(raw);
    }, SMOOTH_STEP_MS);
  }

  /**
   * アバター本来の身長にリセットする。計算結果はRust側のOSCQuery問い合わせを
   * 挟むため即値が分からず、step()と違い楽観的更新はできない。応答
   * (eyeheight-update broadcast)が届くまでボタンを無効化して連打を防ぐ。
   */
  reset(): void {
    if (this._resetting()) return;
    this.cancelSmooth();
    this._resetting.set(true);
    this.bridge.resetEyeHeight();
    setTimeout(() => this._resetting.set(false), RESET_TIMEOUT_MS);
  }

  private clamp(v: number): number {
    return Math.max(EYE_HEIGHT_MIN, Math.min(EYE_HEIGHT_MAX, v));
  }
}
