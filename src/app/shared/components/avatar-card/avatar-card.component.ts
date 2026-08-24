import { Component, CUSTOM_ELEMENTS_SCHEMA, Input, Output, EventEmitter, signal, HostListener } from '@angular/core';
import { VRCAvatar, AvatarFolder } from '../../../core/models/avatar.model';
import { CommonModule } from '@angular/common';
import { IconComponent } from '../icon/icon.component';

export interface CardContextMenuEvent {
  avatar: VRCAvatar;
  x: number;
  y: number;
}

@Component({
  selector: 'app-avatar-card',
  standalone: true,
  imports: [CommonModule, IconComponent],
  template: `
    <div
      class="flex flex-col bg-background border border-[var(--charcoal-color-container-secondary-default)] rounded-m transition-[border-color,transform,box-shadow] duration-150 hover:border-primary-dim hover:shadow-[0_4px_16px_rgba(0,0,0,0.3)]"
      [ngClass]="{
        'opacity-70 pointer-events-none': isSwitching,
        'border-primary shadow-[0_0_0_1px_var(--color-primary)]': selected,
        'cursor-pointer': selectionMode,
        'cursor-default': !selectionMode,
        'hover:-translate-y-0.5': !selectionMode
      }"
      (mouseenter)="hovered.set(true)"
      (mouseleave)="hovered.set(false)"
      (click)="selectionMode ? selectToggle.emit() : openDetail.emit(avatar)"
      (contextmenu)="onCardContextMenu($event)"
    >
      <div class="relative w-full aspect-square bg-[var(--charcoal-color-container-secondary-default)] overflow-hidden rounded-t-m">
        @if (avatar.thumbnailImageUrl) {
          <img
            class="w-full h-full object-cover"
            [src]="avatar.thumbnailImageUrl"
            [alt]="avatar.name"
            loading="lazy"
            (error)="onImgError($event)"
          />
        } @else {
          <div class="w-full h-full flex items-center justify-center text-4xl font-bold text-text-tertiary bg-container-tertiary">
            {{ avatar.name.charAt(0) }}
          </div>
        }
        @if (selectionMode) {
          <div
            class="absolute top-1.5 left-1.5 w-[18px] h-[18px] border-2 border-[rgba(255,255,255,0.5)] rounded-full bg-[rgba(0,0,0,0.35)] flex items-center justify-center text-[10px] text-white font-bold transition-[background,border-color] duration-100"
            [ngClass]="{ 'bg-primary border-primary': selected }"
          >
            @if (selected) { <span>✓</span> }
          </div>
        }
        @if (!selectionMode && hovered() && !isSwitching) {
          <div class="absolute inset-0 bg-[rgba(0,0,0,0.6)] flex items-center justify-center animate-fade-in" (click)="onSwitch(); $event.stopPropagation()">
            <span class="px-[14px] py-1.5 bg-primary text-white rounded-m text-xs font-semibold cursor-pointer transition-colors duration-150 hover:bg-primary-hover"><app-icon name="play" [size]="11"/> 切り替え</span>
          </div>
        }
        @if (isSwitching) {
          <div class="absolute inset-0 bg-[rgba(0,0,0,0.5)] flex items-center justify-center animate-fade-in">
            <app-icon class="text-primary animate-spin" name="spinner" [size]="24"/>
          </div>
        }
      </div>
      <div class="py-2 px-2.5 flex flex-col gap-0.5">
        <div class="text-xs font-semibold text-text truncate" [title]="avatar.name">{{ avatar.name }}</div>
        <div class="text-[11px] text-text-secondary truncate">{{ avatar.authorName }}</div>
        <div class="flex items-center justify-between mt-0.5">
          <div
            class="text-[10px] px-1.5 py-0.5 rounded-[3px] self-start font-medium"
            [ngClass]="avatar.releaseStatus === 'public' ? 'bg-[rgba(86,201,110,0.15)] text-text-positive' : 'bg-[rgba(136,136,170,0.15)] text-text-tertiary'"
          >
            {{ avatar.releaseStatus === 'public' ? '公開' : '非公開' }}
          </div>
          @if (folders.length > 0 && !selectionMode) {
            <div class="relative">
              <button
                class="w-5 h-5 bg-container-secondary border border-[var(--charcoal-color-container-secondary-default)] text-text-secondary cursor-pointer p-0 flex items-center justify-center rounded transition-colors duration-150 hover:bg-primary-dim hover:border-primary hover:text-primary"
                [ngClass]="{ 'bg-primary-dim border-primary text-primary': isInAnyFolder() }"
                (click)="showFolderMenu.set(!showFolderMenu()); $event.stopPropagation()"
                title="フォルダ管理"
              ><app-icon name="folder-plus" [size]="13"/></button>
              @if (showFolderMenu()) {
                <div class="absolute bottom-[calc(100%+4px)] right-0 min-w-[140px] bg-background border border-[var(--charcoal-color-container-secondary-default)] rounded-m shadow-[0_4px_16px_rgba(0,0,0,0.4)] z-50 overflow-hidden" (click)="$event.stopPropagation()">
                  @for (folder of folders; track folder.id) {
                    <button
                      class="flex items-center gap-1.5 w-full py-[7px] px-2.5 bg-transparent border-0 text-text-secondary text-[11px] font-[var(--font-sans)] cursor-pointer text-left transition-colors duration-100 hover:bg-container-secondary"
                      [ngClass]="{ 'text-primary': folder.avatarIds.includes(avatar.id) }"
                      (click)="toggleFolder(folder.id)"
                    >
                      <span class="w-3 flex-shrink-0 text-primary text-[10px]">
                        {{ folder.avatarIds.includes(avatar.id) ? '✓' : '' }}
                      </span>
                      {{ folder.name }}
                    </button>
                  }
                </div>
              }
            </div>
          }
        </div>
      </div>
    </div>
  `,
  schemas: [CUSTOM_ELEMENTS_SCHEMA],
})
export class AvatarCardComponent {
  @Input({ required: true }) avatar!: VRCAvatar;
  @Input() isSwitching = false;
  @Input() folders: AvatarFolder[] = [];
  @Input() selected = false;
  @Input() selectionMode = false;
  @Output() switch = new EventEmitter<string>();
  @Output() addToFolder = new EventEmitter<string>();
  @Output() removeFromFolder = new EventEmitter<string>();
  @Output() selectToggle = new EventEmitter<void>();
  @Output() openDetail = new EventEmitter<VRCAvatar>();
  @Output() cardContextMenu = new EventEmitter<CardContextMenuEvent>();

  hovered = signal(false);
  showFolderMenu = signal(false);

  @HostListener('document:click')
  onDocumentClick() {
    this.showFolderMenu.set(false);
  }

  onCardContextMenu(event: MouseEvent) {
    event.preventDefault();
    event.stopPropagation();
    this.showFolderMenu.set(false);
    this.cardContextMenu.emit({ avatar: this.avatar, x: event.clientX, y: event.clientY });
  }

  isInAnyFolder(): boolean {
    return this.folders.some(f => f.avatarIds.includes(this.avatar.id));
  }

  toggleFolder(folderId: string) {
    const folder = this.folders.find(f => f.id === folderId);
    if (!folder) return;
    if (folder.avatarIds.includes(this.avatar.id)) {
      this.removeFromFolder.emit(folderId);
    } else {
      this.addToFolder.emit(folderId);
    }
  }

  onSwitch() {
    this.switch.emit(this.avatar.id);
  }

  onImgError(event: Event) {
    const img = event.target as HTMLImageElement;
    img.style.display = 'none';
  }
}
