import { Component, Input, Output, EventEmitter } from '@angular/core';
import { AvatarFolder } from '../core/models/avatar.model';

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
}
