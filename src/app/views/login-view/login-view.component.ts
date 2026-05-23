import { Component, signal } from '@angular/core';
import { FormsModule } from '@angular/forms';
import { Router } from '@angular/router';
import { VRChatAuthService } from '../../core/services/vrchat-auth.service';
import { CommonModule } from '@angular/common';

type LoginStep = 'credentials' | '2fa';

@Component({
  selector: 'app-login-view',
  standalone: true,
  imports: [FormsModule, CommonModule],
  templateUrl: './login-view.component.html',
  styleUrl: './login-view.component.scss',
})
export class LoginViewComponent {
  step = signal<LoginStep>('credentials');
  username = '';
  password = '';
  twoFactorCode = '';
  twoFactorMethod = signal<string | null>(null);
  loading = signal(false);
  error = signal<string | null>(null);

  constructor(private auth: VRChatAuthService, private router: Router) {}

  async onLogin() {
    if (!this.username || !this.password) {
      this.error.set('ユーザー名とパスワードを入力してください');
      return;
    }
    this.loading.set(true);
    this.error.set(null);
    try {
      const result = await this.auth.login(this.username, this.password);
      if (result.success) {
        this.password = '';
        this.router.navigate(['/avatars']);
      } else if (result.requires2fa) {
        this.password = ''; // no longer needed; subsequent requests use the session cookie
        this.twoFactorMethod.set(result.method);
        this.step.set('2fa');
      } else {
        this.error.set(result.error ?? 'ログインに失敗しました');
      }
    } catch (e: unknown) {
      this.error.set(e instanceof Error ? e.message : '接続エラーが発生しました');
    } finally {
      this.loading.set(false);
    }
  }

  async onVerify2fa() {
    if (!this.twoFactorCode) return;
    this.loading.set(true);
    this.error.set(null);
    try {
      const ok = await this.auth.verify2fa(this.twoFactorCode, this.twoFactorMethod() ?? 'totp');
      if (ok) {
        this.router.navigate(['/avatars']);
      } else {
        this.error.set('認証コードが正しくありません');
      }
    } catch (e: unknown) {
      this.error.set(e instanceof Error ? e.message : '認証エラー');
    } finally {
      this.loading.set(false);
    }
  }

  backToCredentials() {
    this.step.set('credentials');
    this.twoFactorCode = '';
    this.error.set(null);
  }

  /**
   * TOTP 認証アプリを紛失した場合のフォールバックとしてリカバリーコード
   * (`/auth/twofactorauth/otp/verify`) を使う。
   * emailOtp は固定で受信メールが必要なので切替対象外。
   */
  useRecoveryCode() {
    this.twoFactorMethod.set('otp');
    this.twoFactorCode = '';
    this.error.set(null);
  }

  backToTotp() {
    this.twoFactorMethod.set('totp');
    this.twoFactorCode = '';
    this.error.set(null);
  }
}
