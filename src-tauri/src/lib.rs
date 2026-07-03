#![cfg_attr(not(feature = "tauri-app"), allow(dead_code))]

mod analytics;
mod auth;
#[cfg(feature = "tauri-app")]
mod commands;
mod connectors;
mod db;
#[cfg(feature = "tauri-app")]
mod desktop;
mod desktop_menu;
mod importers;
mod safety;
mod telemetry;

#[cfg(feature = "tauri-app")]
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            #[cfg(desktop)]
            app.handle()
                .plugin(tauri_plugin_window_state::Builder::default().build())?;
            desktop::install(app)?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_dashboard_summary,
            commands::refresh_all
        ])
        .run(tauri::generate_context!())
        .expect("error while running TokenStack");
}
