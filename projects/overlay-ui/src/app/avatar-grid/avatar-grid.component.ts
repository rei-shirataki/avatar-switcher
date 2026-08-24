import { Component, Input, Output, EventEmitter } from '@angular/core';
import { NgClass } from '@angular/common';
import { VRCAvatar } from '../core/models/avatar.model';

@Component({
  selector: 'app-avatar-grid',
  imports: [NgClass],
  templateUrl: './avatar-grid.component.html',
  host: {
    class: 'block flex-1 min-h-0 overflow-y-auto [scrollbar-width:thin] [scrollbar-color:var(--charcoal-color-container-tertiary-default)_transparent] [&::-webkit-scrollbar]:w-2 [&::-webkit-scrollbar-track]:bg-transparent [&::-webkit-scrollbar-thumb]:bg-[var(--charcoal-color-container-tertiary-default)] [&::-webkit-scrollbar-thumb]:rounded [&::-webkit-scrollbar-thumb:hover]:bg-[var(--charcoal-color-dark-neutral-20)]',
  },
})
export class AvatarGridComponent {
  @Input({ required: true }) avatars: VRCAvatar[] = [];
  @Input() currentAvatarId: string | null = null;
  @Input() switchingId: string | null = null;
  @Output() select = new EventEmitter<string>();

  onImgError(event: Event): void {
    (event.target as HTMLImageElement).style.display = 'none';
  }
}
