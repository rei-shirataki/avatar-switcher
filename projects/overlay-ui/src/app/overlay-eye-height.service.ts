import { Injectable, inject, signal } from '@angular/core';
import { OverlayBridgeService } from './overlay-bridge.service';

/**
 * eye-height.service.ts の簡略版(#27, v1スコープ)。現在値表示+ステップボタン
 * のみを対象とし、リセット・プリセット・スムーズモードは持たない。それらが
 * 前提とする EyeHeight/ScaleFactor/ScaleModified の異時点ペアリングrace対策
 * （プレハブ身長の逆算専用ロジック）も同様に不要なため移植していない。
 */
@Injectable({ providedIn: 'root' })
export class OverlayEyeHeightService {
  private readonly bridge = inject(OverlayBridgeService);

  private readonly _value = signal<number | null>(null);
  readonly value = this._value.asReadonly();

  constructor() {
    this.bridge.onEyeHeightUpdate((v) => this._value.set(v));
  }

  /**
   * VRコントローラーでのタップ直後に見た目が反応するよう、fire-and-forgetの
   * 送信結果(VRChatからのOSCエコー)を待たず楽観的に即時反映する。実際の
   * 反映値がエコーで届き次第それで上書きされる。
   */
  step(delta: number): void {
    const current = this._value() ?? 0;
    const next = Math.max(0, current + delta);
    this._value.set(next);
    this.bridge.setEyeHeight(next);
  }
}
