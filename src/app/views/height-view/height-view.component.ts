import { Component, OnDestroy, OnInit, signal } from '@angular/core';
import { FormsModule } from '@angular/forms';
import { TauriService } from '../../core/services/tauri.service';

const STORAGE_KEY_NAME = 'avatar-switcher.height.paramName';
const STORAGE_KEY_VALUE = 'avatar-switcher.height.value';
const STORAGE_KEY_MIN = 'avatar-switcher.height.min';
const STORAGE_KEY_MAX = 'avatar-switcher.height.max';

const DEFAULT_PARAM_NAME = 'Height';
const DEFAULT_VALUE = 0;
const DEFAULT_MIN = -1;
const DEFAULT_MAX = 1;

const PARAM_NAME_PATTERN = /^[A-Za-z0-9_-]{1,64}$/;
const DEBOUNCE_MS = 30;

@Component({
  selector: 'app-height-view',
  standalone: true,
  imports: [FormsModule],
  template: `
    <div class="height-view">
      <h2 class="page-title">身長コントロール</h2>

      <div class="settings-section">
        <h3 class="section-title">OSC パラメータ</h3>
        <div class="card">
          <label class="row row--column">
            <span class="label">パラメータ名</span>
            <input
              type="text"
              class="text-input"
              [ngModel]="paramName()"
              (ngModelChange)="onParamNameChange($event)"
              placeholder="Height"
              spellcheck="false"
              autocomplete="off"
            />
            <p class="hint">
              VRChat に <code>/avatar/parameters/{{ paramName() || 'Height' }}</code>
              として Float 値を送信します。アバター側に同名 Float パラメータがあると反映されます。
            </p>
            @if (paramNameError()) {
              <p class="error">{{ paramNameError() }}</p>
            }
          </label>
        </div>
      </div>

      <div class="settings-section">
        <h3 class="section-title">値</h3>
        <div class="card">
          <div class="row">
            <span class="label">
              現在値 <span class="value">{{ value().toFixed(2) }}</span>
            </span>
            <button class="btn-reset" (click)="reset()" [disabled]="!canSend()">
              リセット (0)
            </button>
          </div>

          <input
            type="range"
            class="slider"
            [min]="min()"
            [max]="max()"
            step="0.01"
            [ngModel]="value()"
            (ngModelChange)="onValueChange($event)"
            [disabled]="!canSend()"
          />

          <div class="range-row">
            <label class="range-field">
              <span class="range-label">最小</span>
              <input
                type="number"
                class="number-input"
                step="0.1"
                [ngModel]="min()"
                (ngModelChange)="onMinChange($event)"
              />
            </label>
            <label class="range-field">
              <span class="range-label">最大</span>
              <input
                type="number"
                class="number-input"
                step="0.1"
                [ngModel]="max()"
                (ngModelChange)="onMaxChange($event)"
              />
            </label>
          </div>
        </div>
      </div>

      @if (lastError()) {
        <div class="error-box">{{ lastError() }}</div>
      }
    </div>
  `,
  styles: [`
    .height-view { padding: 24px; height: 100%; overflow-y: auto; }
    .page-title {
      margin: 0 0 24px;
      font-size: 18px;
      font-weight: 700;
      color: var(--color-text-1);
    }
    .settings-section { margin-bottom: 24px; }
    .section-title {
      margin: 0 0 12px;
      font-size: 11px;
      font-weight: 600;
      text-transform: uppercase;
      letter-spacing: 0.08em;
      color: var(--color-text-3);
    }
    .card {
      background: var(--color-surface-2);
      border: 1px solid var(--color-surface-3);
      border-radius: var(--radius-lg);
      padding: 16px;
      display: flex;
      flex-direction: column;
      gap: 12px;
    }
    .row {
      display: flex;
      align-items: center;
      justify-content: space-between;
      gap: 12px;
    }
    .row--column { flex-direction: column; align-items: stretch; gap: 8px; }
    .label { font-size: 13px; color: var(--color-text-2); }
    .value {
      color: var(--color-primary);
      font-weight: 600;
      font-family: var(--font-mono);
      margin-left: 6px;
    }
    .hint { margin: 0; font-size: 11px; color: var(--color-text-4); }
    .hint code {
      font-family: var(--font-mono);
      background: var(--color-surface-3);
      padding: 1px 5px;
      border-radius: var(--radius-sm);
    }
    .error { margin: 0; font-size: 11px; color: var(--color-error); }
    .error-box {
      padding: 12px 16px;
      background: rgba(245, 108, 108, 0.08);
      border: 1px solid rgba(245, 108, 108, 0.25);
      border-radius: var(--radius-md);
      font-size: 12px;
      color: var(--color-error);
    }
    .text-input, .number-input {
      background: var(--color-surface-1);
      border: 1px solid var(--color-surface-3);
      border-radius: var(--radius-md);
      padding: 8px 10px;
      font-size: 13px;
      color: var(--color-text-1);
      font-family: var(--font-mono);
      outline: none;
      transition: border-color 0.15s;
    }
    .text-input:focus, .number-input:focus { border-color: var(--color-primary); }
    .slider {
      width: 100%;
      accent-color: var(--color-primary);
      cursor: pointer;
    }
    .slider:disabled { cursor: not-allowed; opacity: 0.5; }
    .range-row { display: flex; gap: 12px; }
    .range-field {
      flex: 1;
      display: flex;
      flex-direction: column;
      gap: 4px;
    }
    .range-label {
      font-size: 11px;
      color: var(--color-text-3);
    }
    .btn-reset {
      padding: 6px 14px;
      background: var(--color-surface-3);
      border: 1px solid var(--color-surface-3);
      border-radius: var(--radius-md);
      color: var(--color-text-2);
      font-size: 12px;
      font-family: var(--font-sans);
      cursor: pointer;
      transition: background 0.15s;
      white-space: nowrap;
    }
    .btn-reset:hover:not(:disabled) { background: var(--color-surface-1); }
    .btn-reset:disabled { cursor: not-allowed; opacity: 0.5; }
  `],
})
export class HeightViewComponent implements OnInit, OnDestroy {
  readonly paramName = signal<string>(DEFAULT_PARAM_NAME);
  readonly value = signal<number>(DEFAULT_VALUE);
  readonly min = signal<number>(DEFAULT_MIN);
  readonly max = signal<number>(DEFAULT_MAX);
  readonly paramNameError = signal<string | null>(null);
  readonly lastError = signal<string | null>(null);

