import { Component, ElementRef, HostListener, OnInit, ViewChild, computed, signal } from '@angular/core';
import { FormsModule } from '@angular/forms';
import { NgClass } from '@angular/common';
import {
  EyeHeightService,
  EYE_HEIGHT_DEFAULT,
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
  imports: [FormsModule, NgClass],
  template: `
    <div class="p-6 h-full overflow-y-auto">
      <h2 class="m-0 mb-6 text-lg font-bold text-text">身長コントロール</h2>

      <div class="mb-6">
        <h3 class="m-0 mb-3 text-[11px] font-semibold uppercase tracking-[0.08em] text-text-tertiary">アイハイト (m)</h3>
        <div class="bg-background border border-[var(--charcoal-color-container-secondary-default)] rounded-ch-l p-4 flex flex-col gap-[14px]">
          <div class="flex items-center gap-2.5">
            <div
              class="inline-flex bg-[var(--charcoal-color-dark-neutral--5)] border border-[var(--charcoal-color-container-secondary-default)] rounded-ch-m p-0.5 gap-0.5"
              role="group"
              aria-label="送信モード"
              title="スムーズ: 補間して滑らかに変化 / 普通: 即時反映"
            >
              <button
                type="button"
                class="py-1.5 px-2.5 border-0 rounded-ch-s text-[11px] font-semibold font-[var(--font-sans)] cursor-pointer transition-all duration-100 whitespace-nowrap"
                [ngClass]="eyeHeight.mode() === 'instant' ? 'bg-primary-dim text-primary' : 'bg-transparent text-text-tertiary hover:text-text'"
                (click)="eyeHeight.setMode('instant')"
              >普通</button>
              <button
                type="button"
                class="py-1.5 px-2.5 border-0 rounded-ch-s text-[11px] font-semibold font-[var(--font-sans)] cursor-pointer transition-all duration-100 whitespace-nowrap"
                [ngClass]="eyeHeight.mode() === 'smooth' ? 'bg-primary-dim text-primary' : 'bg-transparent text-text-tertiary hover:text-text'"
                (click)="eyeHeight.setMode('smooth')"
              >スムーズ</button>
            </div>
            <input
              type="text"
              inputmode="decimal"
              class="flex-1 bg-[var(--charcoal-color-dark-neutral--5)] border border-[var(--charcoal-color-container-secondary-default)] rounded-ch-m py-2.5 px-3 text-lg font-semibold text-text font-[var(--font-mono)] outline-none transition-colors duration-150 text-right focus:border-primary"
              [value]="displayValue()"
              (change)="onValueChange($any($event.target).value)"
            />
            <span class="text-sm text-text-tertiary font-[var(--font-mono)]">m</span>
            <button
              class="py-2 px-[14px] bg-container-secondary border border-[var(--charcoal-color-container-secondary-default)] rounded-ch-m text-text-secondary text-xs font-[var(--font-sans)] cursor-pointer transition-colors duration-150 whitespace-nowrap enabled:hover:bg-[var(--charcoal-color-dark-neutral--5)] disabled:opacity-60 disabled:cursor-default"
              [disabled]="eyeHeight.fetchingDefault()"
              (click)="reset()"
              title="アバター本来の身長に戻す（取得できなければ 1.6m）"
            >{{ eyeHeight.fetchingDefault() ? '取得中…' : 'リセット' }}</button>
          </div>

          <div class="grid grid-cols-4 gap-1.5">
            @for (step of steps; track step) {
              <button class="flex-1 py-2 px-1.5 rounded-ch-m border border-[var(--charcoal-color-container-secondary-default)] bg-[var(--charcoal-color-dark-neutral--5)] text-text-secondary text-xs font-[var(--font-mono)] font-semibold cursor-pointer transition-all duration-100 hover:bg-container-secondary hover:text-text active:translate-y-px" (click)="adjust(step)">
                + {{ step.toFixed(2) }}
              </button>
            }
            @for (step of steps; track step) {
              <button class="flex-1 py-2 px-1.5 rounded-ch-m border border-[var(--charcoal-color-container-secondary-default)] bg-[var(--charcoal-color-dark-neutral--5)] text-text-secondary text-xs font-[var(--font-mono)] font-semibold cursor-pointer transition-all duration-100 hover:bg-container-secondary hover:text-text active:translate-y-px" (click)="adjust(-step)">
                − {{ step.toFixed(2) }}
              </button>
            }
          </div>

        </div>
        <p class="mt-2 py-2.5 px-[14px] bg-[rgba(245,166,35,0.08)] border border-[rgba(245,166,35,0.25)] rounded-ch-m text-[11px] text-text-notice leading-[1.6]">ワールド移動・アバター変更時は VRChat 側の身長がリセットされます。変更後は再度適用してください。</p>
      </div>

      <div class="mb-6">
        <div class="flex items-center justify-between mb-3">
          <h3 class="m-0 text-[11px] font-semibold uppercase tracking-[0.08em] text-text-tertiary">プリセット</h3>
          <button class="py-1.5 px-3 bg-primary-dim border border-primary rounded-ch-m text-primary text-xs font-semibold font-[var(--font-sans)] cursor-pointer transition-colors duration-150 hover:bg-primary hover:text-white" (click)="openSaveDialog()">
            + 現在値を保存
          </button>
        </div>
        <div class="bg-background border border-[var(--charcoal-color-container-secondary-default)] rounded-ch-l p-4 flex flex-col gap-2">
          @if (presets().length === 0) {
            <p class="m-0 text-xs text-text-placeholder text-center py-2">プリセットはまだありません。現在値を保存しましょう。</p>
          } @else {
            @for (preset of presets(); track preset.id) {
              <div class="flex gap-1.5 items-stretch">
                <button
                  class="flex-1 flex items-center justify-between py-2.5 px-3 bg-[var(--charcoal-color-dark-neutral--5)] border border-[var(--charcoal-color-container-secondary-default)] rounded-ch-m text-text font-[var(--font-sans)] cursor-pointer transition-all duration-100 hover:bg-container-secondary hover:border-primary"
                  (click)="applyPreset(preset)"
                  (contextmenu)="openContextMenu(preset, $event)"
                >
                  <span class="text-[13px] font-medium">{{ preset.name }}</span>
                  <span class="text-xs text-primary font-[var(--font-mono)] font-semibold">{{ preset.value.toFixed(2) }} m</span>
                </button>
                <button
                  class="w-9 bg-[var(--charcoal-color-dark-neutral--5)] border border-[var(--charcoal-color-container-secondary-default)] rounded-ch-m text-text-tertiary text-lg leading-none cursor-pointer transition-all duration-100 hover:bg-[rgba(245,108,108,0.1)] hover:border-[rgba(245,108,108,0.3)] hover:text-text-negative"
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
        <div class="py-3 px-4 bg-[rgba(245,108,108,0.08)] border border-[rgba(245,108,108,0.25)] rounded-ch-m text-xs text-text-negative">{{ err }}</div>
      }
      @if (eyeHeight.scalingAllowed() === false) {
        <div class="py-3 px-4 bg-[rgba(230,162,60,0.08)] border border-[rgba(230,162,60,0.25)] rounded-ch-m text-xs text-text-notice">このワールドでは身長変更が許可されていないため、VRChat側で反映されない可能性があります</div>
      }

      @if (contextMenu(); as menu) {
        <div class="fixed z-[901] bg-background border border-[var(--charcoal-color-container-secondary-default)] rounded-ch-m p-1 shadow-[0_4px_16px_rgba(0,0,0,0.35)] min-w-[120px]" [style.left.px]="menu.x" [style.top.px]="menu.y" (click)="$event.stopPropagation()">
          <button class="block w-full py-2 px-3 bg-transparent border-0 rounded-ch-s text-text text-xs font-[var(--font-sans)] text-left cursor-pointer hover:bg-container-secondary" (click)="openEditDialog(menu.preset); closeContextMenu()">編集</button>
          <button class="block w-full py-2 px-3 bg-transparent border-0 rounded-ch-s text-text-negative text-xs font-[var(--font-sans)] text-left cursor-pointer hover:bg-[rgba(245,108,108,0.1)]" (click)="deletePreset(menu.preset.id); closeContextMenu()">削除</button>
        </div>
      }

      @if (dialogOpen()) {
        <div class="fixed inset-0 bg-[rgba(0,0,0,0.55)] flex items-center justify-center z-[1000] animate-dialog-fade-in" (click)="closeDialog()">
          <div
            class="w-[min(360px,calc(100vw_-_48px))] bg-background border border-[var(--charcoal-color-container-secondary-default)] rounded-ch-l p-5 flex flex-col gap-[14px] shadow-[0_12px_32px_rgba(0,0,0,0.4)] animate-dialog-slide-in"
            role="dialog"
            aria-modal="true"
            aria-labelledby="preset-dialog-title"
            (click)="$event.stopPropagation()"
            (keydown.escape)="closeDialog()"
          >
            <h3 id="preset-dialog-title" class="m-0 text-sm font-bold text-text">{{ dialogMode() === 'edit' ? 'プリセット編集' : 'プリセット保存' }}</h3>

            <label class="flex flex-col gap-1.5">
              <span class="text-[11px] font-semibold uppercase tracking-[0.06em] text-text-tertiary">名前</span>
              <input
                #nameInput
                type="text"
                class="bg-[var(--charcoal-color-dark-neutral--5)] border border-[var(--charcoal-color-container-secondary-default)] rounded-ch-m py-[9px] px-3 text-[13px] text-text font-[var(--font-sans)] outline-none transition-colors duration-150 w-full box-border focus:border-primary"
                [ngModel]="draftName()"
                (ngModelChange)="draftName.set($event)"
                (keydown.enter)="confirmSave()"
                maxlength="40"
                spellcheck="false"
                autocomplete="off"
                placeholder="例: 標準, 小柄, 高め"
              />
            </label>

            <label class="flex flex-col gap-1.5">
              <span class="text-[11px] font-semibold uppercase tracking-[0.06em] text-text-tertiary">身長 (m)</span>
              <div class="flex items-center gap-2">
                <input
                  type="number"
                  class="bg-[var(--charcoal-color-dark-neutral--5)] border border-[var(--charcoal-color-container-secondary-default)] rounded-ch-m py-[9px] px-3 text-[13px] text-text font-[var(--font-sans)] outline-none transition-colors duration-150 w-full box-border focus:border-primary font-[var(--font-mono)] text-right font-semibold"
                  [min]="minValue()"
                  [max]="maxValue()"
                  step="0.01"
                  [ngModel]="draftValue()"
                  (ngModelChange)="draftValue.set($event)"
                  (keydown.enter)="confirmSave()"
                />
                <span class="text-sm text-text-tertiary font-[var(--font-mono)]">m</span>
              </div>
              <span class="text-[10px] text-text-placeholder">{{ minValue() }} 〜 {{ maxValue() }} m</span>
            </label>

            @if (dialogError()) {
              <p class="m-0 text-[11px] text-text-negative">{{ dialogError() }}</p>
            }

            <div class="flex gap-2 justify-end mt-1">
              <button class="py-2 px-[18px] rounded-ch-m text-xs font-semibold font-[var(--font-sans)] cursor-pointer transition-all duration-100 border border-[var(--charcoal-color-container-secondary-default)] bg-container-secondary text-text-secondary hover:bg-[var(--charcoal-color-dark-neutral--5)]" (click)="closeDialog()">
                キャンセル
              </button>
              <button class="py-2 px-[18px] rounded-ch-m text-xs font-semibold font-[var(--font-sans)] cursor-pointer transition-all duration-100 border border-primary bg-primary text-white hover:brightness-[1.08]" (click)="confirmSave()">
                保存
              </button>
            </div>
          </div>
        </div>
      }
    </div>
  `,
  styles: [`
    /* number 型のネイティブスピナー（上下矢印）を非表示。増減操作は独自のstepボタン群で行うため不要。
       属性セレクタ+ベンダープレフィックス擬似要素のためTailwindユーティリティで表現できない。 */
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
  `],
})
export class HeightViewComponent implements OnInit {
  readonly presets = signal<Preset[]>([]);
  readonly steps = STEPS;
  /** ワールド(Udon)が `/avatar/eyeheightmin` / `max` を公開していればそれを、
   *  なければアプリのデフォルト範囲を表示に使う。 */
  readonly minValue = computed(() => this.eyeHeight.worldMinHeight());
  readonly maxValue = computed(() => this.eyeHeight.worldMaxHeight());
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

  async reset(): Promise<void> {
    const target = await this.eyeHeight.getAvatarDefault();
    await this.eyeHeight.setValue(target);
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
    if (value < this.minValue() || value > this.maxValue()) {
      this.dialogError.set(`身長は ${this.minValue()} 〜 ${this.maxValue()} m の範囲で指定してください`);
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
