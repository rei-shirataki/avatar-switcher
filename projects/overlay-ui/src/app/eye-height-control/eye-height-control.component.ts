import { Component, DestroyRef, inject } from '@angular/core';
import { DecimalPipe, NgClass } from '@angular/common';
import { OverlayEyeHeightService } from '../overlay-eye-height.service';

/** 長押し開始からオートリピートが始まるまでの遅延。単発タップと区別するため。 */
const REPEAT_INITIAL_DELAY_MS = 400;
/** オートリピート中の増減間隔。 */
const REPEAT_INTERVAL_MS = 150;

@Component({
  selector: 'app-eye-height-control',
  imports: [DecimalPipe, NgClass],
  templateUrl: './eye-height-control.component.html',
})
export class EyeHeightControlComponent {
  protected readonly eyeHeight = inject(OverlayEyeHeightService);

  private repeatTimeout: ReturnType<typeof setTimeout> | null = null;
  private repeatInterval: ReturnType<typeof setInterval> | null = null;

  constructor() {
    // コンポーネント破棄時（フォルダタブ切替等でDOMごと消える場合）にタイマーが
    // 残り続けないよう明示的に止める。
    inject(DestroyRef).onDestroy(() => this.stopRepeat());
  }

  /**
   * ステップボタンの押下開始。即座に1回分反映しつつ、
   * REPEAT_INITIAL_DELAY_MS 経過後もまだ押されていればオートリピートを始める。
   * VRコントローラーでの押しっぱなし（長押し）で連続増減できるようにするための挙動。
   */
  startRepeat(delta: number): void {
    this.stopRepeat();
    this.eyeHeight.step(delta);
    this.repeatTimeout = setTimeout(() => {
      this.repeatInterval = setInterval(() => this.eyeHeight.step(delta), REPEAT_INTERVAL_MS);
    }, REPEAT_INITIAL_DELAY_MS);
  }

  /** ボタンを離した/ポインタが外れた時に呼ぶ。 */
  stopRepeat(): void {
    if (this.repeatTimeout !== null) {
      clearTimeout(this.repeatTimeout);
      this.repeatTimeout = null;
    }
    if (this.repeatInterval !== null) {
      clearInterval(this.repeatInterval);
      this.repeatInterval = null;
    }
  }
}
