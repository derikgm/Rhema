mod asr;
mod audio;
mod commands;
mod models;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            // Audio / ASR
            commands::greet,
            commands::start_listening,
            commands::stop_listening,
            commands::start_asr,
            commands::stop_asr,
            // Modelos
            commands::list_models,
            commands::download_model,
            commands::delete_model,
            commands::set_active_model,
            commands::get_active_model,
            commands::models_dir_path,
            // Exportación / utilidades
            commands::save_text,
            commands::save_draft,
            commands::list_dir,
            commands::read_text,
            // Notas y carpetas
            commands::pick_folder,
            commands::read_folder_tree,
            commands::read_note,
            commands::save_note,
            commands::create_note,
            commands::create_folder,
            commands::delete_entry,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
