import { Component, Input, Output, EventEmitter } from '@angular/core';
import { NgClass } from '@angular/common';
import { AvatarFolder } from '../core/models/avatar.model';
import { FAVORITES_TAB, UPLOADED_TAB } from '../overlay-avatar.service';

@Component({
  selector: 'app-folder-tabs',
  imports: [NgClass],
  templateUrl: './folder-tabs.component.html',
})
export class FolderTabsComponent {
  @Input() folders: AvatarFolder[] = [];
  @Input() selectedFolderId: string | null = null;
  @Output() select = new EventEmitter<string | null>();

  readonly FAVORITES_TAB = FAVORITES_TAB;
  readonly UPLOADED_TAB = UPLOADED_TAB;
}