  private debounceTimer: ReturnType<typeof setTimeout> | null = null;
  private pendingValue: number | null = null;

  constructor(private tauri: TauriService) {}

  ngOnInit(): void {
    const savedName = localStorage.getItem(STORAGE_KEY_NAME);
    if (savedName && PARAM_NAME_PATTERN.test(savedName)) {
      this.paramName.set(savedName);
    }
    const savedValue = parseFloat(localStorage.getItem(STORAGE_KEY_VALUE) ?? '');
    if (Number.isFinite(savedValue)) this.value.set(savedValue);

    const savedMin = parseFloat(localStorage.getItem(STORAGE_KEY_MIN) ?? '');
    if (Number.isFinite(savedMin)) this.min.set(savedMin);
    const savedMax = parseFloat(localStorage.getItem(STORAGE_KEY_MAX) ?? '');
    if (Number.isFinite(savedMax)) this.max.set(savedMax);

    if (this.min() >= this.max()) {
      this.min.set(DEFAULT_MIN);
      this.max.set(DEFAULT_MAX);
    }
    this.value.set(this.clampValue(this.value()));
  }

  ngOnDestroy(): void {
    // 残った debounce があれば即時 flush して値を送り切る
    if (this.debounceTimer !== null) {
      clearTimeout(this.debounceTimer);
      this.debounceTimer = null;
      if (this.pendingValue !== null) this.sendOsc(this.pendingValue);
    }
  }

  canSend(): boolean {
    return PARAM_NAME_PATTERN.test(this.paramName());
  }

  onParamNameChange(name: string): void {
    this.paramName.set(name);
    if (!name) {
      this.paramNameError.set('パラメータ名を入力してください');
      return;
    }
    if (!PARAM_NAME_PATTERN.test(name)) {
      this.paramNameError.set('英数字・アンダースコア・ハイフンのみ、1〜64 文字');
      return;
    }
    this.paramNameError.set(null);
    localStorage.setItem(STORAGE_KEY_NAME, name);
  }

  onValueChange(raw: number | string): void {
    const v = typeof raw === 'number' ? raw : parseFloat(raw);
    if (!Number.isFinite(v)) return;
    const clamped = this.clampValue(v);
    this.value.set(clamped);
    localStorage.setItem(STORAGE_KEY_VALUE, String(clamped));
    this.scheduleSend(clamped);
  }

  onMinChange(raw: number | string): void {
    const v = typeof raw === 'number' ? raw : parseFloat(raw);
    if (!Number.isFinite(v)) return;
    if (v >= this.max()) return;
    this.min.set(v);
    localStorage.setItem(STORAGE_KEY_MIN, String(v));
    this.value.set(this.clampValue(this.value()));
  }

  onMaxChange(raw: number | string): void {
    const v = typeof raw === 'number' ? raw : parseFloat(raw);
    if (!Number.isFinite(v)) return;
    if (v <= this.min()) return;
    this.max.set(v);
    localStorage.setItem(STORAGE_KEY_MAX, String(v));
    this.value.set(this.clampValue(this.value()));
  }

  reset(): void {
    this.value.set(0);
    localStorage.setItem(STORAGE_KEY_VALUE, '0');
    this.scheduleSend(0);
  }

  private clampValue(v: number): number {
    return Math.max(this.min(), Math.min(this.max(), v));
  }

  private scheduleSend(v: number): void {
    if (!this.canSend()) return;
    this.pendingValue = v;
    if (this.debounceTimer !== null) clearTimeout(this.debounceTimer);
    this.debounceTimer = setTimeout(() => {
      this.debounceTimer = null;
      const value = this.pendingValue;
      this.pendingValue = null;
      if (value !== null) this.sendOsc(value);
    }, DEBOUNCE_MS);
  }

  private async sendOsc(value: number): Promise<void> {
    try {
      await this.tauri.invoke('osc_set_avatar_parameter_float', {
        name: this.paramName(),
        value,
      });
      this.lastError.set(null);
    } catch (e) {
      this.lastError.set(`OSC 送信失敗: ${String(e)}`);
    }
  }
}
