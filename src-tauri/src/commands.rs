use tauri::{AppHandle, Manager};
use std::fs;

#[tauri::command]
pub fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
pub fn start_listening(app: tauri::AppHandle) -> Result<(), String> {
    crate::audio::start_listening(app)
}

#[tauri::command]
pub fn stop_listening() -> Result<(), String> {
    crate::audio::stop_listening()
}

#[tauri::command]
pub fn start_asr(app: tauri::AppHandle) -> Result<(), String> {
    crate::asr::start_session(app)
}

#[tauri::command]
pub fn stop_asr() -> Result<(), String> {
    crate::asr::stop_session();
    Ok(())
}

#[tauri::command]
pub fn list_models() -> Vec<crate::models::ModelInfo> {
    crate::models::catalog()
}

#[tauri::command]
pub fn download_model(app: AppHandle, url: String, filename: String) -> Result<String, String> {
    let total_mb = crate::models::catalog()
        .iter()
        .find(|m| m.filename == filename)
        .map(|m| m.size_mb)
        .unwrap_or(0.0);
    crate::models::download_model(&app, &url, &filename, total_mb)
}

#[tauri::command]
pub fn save_text(path: String, content: String) -> Result<(), String> {
    if let Some(parent) = std::path::Path::new(&path).parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::write(path, content).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn save_draft(app: AppHandle, content: String) -> Result<String, String> {
    let base = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let dir = base.join("drafts");
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let path = dir.join("draft.txt");
    fs::write(&path, content).map_err(|e| e.to_string())?;
    Ok(path.to_string_lossy().to_string())
}

#[derive(serde::Serialize)]
pub struct DirEntry { pub name: String, pub path: String, pub is_dir: bool }

#[tauri::command]
pub fn list_dir(path: String) -> Result<Vec<DirEntry>, String> {
    let mut out = vec![];
    for e in fs::read_dir(&path).map_err(|e| e.to_string())? {
        let e = e.map_err(|e| e.to_string())?;
        let name = e.file_name().to_string_lossy().to_string();
        let is_dir = e.file_type().map(|t| t.is_dir()).unwrap_or(false);
        out.push(DirEntry { name, path: e.path().to_string_lossy().to_string(), is_dir });
    }
    out.sort_by(|a,b| b.is_dir.cmp(&a.is_dir).then(a.name.cmp(&b.name)));
    Ok(out)
}

#[tauri::command]
pub fn read_text(path: String) -> Result<String, String> {
    fs::read_to_string(path).map_err(|e| e.to_string())
}
