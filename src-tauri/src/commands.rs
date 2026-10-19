use std::fs;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

use crate::models::{self, ModelStatus};

// =====================================================================
// Audio / ASR
// =====================================================================

#[tauri::command]
pub fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
pub fn start_listening(app: AppHandle) -> Result<(), String> {
    crate::audio::start_listening(app)
}

#[tauri::command]
pub fn stop_listening() -> Result<(), String> {
    crate::audio::stop_listening()
}

#[tauri::command]
pub fn start_asr(app: AppHandle) -> Result<(), String> {
    crate::asr::start_session(app)
}

#[tauri::command]
pub fn stop_asr() -> Result<(), String> {
    crate::asr::stop_session();
    Ok(())
}

// =====================================================================
// Gestión de modelos
// =====================================================================

#[tauri::command]
pub fn list_models(app: AppHandle) -> Vec<ModelStatus> {
    models::status_list(&app)
}

#[tauri::command]
pub fn download_model(app: AppHandle, id: String) -> Result<String, String> {
    models::download_model(&app, &id)
}

#[tauri::command]
pub fn delete_model(app: AppHandle, id: String) -> Result<(), String> {
    models::delete_model(&app, &id)
}

#[tauri::command]
pub fn set_active_model(app: AppHandle, id: String) -> Result<(), String> {
    models::set_active(&app, &id)
}

#[tauri::command]
pub fn get_active_model(app: AppHandle) -> Option<String> {
    models::get_active(&app)
}

#[tauri::command]
pub fn models_dir_path(app: AppHandle) -> Result<String, String> {
    Ok(models::models_dir(&app)?.to_string_lossy().to_string())
}

// =====================================================================
// Exportación y utilidades (compatibilidad)
// =====================================================================

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
pub struct DirEntry {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
}

#[tauri::command]
pub fn list_dir(path: String) -> Result<Vec<DirEntry>, String> {
    let mut out = vec![];
    for e in fs::read_dir(&path).map_err(|e| e.to_string())? {
        let e = e.map_err(|e| e.to_string())?;
        let name = e.file_name().to_string_lossy().to_string();
        let is_dir = e.file_type().map(|t| t.is_dir()).unwrap_or(false);
        out.push(DirEntry {
            name,
            path: e.path().to_string_lossy().to_string(),
            is_dir,
        });
    }
    out.sort_by(|a, b| b.is_dir.cmp(&a.is_dir).then(a.name.cmp(&b.name)));
    Ok(out)
}

#[tauri::command]
pub fn read_text(path: String) -> Result<String, String> {
    fs::read_to_string(path).map_err(|e| e.to_string())
}

// =====================================================================
// Notas y carpetas (tree-folder)
// =====================================================================

#[derive(Debug, Serialize, Deserialize)]
pub struct FileEntry {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub children: Option<Vec<FileEntry>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NoteInfo {
    pub name: String,
    pub path: String,
    pub content: String,
}

#[tauri::command]
pub async fn pick_folder(app: tauri::AppHandle) -> Result<String, String> {
    use tauri_plugin_dialog::DialogExt;

    let folder = app.dialog().file().blocking_pick_folder();

    match folder {
        Some(path) => {
            let path_buf: PathBuf = path.to_string().into();
            Ok(path_buf.to_string_lossy().to_string())
        }
        None => Err("No folder selected".to_string()),
    }
}

#[tauri::command]
pub async fn read_folder_tree(path: String, depth: Option<u32>) -> Result<FileEntry, String> {
    let max_depth = depth.unwrap_or(10);
    read_dir_recursive(PathBuf::from(&path), max_depth, 0)
}

fn read_dir_recursive(path: PathBuf, max_depth: u32, current_depth: u32) -> Result<FileEntry, String> {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| path.to_string_lossy().to_string());

    let is_dir = path.is_dir();

    let children = if is_dir && current_depth < max_depth {
        let mut entries = Vec::new();
        if let Ok(read_dir) = fs::read_dir(&path) {
            for entry in read_dir.flatten() {
                if let Ok(child) = read_dir_recursive(entry.path(), max_depth, current_depth + 1) {
                    entries.push(child);
                }
            }
        }
        entries.sort_by(|a, b| match (a.is_dir, b.is_dir) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
        });
        Some(entries)
    } else {
        None
    };

    Ok(FileEntry {
        name,
        path: path.to_string_lossy().to_string(),
        is_dir,
        children,
    })
}

#[tauri::command]
pub async fn read_note(path: String) -> Result<NoteInfo, String> {
    let path_buf = PathBuf::from(&path);
    let name = path_buf
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "untitled".to_string());

    let content = fs::read_to_string(&path).map_err(|e| e.to_string())?;

    Ok(NoteInfo {
        name,
        path,
        content,
    })
}

#[tauri::command]
pub async fn save_note(path: String, content: String) -> Result<(), String> {
    fs::write(&path, content).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn create_note(dir_path: String, name: String) -> Result<NoteInfo, String> {
    let mut path = PathBuf::from(&dir_path);

    let name = if name.ends_with(".md") {
        name
    } else {
        format!("{}.md", name)
    };

    path.push(&name);

    if path.exists() {
        return Err(format!("Note '{}' already exists", name));
    }

    fs::write(&path, "").map_err(|e| e.to_string())?;

    Ok(NoteInfo {
        name,
        path: path.to_string_lossy().to_string(),
        content: String::new(),
    })
}

#[tauri::command]
pub async fn create_folder(dir_path: String, name: String) -> Result<FileEntry, String> {
    let mut path = PathBuf::from(&dir_path);
    path.push(&name);

    if path.exists() {
        return Err(format!("Folder '{}' already exists", name));
    }

    fs::create_dir_all(&path).map_err(|e| e.to_string())?;

    Ok(FileEntry {
        name,
        path: path.to_string_lossy().to_string(),
        is_dir: true,
        children: Some(vec![]),
    })
}

#[tauri::command]
pub async fn delete_entry(path: String) -> Result<(), String> {
    let path_buf = PathBuf::from(&path);

    if !path_buf.exists() {
        return Err("Path does not exist".to_string());
    }

    if path_buf.is_dir() {
        fs::remove_dir_all(&path_buf).map_err(|e| e.to_string())?;
    } else {
        fs::remove_file(&path_buf).map_err(|e| e.to_string())?;
    }

    Ok(())
}
