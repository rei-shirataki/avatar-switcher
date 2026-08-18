import { Component, Input, Output, EventEmitter } from '@angular/core';
import { AvatarFolder } from '../core/models/avatar.model';
import { FAVORITES_TAB, UPLOADED_TAB } from '../overlay-avatar.service';

@Component({
  selector: 'app-folder-tabs',
  imports: [],
  templateUrl: './folder-tabs.component.html',
  styleUrl: './folder-tabs.component.scss',
})
export class FolderTabsComponent {
  @Input() folders: AvatarFolder[] = [];
  @Input() selectedFolderId: string | null = null;
  @Output() select = new EventEmitter<string | null>();

  readonly FAVORITES_TAB = FAVORITES_TAB;
  readonly UPLOADED_TAB = UPLOADED_TAB;
}
