use std::sync::Mutex;
use tauri::{
    menu::{MenuBuilder, MenuItem},
    tray::TrayIconBuilder,
    Manager,
};
use serde::Serialize;

#[cfg(desktop)]
mod app_updates {
    use super::*;
    use tauri::ipc::Channel;
    use tauri_plugin_updater::{Update, UpdaterExt};

    #[derive(Clone, Serialize)]
    #[serde(tag = "event", content = "data")]
    pub enum DownloadEvent {
        #[serde(rename_all = "camelCase")]
        Started { content_length: Option<u64> },
        #[serde(rename_all = "camelCase")]
        Progress { chunk_length: usize },
        Finished,
    }

    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct UpdateInfo {
        pub version: String,
        pub current_version: String,
        pub date: Option<String>,
        pub body: Option<String>,
    }

    pub struct PendingUpdate(pub Mutex<Option<Update>>);

    #[tauri::command]
    pub async fn check_update(
        app: tauri::AppHandle,
        pending: tauri::State<'_, PendingUpdate>,
    ) -> Result<Option<UpdateInfo>, String> {
        let updater = app.updater().map_err(|e| e.to_string())?;
        let update = updater.check().await.map_err(|e| e.to_string())?;
        let info = update.as_ref().map(|u| UpdateInfo {
            version: u.version.clone(),
            current_version: u.current_version.clone(),
            date: u.date.map(|d| d.to_string()),
            body: u.body.clone(),
        });
        *pending.0.lock().unwrap() = update;
        Ok(info)
    }

    #[tauri::command]
    pub async fn install_update(
        pending: tauri::State<'_, PendingUpdate>,
        on_event: Channel<DownloadEvent>,
    ) -> Result<(), String> {
        let update = pending.0.lock().unwrap().take();
        let Some(update) = update else {
            return Err("there is no pending update".to_string());
        };

        update
            .download_and_install(
                |chunk_length, content_length| {
                    let _ = on_event.send(DownloadEvent::Started { content_length });
                    let _ = on_event.send(DownloadEvent::Progress { chunk_length });
                },
                || {
                    let _ = on_event.send(DownloadEvent::Finished);
                },
            )
            .await
            .map_err(|e| e.to_string())?;

        Ok(())
    }
}

pub fn run(server_port: u16) {
    let is_dev = cfg!(debug_assertions);

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(move |app| {
            #[cfg(desktop)]
            {
                app.handle().plugin(tauri_plugin_updater::Builder::new().build())?;
                app.handle().plugin(tauri_plugin_process::init())?;
                app.manage(app_updates::PendingUpdate(Mutex::new(None)));
            }

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
        .invoke_handler(tauri::generate_handler![
            #[cfg(desktop)]
            app_updates::check_update,
            #[cfg(desktop)]
            app_updates::install_update,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
