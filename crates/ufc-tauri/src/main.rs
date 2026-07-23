#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            commands::detect_format,
            commands::convert_file,
            commands::batch_convert,
            commands::list_formats,
            commands::get_queue_status,
            commands::cancel_conversion,
            commands::get_history,
            commands::clear_history,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
