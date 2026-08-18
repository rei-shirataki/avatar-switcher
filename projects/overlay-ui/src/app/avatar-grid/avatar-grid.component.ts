import { Component, Input, Output, EventEmitter } from '@angular/core';
import { VRCAvatar } from '../core/models/avatar.model';

@Component({
  selector: 'app-avatar-grid',
  imports: [],
  templateUrl: './avatar-grid.component.html',
  styleUrl: './avatar-grid.component.scss',
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
