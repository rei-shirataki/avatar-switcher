import { Component, ElementRef, HostListener, OnInit, ViewChild, computed, signal } from '@angular/core';
import { FormsModule } from '@angular/forms';
import {
  EyeHeightService,
  EYE_HEIGHT_DEFAULT,
  EYE_HEIGHT_MAX,
  EYE_HEIGHT_MIN,
} from '../../core/services/eye-height.service';
import { coerceFiniteNumber } from '../../core/utils/number.util';

const STORAGE_KEY_PRESETS = 'avatar-switcher.eyeheight.presets';
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
            <div
              class="mode-toggle"
              role="group"
              aria-label="送信モード"
              title="スムーズ: 補間して滑らかに変化 / 普通: 即時反映"
            >
              <button
                type="button"
                class="mode-btn"
                [class.active]="eyeHeight.mode() === 'instant'"
                (click)="eyeHeight.setMode('instant')"
              >普通</button>
              <button
                type="button"
                class="mode-btn"
                [class.active]="eyeHeight.mode() === 'smooth'"
                (click)="eyeHeight.setMode('smooth')"
              >スムーズ</button>
            </div>
            <input
              type="text"
              inputmode="decimal"
              class="value-input"
              [value]="displayValue()"
              (change)="onValueChange($any($event.target).value)"
            />
            <span class="unit">m</span>
            <button
              class="btn-reset"
              (click)="reset()"
              title="アバター本来の身長に戻す（取得できなければ 1.6m）"
            >リセット</button>
          </div>

          <div class="step-grid">
            @for (step of steps; track step) {
              <button class="step-btn step-btn--plus" (click)="adjust(step)">
                + {{ step.toFixed(2) }}
              </button>
            }
            @for (step of steps; track step) {
              <button class="step-btn step-btn--minus" (click)="adjust(-step)">
                − {{ step.toFixed(2) }}
              </button>
            }
          </div>

        </div>
        <p class="info-box">ワールド移動・アバター変更時は VRChat 側の身長がリセットされます。変更後は再度適用してください。</p>
      </div>

      <div class="settings-section">
        <div class="section-head">
          <h3 class="section-title">プリセット</h3>
          <button class="btn-add-preset" (click)="openSaveDialog()">
            + 現在値を保存
          </button>
        </div>
        <div class="card preset-card">
          @if (presets().length === 0) {
            <p class="empty">プリセットはまだありません。現在値を保存しましょう。</p>
          } @else {
            @for (preset of presets(); track preset.id) {
              <div class="preset-row">
                <button
                  class="preset-apply"
                  (click)="applyPreset(preset)"
                  (contextmenu)="openContextMenu(preset, $event)"
                >
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

      @if (eyeHeight.lastError(); as err) {
        <div class="error-box">{{ err }}</div>
      }

      @if (contextMenu(); as menu) {
        <div class="ctx-menu" [style.left.px]="menu.x" [style.top.px]="menu.y" (click)="$event.stopPropagation()">
          <button class="ctx-item" (click)="openEditDialog(menu.preset); closeContextMenu()">編集</button>
          <button class="ctx-item ctx-item--danger" (click)="deletePreset(menu.preset.id); closeContextMenu()">削除</button>
        </div>
      }

      @if (dialogOpen()) {
        <div class="dialog-backdrop" (click)="closeDialog()">
          <div
            class="dialog"
            role="dialog"
            aria-modal="true"
            aria-labelledby="preset-dialog-title"
            (click)="$event.stopPropagation()"
            (keydown.escape)="closeDialog()"
          >
            <h3 id="preset-dialog-title" class="dialog-title">{{ dialogMode() === 'edit' ? 'プリセット編集' : 'プリセット保存' }}</h3>

            <label class="dialog-field">
              <span class="dialog-label">名前</span>
              <input
                #nameInput
                type="text"
                class="dialog-input"
                [ngModel]="draftName()"
                (ngModelChange)="draftName.set($event)"
                (keydown.enter)="confirmSave()"
                maxlength="40"
                spellcheck="false"
                autocomplete="off"
                placeholder="例: 標準, 小柄, 高め"
              />
            </label>

            <label class="dialog-field">
              <span class="dialog-label">身長 (m)</span>
              <div class="dialog-value-row">
                <input
                  type="number"
                  class="dialog-input dialog-input--value"
                  [min]="minValue"
                  [max]="maxValue"
                  step="0.01"
                  [ngModel]="draftValue()"
                  (ngModelChange)="draftValue.set($event)"
                  (keydown.enter)="confirmSave()"
                />
                <span class="unit">m</span>
              </div>
              <span class="dialog-hint">{{ minValue }} 〜 {{ maxValue }} m</span>
            </label>

            @if (dialogError()) {
              <p class="dialog-error">{{ dialogError() }}</p>
            }

            <div class="dialog-actions">
              <button class="dialog-btn dialog-btn--ghost" (click)="closeDialog()">
                キャンセル
              </button>
              <button class="dialog-btn dialog-btn--primary" (click)="confirmSave()">
                保存
              </button>
            </div>
          </div>
        </div>
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

    /* number 型のネイティブスピナー（上下矢印）を非表示。
       増減操作は独自の step ボタン群で行うため不要。 */
    input[type="number"] {
      -moz-appearance: textfield;
      appearance: textfield;
    }
    input[type="number"]::-webkit-inner-spin-button,
    input[type="number"]::-webkit-outer-spin-button {
      -webkit-appearance: none;
      appearance: none;
      margin: 0;
    }

    /* 送信モード切替トグル（普通 / スムーズ） */
    .mode-toggle {
      display: inline-flex;
      background: var(--color-surface-1);
      border: 1px solid var(--color-surface-3);
      border-radius: var(--radius-md);
      padding: 2px;
      gap: 2px;
    }
    .mode-btn {
      padding: 6px 10px;
      background: transparent;
      border: none;
      border-radius: calc(var(--radius-md) - 2px);
      color: var(--color-text-3);
      font-size: 11px;
      font-weight: 600;
      font-family: var(--font-sans);
      cursor: pointer;
      transition: all 0.12s;
      white-space: nowrap;
    }
    .mode-btn:hover { color: var(--color-text-1); }
    .mode-btn.active {
      background: var(--color-primary);
      color: var(--color-bg, #fff);
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
      grid-template-columns: repeat(4, 1fr);
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

    .info-box {
      margin: 8px 0 0;
      padding: 10px 14px;
      background: rgba(245, 166, 35, 0.08);
      border: 1px solid rgba(245, 166, 35, 0.25);
      border-radius: var(--radius-md);
      font-size: 11px;
      color: var(--color-warning);
      line-height: 1.6;
    }

    .error-box {
      padding: 12px 16px;
      background: rgba(245, 108, 108, 0.08);
      border: 1px solid rgba(245, 108, 108, 0.25);
      border-radius: var(--radius-md);
      font-size: 12px;
      color: var(--color-error);
    }

    /* ---- Custom modal dialog ---- */
    .dialog-backdrop {
      position: fixed;
      inset: 0;
      background: rgba(0, 0, 0, 0.55);
      display: flex;
      align-items: center;
      justify-content: center;
      z-index: 1000;
      animation: dialog-fade-in 0.12s ease-out;
    }
    @keyframes dialog-fade-in {
      from { opacity: 0; }
      to { opacity: 1; }
    }
    .dialog {
      width: min(360px, calc(100vw - 48px));
      background: var(--color-surface-2);
      border: 1px solid var(--color-surface-3);
      border-radius: var(--radius-lg);
      padding: 20px;
      display: flex;
      flex-direction: column;
      gap: 14px;
      box-shadow: 0 12px 32px rgba(0, 0, 0, 0.4);
      animation: dialog-slide-in 0.15s ease-out;
    }
    @keyframes dialog-slide-in {
      from { transform: translateY(8px); opacity: 0; }
      to { transform: translateY(0); opacity: 1; }
    }
    .dialog-title {
      margin: 0;
      font-size: 14px;
      font-weight: 700;
      color: var(--color-text-1);
    }
    .dialog-field {
      display: flex;
      flex-direction: column;
      gap: 6px;
    }
    .dialog-label {
      font-size: 11px;
      font-weight: 600;
      text-transform: uppercase;
      letter-spacing: 0.06em;
      color: var(--color-text-3);
    }
    .dialog-input {
      background: var(--color-surface-1);
      border: 1px solid var(--color-surface-3);
      border-radius: var(--radius-md);
      padding: 9px 12px;
      font-size: 13px;
      color: var(--color-text-1);
      font-family: var(--font-sans);
      outline: none;
      transition: border-color 0.15s;
      width: 100%;
      box-sizing: border-box;
    }
    .dialog-input:focus { border-color: var(--color-primary); }
    .dialog-input--value {
      font-family: var(--font-mono);
      text-align: right;
      font-weight: 600;
    }
    .dialog-value-row {
      display: flex;
      align-items: center;
      gap: 8px;
    }
    .dialog-hint {
      font-size: 10px;
      color: var(--color-text-4);
    }
    .dialog-error {
      margin: 0;
      font-size: 11px;
      color: var(--color-error);
    }
    .dialog-actions {
      display: flex;
      gap: 8px;
      justify-content: flex-end;
      margin-top: 4px;
    }
    .dialog-btn {
      padding: 8px 18px;
      border-radius: var(--radius-md);
      font-size: 12px;
      font-weight: 600;
      font-family: var(--font-sans);
      cursor: pointer;
      transition: all 0.12s;
      border: 1px solid var(--color-surface-3);
    }
    .dialog-btn--ghost {
      background: var(--color-surface-3);
      color: var(--color-text-2);
    }
    .dialog-btn--ghost:hover { background: var(--color-surface-1); }
    .dialog-btn--primary {
      background: var(--color-primary);
      border-color: var(--color-primary);
      color: var(--color-bg, #fff);
    }
    .dialog-btn--primary:hover { filter: brightness(1.08); }

    /* ---- Context menu ---- */
    .ctx-menu {
      position: fixed;
      z-index: 901;
      background: var(--color-surface-2);
      border: 1px solid var(--color-surface-3);
      border-radius: var(--radius-md);
      padding: 4px;
      box-shadow: 0 4px 16px rgba(0, 0, 0, 0.35);
      min-width: 120px;
    }
    .ctx-item {
      display: block;
      width: 100%;
      padding: 8px 12px;
      background: transparent;
      border: none;
      border-radius: calc(var(--radius-md) - 4px);
      color: var(--color-text-1);
      font-size: 12px;
      font-family: var(--font-sans);
      text-align: left;
      cursor: pointer;
      &:hover { background: var(--color-surface-3); }
    }
    .ctx-item--danger {
      color: var(--color-error);
      &:hover { background: rgba(245, 108, 108, 0.1); }
    }
  `],
})
export class HeightViewComponent implements OnInit {
  readonly presets = signal<Preset[]>([]);
  readonly steps = STEPS;
  readonly minValue = EYE_HEIGHT_MIN;
  readonly maxValue = EYE_HEIGHT_MAX;
  /** 入力欄表示用：常に 2 桁固定。
   *  スムーズ補間中は中間値ではなく目標値（target）を表示することで、
   *  「これからどこへ動くか」がユーザに明確に伝わるようにする。 */
  readonly displayValue = computed(() => {
    const v = this.eyeHeight.isSmoothing() ? this.eyeHeight.target() : this.eyeHeight.value();
    return v.toFixed(2);
  });

  // ---- Context menu state ----
  readonly contextMenu = signal<{ x: number; y: number; preset: Preset } | null>(null);

  // ---- Dialog state ----
  readonly dialogOpen = signal(false);
  readonly dialogMode = signal<'save' | 'edit'>('save');
  readonly editingPresetId = signal<string | null>(null);
  readonly draftName = signal('');
  readonly draftValue = signal<number>(EYE_HEIGHT_DEFAULT);
  readonly dialogError = signal<string | null>(null);

  @ViewChild('nameInput') nameInput?: ElementRef<HTMLInputElement>;

  constructor(public eyeHeight: EyeHeightService) {}

  ngOnInit(): void {
    this.loadPresets();
  }

  onValueChange(raw: number | string): void {
    const v = coerceFiniteNumber(raw);
    if (v === null) return;
    this.eyeHeight.setValue(v);
  }

  adjust(delta: number): void {
    this.eyeHeight.adjust(delta);
  }

  reset(): void {
    this.eyeHeight.setValue(this.eyeHeight.getAvatarDefault());
  }

  applyPreset(preset: Preset): void {
    this.eyeHeight.setValue(preset.value);
  }

  openSaveDialog(): void {
    const current = this.eyeHeight.value();
    this.dialogMode.set('save');
    this.editingPresetId.set(null);
    this.draftName.set(`${current.toFixed(2)} m`);
    this.draftValue.set(current);
    this.dialogError.set(null);
    this.dialogOpen.set(true);
    queueMicrotask(() => {
      const el = this.nameInput?.nativeElement;
      if (el) { el.focus(); el.select(); }
    });
  }

  @HostListener('document:click')
  onDocumentClick(): void {
    this.contextMenu.set(null);
  }

  /** ESC で開いているコンテキストメニューを閉じる（ダイアログとの一貫性のため）。 */
  @HostListener('document:keydown.escape')
  onDocumentEscape(): void {
    if (this.contextMenu()) this.contextMenu.set(null);
  }

  /** スクロール / ホイールでもメニューを閉じる。position:fixed のため
   *  プリセット側だけスクロールしてメニューだけ空中に残るのを防ぐ。 */
  @HostListener('window:scroll')
  @HostListener('window:wheel')
  onScrollOrWheel(): void {
    if (this.contextMenu()) this.contextMenu.set(null);
  }

  /** 右クリック位置がビューポート右端・下端を超えないように clamp する。
   *  メニューサイズは概算（min-width 120 / 高さ ~80）。実測値に厳密に合わせなくても、
   *  画面外にはみ出さないことが目的。 */
  openContextMenu(preset: Preset, event: MouseEvent): void {
    event.preventDefault();
    const MENU_W = 140;
    const MENU_H = 88;
    const MARGIN = 4;
    const maxX = window.innerWidth - MENU_W - MARGIN;
    const maxY = window.innerHeight - MENU_H - MARGIN;
    const x = Math.max(MARGIN, Math.min(event.clientX, maxX));
    const y = Math.max(MARGIN, Math.min(event.clientY, maxY));
    this.contextMenu.set({ x, y, preset });
  }

  closeContextMenu(): void {
    this.contextMenu.set(null);
  }

  openEditDialog(preset: Preset): void {
    this.dialogMode.set('edit');
    this.editingPresetId.set(preset.id);
    this.draftName.set(preset.name);
    this.draftValue.set(preset.value);
    this.dialogError.set(null);
    this.dialogOpen.set(true);
    queueMicrotask(() => {
      const el = this.nameInput?.nativeElement;
      if (el) { el.focus(); el.select(); }
    });
  }

  closeDialog(): void {
    this.dialogOpen.set(false);
    this.editingPresetId.set(null);
  }

  confirmSave(): void {
    const name = this.draftName().trim();
    if (!name) {
      this.dialogError.set('名前を入力してください');
      return;
    }
    const value = coerceFiniteNumber(this.draftValue());
    if (value === null) {
      this.dialogError.set('身長は数値で入力してください');
      return;
    }
    if (value < this.minValue || value > this.maxValue) {
      this.dialogError.set(`身長は ${this.minValue} 〜 ${this.maxValue} m の範囲で指定してください`);
      return;
    }
    const editId = this.editingPresetId();
    let next: Preset[];
    if (editId) {
      next = this.presets().map(p =>
        p.id === editId ? { ...p, name: name.slice(0, 40), value } : p,
      );
    } else {
      next = [...this.presets(), { id: this.generateId(), name: name.slice(0, 40), value }];
    }
    this.presets.set(next);
    this.savePresets(next);
    this.dialogOpen.set(false);
    this.editingPresetId.set(null);
  }

  deletePreset(id: string): void {
    const next = this.presets().filter(p => p.id !== id);
    this.presets.set(next);
    this.savePresets(next);
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
