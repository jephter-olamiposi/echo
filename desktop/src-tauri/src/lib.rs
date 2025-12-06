mod clipboard;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let handle = app.handle().clone();
            clipboard::start_clipboard_listener(handle);
            Ok(())
        })
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_http::init())
        .invoke_handler(tauri::generate_handler![
            clipboard::get_clipboard,
            clipboard::set_clipboard
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
