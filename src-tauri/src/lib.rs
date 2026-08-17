mod firewall;
mod net;
mod qr;
mod reachability;
mod safety;
mod server;
mod server_manager;
mod session;

use tauri::{
    menu::{Menu, MenuItem},
    Manager, WindowEvent,
};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            #[cfg(desktop)]
            {
                // Window shows automatically (visible: true in tauri.conf.json)
                if let Some(win) = app.get_webview_window("main") {
                    let _ = win.set_focus();
                }
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                if window.label() == "main" {
                    // Normal close = quit the app
                    api.prevent_close();
                    window.app_handle().exit(0);
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running airlynk");
}