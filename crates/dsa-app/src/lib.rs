use tauri::{
    menu::{MenuBuilder, MenuItem},
    tray::TrayIconBuilder,
    Manager,
};

pub fn run(server_port: u16) {
    let is_dev = cfg!(debug_assertions);

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(move |app| {
            let handle = app.handle().clone();

            let show_item = MenuItem::with_id(&handle, "show", "显示窗口", true, None::<&str>)?;
            let quit_item = MenuItem::with_id(&handle, "quit", "退出", true, None::<&str>)?;
            let menu = MenuBuilder::new(&handle)
                .item(&show_item)
                .separator()
                .item(&quit_item)
                .build()?;

            let mut tray = TrayIconBuilder::new()
                .tooltip("DSA - Daily Stock Analysis")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(move |app, event| {
                    match event.id().as_ref() {
                        "show" => {
                            if let Some(window) = app.get_webview_window("main") {
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        }
                        "quit" => {
                            app.exit(0);
                        }
                        _ => {}
                    }
                })
                .on_tray_icon_event(|tray, event| {
                    if let tauri::tray::TrayIconEvent::DoubleClick { .. } = event {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                });

            if let Some(icon) = app.default_window_icon().cloned() {
                tray = tray.icon(icon);
            }

            tray.build(app)?;

            if !is_dev {
                let window = app.get_webview_window("main").unwrap();
                let url = format!("http://127.0.0.1:{}", server_port);
                let _ = window.eval(&format!(
                    "window.location.replace('{}')",
                    url
                ));
            }

            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                window.hide().unwrap();
                api.prevent_close();
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
