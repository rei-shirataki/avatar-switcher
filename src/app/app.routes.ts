import { Routes } from '@angular/router';
import { authGuard } from './core/guards/auth.guard';

export const routes: Routes = [
  {
    path: 'login',
    loadComponent: () =>
      import('./views/login-view/login-view.component').then(m => m.LoginViewComponent),
  },
  {
    path: 'avatars',
    canActivate: [authGuard],
    loadComponent: () =>
      import('./views/avatars-view/avatars-view.component').then(m => m.AvatarsViewComponent),
  },
  {
    path: 'settings',
    canActivate: [authGuard],
    loadComponent: () =>
      import('./views/settings-view/settings-view.component').then(m => m.SettingsViewComponent),
  },
  { path: '', redirectTo: 'avatars', pathMatch: 'full' },
  { path: '**', redirectTo: 'avatars' },
];
