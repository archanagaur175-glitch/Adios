mod commands;
mod db;
mod win;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let window = app
                .get_webview_window("main")
                .expect("main window should exist");
            let effect = win::vibrancy::apply(&window);
            println!("[vibrancy] applied: {effect}");

            let data_dir = app.path().app_data_dir().expect("app data dir");
            let conn = db::open(&data_dir.join("adios.db")).expect("open database");
            app.manage(db::DbState::new(conn));
            println!("[db] opened at {}", data_dir.join("adios.db").display());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::settings::get_settings,
            commands::settings::set_setting
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
