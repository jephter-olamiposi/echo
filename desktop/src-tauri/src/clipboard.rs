use arboard::Clipboard;
use std::thread;
use std::time::Duration;
use tauri::{AppHandle, Emitter};

const POLL_INTERVAL_MS: u64 = 500;
const INIT_RETRY_SECS: u64 = 5;

#[tauri::command]
pub fn get_clipboard() -> Result<String, String> {
    Clipboard::new()
        .map_err(|e| e.to_string())?
        .get_text()
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_clipboard(text: String) -> Result<(), String> {
    Clipboard::new()
        .map_err(|e| e.to_string())?
        .set_text(text)
        .map_err(|e| e.to_string())
}

pub fn start_clipboard_listener(app: AppHandle) {
    thread::spawn(move || loop {
        match Clipboard::new() {
            Ok(mut clipboard) => {
                let mut last_text = clipboard.get_text().unwrap_or_default();
                eprintln!("[clipboard] monitor started");

                loop {
                    thread::sleep(Duration::from_millis(POLL_INTERVAL_MS));

                    match clipboard.get_text() {
                        Ok(current) if current != last_text && !current.is_empty() => {
                            last_text = current.clone();
                            if let Err(e) = app.emit("clipboard-change", current) {
                                eprintln!("[clipboard] emit error: {e}");
                            }
                        }
                        Ok(_) => {}
                        Err(_) => {
                            // Non-text content (images, files) - silently ignore
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("[clipboard] init error: {e}, retrying in {INIT_RETRY_SECS}s");
                thread::sleep(Duration::from_secs(INIT_RETRY_SECS));
            }
        }
    });
}
