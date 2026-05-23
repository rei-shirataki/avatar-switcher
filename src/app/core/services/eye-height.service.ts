import { Injectable, signal } from '@angular/core';
import { listen, UnlistenFn } from '@tauri-apps/api/event';
import { TauriService } from './tauri.service';

const STORAGE_KEY_VALUE = 'avatar-switcher.eyeheight.value';

export const EYE_HEIGHT_DEFAULT = 1.6;
export const EYE_HEIGHT_MIN = 0.2;
export const EYE_HEIGHT_MAX = 5.0;

@Injectable({ providedIn: 'root' })
export class EyeHeightService {
  private readonly _value = signal<number>(EYE_HEIGHT_DEFAULT);
  private readonly _lastError = signal<string | null>(null);

  readonly value = this._value.asReadonly();
  readonly lastError = this._lastError.asReadonly();

  private unlisten: UnlistenFn | null = null;

  constructor(private tauri: TauriService) {
    const saved = parseFloat(localStorage.getItem(STORAGE_KEY_VALUE) ?? '');
    if (Number.isFinite(saved)) {
      this._value.set(this.clamp(saved));
    }
    // VRChat からの現在アイハイト通知を購読する。
    // サービスは providedIn: 'root' で常駐するため、別ビューにいてアバター切替された
    // 場合でも EyeHeightAsMeters を取りこぼさずに最新値を保持できる。
    listen<number>('osc:eye-height', e => this.applyExternalValue(e.payload))
      .then(un => { this.unlisten = un; })
      .catch(err => {
        this._lastError.set(`OSC受信購読失敗: ${String(err)}`);
      });
  }

  /** ユーザ操作で値をセットし VRChat へ送信する。 */
  async setValue(v: number): Promise<void> {
    if (!Number.isFinite(v)) return;
    const clamped = this.clamp(v);
    this._value.set(clamped);
    localStorage.setItem(STORAGE_KEY_VALUE, String(clamped));
    try {
      await this.tauri.invoke('osc_set_avatar_eye_height', { value: clamped });
      this._lastError.set(null);
    } catch (e) {
      this._lastError.set(`OSC 送信失敗: ${String(e)}`);
    }
  }

  /** 増減ボタン用：累積誤差防止のため 0.01 桁で丸めて送信。 */
  async adjust(delta: number): Promise<void> {
    const next = Math.round((this._value() + delta) * 100) / 100;
    await this.setValue(next);
  }

  /** VRChat から受け取った値を再送なしで反映する。 */
  private applyExternalValue(raw: unknown): void {
    const v = typeof raw === 'number' ? raw : parseFloat(String(raw));
    if (!Number.isFinite(v)) return;
    const clamped = this.clamp(v);
    this._value.set(clamped);
    localStorage.setItem(STORAGE_KEY_VALUE, String(clamped));
  }

  private clamp(v: number): number {
    return Math.max(EYE_HEIGHT_MIN, Math.min(EYE_HEIGHT_MAX, v));
  }
}
