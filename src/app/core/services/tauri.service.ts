import { Injectable } from '@angular/core';
import { invoke } from '@tauri-apps/api/core';
import { listen, Event, UnlistenFn } from '@tauri-apps/api/event';

@Injectable({ providedIn: 'root' })
export class TauriService {
  invoke<T>(command: string, args?: Record<string, unknown>): Promise<T> {
    return invoke<T>(command, args);
  }

  listen<T>(event: string, callback: (event: Event<T>) => void): Promise<UnlistenFn> {
    return listen<T>(event, callback);
  }
}
