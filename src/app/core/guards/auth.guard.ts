import { inject } from '@angular/core';
import { CanActivateFn, Router } from '@angular/router';
import { VRChatAuthService } from '../services/vrchat-auth.service';

export const authGuard: CanActivateFn = async () => {
  const auth = inject(VRChatAuthService);
  const router = inject(Router);
  await auth.ensureInitialized();
  if (auth.isLoggedIn()) return true;
  return router.createUrlTree(['/login']);
};
