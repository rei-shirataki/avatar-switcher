import { Component, OnInit, signal } from '@angular/core';
import { FormsModule } from '@angular/forms';
import { TauriService } from '../../core/services/tauri.service';

const STORAGE_KEY_VALUE = 'avatar-switcher.eyeheight.value';
const STORAGE_KEY_PRESETS = 'avatar-switcher.eyeheight.presets';

const DEFAULT_VALUE = 1.6;
const MIN_VALUE = 0.2;
const MAX_VALUE = 5.0;
const STEPS = [0.01, 0.1, 0.5, 1.0] as const;

interface Preset {
  id: string;
  name: string;
  value: number;
}

@Component({
  selector: 'app-height-view',
  standalone: true,
  imports: [FormsModule],
  template: `
    <div class="height-view">
      <h2 class="page-title">身長コントロール</h2>

      <div class="settings-section">
        <h3 class="section-title">アイハイト (m)</h3>
        <div class="card">
          <div class="value-row">
            <input
              type="number"
              class="value-input"
              [min]="minValue"
              [max]="maxValue"
              step="0.01"
              [ngModel]="value()"
              (ngModelChange)="onValueChange($event)"
            />
            <span class="unit">m</span>
            <button class="btn-reset" (click)="reset()">リセット</button>
          </div>

          <div class="step-grid">
            @for (step of steps; track step) {
              <div class="step-pair">
                <button class="step-btn step-btn--minus" (click)="adjust(-step)">
                  − {{ step.toFixed(2) }}
                </button>
                <button class="step-btn step-btn--plus" (click)="adjust(step)">
                  + {{ step.toFixed(2) }}
                </button>
              </div>
            }
          </div>

          <p class="hint">
            <code>/avatar/eyeheight</code> に Float (m) を送信します。
            範囲: {{ minValue }} 〜 {{ maxValue }} m
          </p>
        </div>
      </div>

      <div class="settings-section">
        <div class="section-head">
          <h3 class="section-title">プリセット</h3>
          <button class="btn-add-preset" (click)="addPreset()">
            + 現在値を保存
          </button>
        </div>
        <div class="card preset-card">
          @if (presets().length === 0) {
            <p class="empty">プリセットはまだありません。現在値を保存しましょう。</p>
          } @else {
            @for (preset of presets(); track preset.id) {
              <div class="preset-row">
                <button class="preset-apply" (click)="applyPreset(preset)">
                  <span class="preset-name">{{ preset.name }}</span>
                  <span class="preset-value">{{ preset.value.toFixed(2) }} m</span>
                </button>
                <button
                  class="preset-delete"
                  (click)="deletePreset(preset.id)"
                  title="削除"
                  aria-label="削除"
                >×</button>
              </div>
            }
          }
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
    .section-head {
      display: flex;
      align-items: center;
      justify-content: space-between;
      margin-bottom: 12px;
    }
    .section-title {
      margin: 0 0 12px;
      font-size: 11px;
      font-weight: 600;
      text-transform: uppercase;
      letter-spacing: 0.08em;
      color: var(--color-text-3);
    }
    .section-head .section-title { margin: 0; }
    .card {
      background: var(--color-surface-2);
      border: 1px solid var(--color-surface-3);
      border-radius: var(--radius-lg);
      padding: 16px;
      display: flex;
      flex-direction: column;
      gap: 14px;
    }
    .value-row {
      display: flex;
      align-items: center;
      gap: 10px;
    }
    .value-input {
      flex: 1;
      background: var(--color-surface-1);
      border: 1px solid var(--color-surface-3);
      border-radius: var(--radius-md);
      padding: 10px 12px;
      font-size: 18px;
      font-weight: 600;
      color: var(--color-text-1);
      font-family: var(--font-mono);
      outline: none;
      transition: border-color 0.15s;
      text-align: right;
    }
    .value-input:focus { border-color: var(--color-primary); }
    .unit {
      font-size: 14px;
      color: var(--color-text-3);
      font-family: var(--font-mono);
    }
    .btn-reset {
      padding: 8px 14px;
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
    .btn-reset:hover { background: var(--color-surface-1); }

    .step-grid {
      display: grid;
      grid-template-columns: repeat(2, 1fr);
      gap: 8px;
    }
    .step-pair {
      display: flex;
      gap: 6px;
    }
    .step-btn {
      flex: 1;
      padding: 8px 6px;
      border-radius: var(--radius-md);
      border: 1px solid var(--color-surface-3);
      background: var(--color-surface-1);
      color: var(--color-text-2);
      font-size: 12px;
      font-family: var(--font-mono);
      font-weight: 600;
      cursor: pointer;
      transition: all 0.12s;
    }
    .step-btn:hover {
      background: var(--color-surface-3);
      color: var(--color-text-1);
    }
    .step-btn:active { transform: translateY(1px); }

    .hint { margin: 0; font-size: 11px; color: var(--color-text-4); }
    .hint code {
      font-family: var(--font-mono);
      background: var(--color-surface-3);
      padding: 1px 5px;
      border-radius: var(--radius-sm);
    }

    .btn-add-preset {
      padding: 6px 12px;
      background: var(--color-primary-dim);
      border: 1px solid var(--color-primary);
      border-radius: var(--radius-md);
      color: var(--color-primary);
      font-size: 12px;
      font-weight: 600;
      font-family: var(--font-sans);
      cursor: pointer;
      transition: background 0.15s;
    }
    .btn-add-preset:hover { background: var(--color-primary); color: var(--color-bg, #fff); }

    .preset-card { gap: 8px; }
    .empty {
      margin: 0;
      font-size: 12px;
      color: var(--color-text-4);
      text-align: center;
      padding: 8px 0;
    }
    .preset-row {
      display: flex;
      gap: 6px;
      align-items: stretch;
    }
    .preset-apply {
      flex: 1;
      display: flex;
      align-items: center;
      justify-content: space-between;
      padding: 10px 12px;
      background: var(--color-surface-1);
      border: 1px solid var(--color-surface-3);
      border-radius: var(--radius-md);
      color: var(--color-text-1);
      font-family: var(--font-sans);
      cursor: pointer;
      transition: all 0.12s;
    }
    .preset-apply:hover {
      background: var(--color-surface-3);
      border-color: var(--color-primary);
    }
    .preset-name { font-size: 13px; font-weight: 500; }
    .preset-value {
      font-size: 12px;
      color: var(--color-primary);
      font-family: var(--font-mono);
      font-weight: 600;
    }
    .preset-delete {
      width: 36px;
      background: var(--color-surface-1);
      border: 1px solid var(--color-surface-3);
      border-radius: var(--radius-md);
      color: var(--color-text-3);
      font-size: 18px;
      line-height: 1;
      cursor: pointer;
      transition: all 0.12s;
    }
    .preset-delete:hover {
      background: rgba(245, 108, 108, 0.1);
      border-color: rgba(245, 108, 108, 0.3);
      color: var(--color-error);
    }

    .error-box {
      padding: 12px 16px;
      background: rgba(245, 108, 108, 0.08);
      border: 1px solid rgba(245, 108, 108, 0.25);
      border-radius: var(--radius-md);
      font-size: 12px;
      color: var(--color-error);
    }
  `],
})
export class HeightViewComponent implements OnInit {
  readonly value = signal<number>(DEFAULT_VALUE);
  readonly presets = signal<Preset[]>([]);
  readonly lastError = signal<string | null>(null);

