import { Component, inject } from '@angular/core';
import { DecimalPipe } from '@angular/common';
import { OverlayEyeHeightService } from '../overlay-eye-height.service';

@Component({
  selector: 'app-eye-height-control',
  imports: [DecimalPipe],
  templateUrl: './eye-height-control.component.html',
  styleUrl: './eye-height-control.component.scss',
})
export class EyeHeightControlComponent {
  protected readonly eyeHeight = inject(OverlayEyeHeightService);
}
