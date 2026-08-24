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
      class="avatar-card"
      [class.avatar-card--switching]="isSwitching"
      [class.avatar-card--selected]="selected"
      [class.avatar-card--selection-mode]="selectionMode"
      (mouseenter)="hovered.set(true)"
      (mouseleave)="hovered.set(false)"
      (click)="selectionMode ? selectToggle.emit() : openDetail.emit(avatar)"
      (contextmenu)="onCardContextMenu($event)"
    >
      <div class="avatar-card__thumb">
        @if (avatar.thumbnailImageUrl) {
          <img
            [src]="avatar.thumbnailImageUrl"
            [alt]="avatar.name"
            loading="lazy"
            (error)="onImgError($event)"
          />
        } @else {
          <div class="avatar-card__thumb-placeholder">
            {{ avatar.name.charAt(0) }}
          </div>
        }
        @if (selectionMode) {
          <div class="avatar-card__select-ring" [class.avatar-card__select-ring--checked]="selected">
            @if (selected) { <span>✓</span> }
          </div>
        }
        @if (!selectionMode && hovered() && !isSwitching) {
          <div class="avatar-card__overlay" (click)="onSwitch(); $event.stopPropagation()">
            <span class="avatar-card__switch-btn"><pixiv-icon name="24/Play" fixed-size="11" style="--charcoal-icon-size: 11px"></pixiv-icon> 切り替え</span>
          </div>
        }
        @if (isSwitching) {
          <div class="avatar-card__overlay avatar-card__overlay--switching">
            <app-icon class="avatar-card__spinner" name="spinner" [size]="24"/>
          </div>
        }
      </div>
      <div class="avatar-card__info">
        <div class="avatar-card__name" [title]="avatar.name">{{ avatar.name }}</div>
        <div class="avatar-card__author">{{ avatar.authorName }}</div>
        <div class="avatar-card__bottom">
          <div class="avatar-card__badge" [class]="'badge-' + avatar.releaseStatus">
            {{ avatar.releaseStatus === 'public' ? '公開' : '非公開' }}
          </div>
          @if (folders.length > 0 && !selectionMode) {
            <div class="avatar-card__folder-wrap">
              <button
                class="avatar-card__folder-btn"
                [class.avatar-card__folder-btn--active]="isInAnyFolder()"
                (click)="showFolderMenu.set(!showFolderMenu()); $event.stopPropagation()"
                title="フォルダ管理"
              ><app-icon name="folder-plus" [size]="13"/></button>
              @if (showFolderMenu()) {
                <div class="avatar-card__folder-menu" (click)="$event.stopPropagation()">
                  @for (folder of folders; track folder.id) {
                    <button
                      class="avatar-card__folder-item"
                      [class.avatar-card__folder-item--checked]="folder.avatarIds.includes(avatar.id)"
                      (click)="toggleFolder(folder.id)"
                    >
                      <span class="avatar-card__folder-check">
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
  styleUrl: './avatar-card.component.scss',
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
