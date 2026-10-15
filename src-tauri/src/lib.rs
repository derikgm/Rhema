mod audio;
mod asr;
mod commands;
mod models;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            commands::greet,
            commands::start_listening,
            commands::stop_listening,
            commands::start_asr,
            commands::stop_asr,
            commands::list_models,
            commands::download_model,
            commands::save_text,
            commands::save_draft,
            commands::list_dir,
            commands::read_text
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

