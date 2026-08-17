mod vrchat;
mod osc;
mod storage;

use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager,
};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // OSC/OSCQuery の再接続調査用。RUST_LOG 未設定時は info レベルで出す。
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.show();
                let _ = w.set_focus();
            }
        }))
        .setup(|app| {
            // --- System tray ---
            let show_item = MenuItem::with_id(app, "show", "AvatarSwitcher を表示", true, None::<&str>)?;
            let sep = PredefinedMenuItem::separator(app)?;
            let quit_item = MenuItem::with_id(app, "quit", "終了", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show_item, &sep, &quit_item])?;

            TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .tooltip("AvatarSwitcher")
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => {
                        if let Some(w) = app.get_webview_window("main") {
                            let _ = w.show();
                            let _ = w.set_focus();
                        }
                    }
                    "quit" => {
                        app.exit(0);
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(w) = app.get_webview_window("main") {
                            let _ = w.show();
                            let _ = w.set_focus();
                        }
                    }
                })
                .build(app)?;

            // --- VRChat init ---
            // SWR キャッシュは setup の同期セクションで先に初期化する。
            // フロントが起動直後に vrchat_get_cached_* を呼んだとき、
            // vrchat::init の async spawn 完了を待たずに CACHE_DIR が
            // 解決できるようにするため。
            let app_data_dir = app
                .path()
                .app_data_dir()
                .unwrap_or_else(|_| std::path::PathBuf::from("."));
            std::fs::create_dir_all(&app_data_dir).ok();
            vrchat::cache::init(app_data_dir);

            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                vrchat::init(&app_handle).await;
            });

            // --- OSC 受信 + OSCQuery init ---
            let app_handle_osc = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let osc_receive_port = match osc::init_receiver(app_handle_osc).await {
                    Ok(p) => p,
                    Err(e) => {
                        log::error!("OSC 受信ソケット初期化失敗: {}", e);
                        0
                    }
                };
                if let Err(e) = osc::oscquery::start(osc_receive_port).await {
                    log::error!("OSCQuery 起動失敗: {}", e);
                }
            });

            Ok(())
        })
        // ウィンドウの×ボタンでトレイに最小化
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                window.hide().unwrap();
                api.prevent_close();
            }
        })
        .invoke_handler(tauri::generate_handler![
            // VRChat auth
            vrchat::commands::vrchat_login,
            vrchat::commands::vrchat_verify_2fa,
            vrchat::commands::vrchat_get_current_user,
            vrchat::commands::vrchat_logout,
            // VRChat avatars
            vrchat::commands::vrchat_get_my_avatars,
            vrchat::commands::vrchat_get_favorite_avatars,
            vrchat::commands::vrchat_get_cached_avatars,
            vrchat::commands::vrchat_get_cached_favorites,
            vrchat::commands::vrchat_select_avatar,
            vrchat::commands::vrchat_update_avatar,
            vrchat::commands::vrchat_update_avatar_image,
            // OSC
            osc::commands::osc_change_avatar,
            osc::commands::osc_set_avatar_eye_height,
            osc::commands::osc_query_avatar_scale_snapshot,
            osc::commands::osc_get_status,
            // Storage (folders)
            storage::commands::folders_get_all,
            storage::commands::folders_create,
            storage::commands::folders_update,
            storage::commands::folders_delete,
            storage::commands::folders_add_avatar,
            storage::commands::folders_add_avatars,
            storage::commands::folders_remove_avatar,
            // Storage (avatar overrides)
            storage::commands::avatar_overrides_get_all,
            storage::commands::avatar_overrides_set,
            storage::commands::avatar_overrides_delete,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