  readonly steps = STEPS;
  readonly minValue = MIN_VALUE;
  readonly maxValue = MAX_VALUE;

  constructor(private tauri: TauriService) {}

  ngOnInit(): void {
    const savedValue = parseFloat(localStorage.getItem(STORAGE_KEY_VALUE) ?? '');
    if (Number.isFinite(savedValue)) {
      this.value.set(this.clamp(savedValue));
    }
    this.loadPresets();
  }

  onValueChange(raw: number | string): void {
    const v = typeof raw === 'number' ? raw : parseFloat(raw);
    if (!Number.isFinite(v)) return;
    this.setValue(v);
  }

  adjust(delta: number): void {
    // 浮動小数の累積誤差を抑えるため小数点 2 桁で丸める
    const next = Math.round((this.value() + delta) * 100) / 100;
    this.setValue(next);
  }

  reset(): void {
    this.setValue(DEFAULT_VALUE);
  }

  applyPreset(preset: Preset): void {
    this.setValue(preset.value);
  }

  addPreset(): void {
    const current = this.value();
    const name = prompt(
      `プリセット名を入力 (現在値: ${current.toFixed(2)} m)`,
      `${current.toFixed(2)} m`,
    );
    if (name === null) return;
    const trimmed = name.trim();
    if (!trimmed) return;
    const preset: Preset = {
      id: this.generateId(),
      name: trimmed.slice(0, 40),
      value: current,
    };
    const next = [...this.presets(), preset];
    this.presets.set(next);
    this.savePresets(next);
  }

  deletePreset(id: string): void {
    const next = this.presets().filter(p => p.id !== id);
    this.presets.set(next);
    this.savePresets(next);
  }

  private setValue(v: number): void {
    const clamped = this.clamp(v);
    this.value.set(clamped);
    localStorage.setItem(STORAGE_KEY_VALUE, String(clamped));
    this.sendOsc(clamped);
  }

  private clamp(v: number): number {
    return Math.max(MIN_VALUE, Math.min(MAX_VALUE, v));
  }

  private async sendOsc(value: number): Promise<void> {
    try {
      await this.tauri.invoke('osc_set_avatar_eye_height', { value });
      this.lastError.set(null);
    } catch (e) {
      this.lastError.set(`OSC 送信失敗: ${String(e)}`);
    }
  }

  private loadPresets(): void {
    const raw = localStorage.getItem(STORAGE_KEY_PRESETS);
    if (!raw) return;
    try {
      const parsed = JSON.parse(raw);
      if (!Array.isArray(parsed)) return;
      const valid = parsed.filter(
        (p): p is Preset =>
          p && typeof p === 'object'
          && typeof p.id === 'string'
          && typeof p.name === 'string'
          && typeof p.value === 'number'
          && Number.isFinite(p.value),
      );
      this.presets.set(valid);
    } catch {
      // 破損データは無視
    }
  }

  private savePresets(presets: Preset[]): void {
    localStorage.setItem(STORAGE_KEY_PRESETS, JSON.stringify(presets));
  }

  private generateId(): string {
    return `${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;
  }
}
