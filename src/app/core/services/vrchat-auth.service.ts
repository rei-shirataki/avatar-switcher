import { Injectable, signal, computed } from '@angular/core';
import { TauriService } from './tauri.service';
import { VRCUser, LoginResult } from '../models/auth.model';

@Injectable({ providedIn: 'root' })
export class VRChatAuthService {
  private readonly _currentUser = signal<VRCUser | null>(null);
  readonly user = this._currentUser.asReadonly();
  readonly isLoggedIn = computed(() => this._currentUser() !== null);

  private _initPromise: Promise<void> | null = null;

  constructor(private tauri: TauriService) {}

  ensureInitialized(): Promise<void> {
    if (!this._initPromise) {
      this._initPromise = this.initialize();
    }
    return this._initPromise;
  }

  async initialize(): Promise<void> {
    try {
      const user = await this.tauri.invoke<VRCUser | null>('vrchat_get_current_user');
      this._currentUser.set(user);
    } catch {
      this._currentUser.set(null);
    }
  }

  async login(username: string, password: string): Promise<LoginResult> {
    const result = await this.tauri.invoke<LoginResult>('vrchat_login', { username, password });
    if (result.success && result.user) {
      this._currentUser.set(result.user);
    }
    return result;
  }

  async verify2fa(code: string, method: string): Promise<boolean> {
    const verified = await this.tauri.invoke<boolean>('vrchat_verify_2fa', { code, method });
    if (verified) {
      const user = await this.tauri.invoke<VRCUser | null>('vrchat_get_current_user');
      this._currentUser.set(user);
    }
    return verified;
  }

  async logout(): Promise<void> {
    await this.tauri.invoke('vrchat_logout');
    this._currentUser.set(null);
  }
}
