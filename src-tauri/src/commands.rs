use std::fs;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};

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
            for entry in read_dir {
                if let Ok(entry) = entry {
                    if let Ok(child) = read_dir_recursive(entry.path(), max_depth, current_depth + 1) {
                        entries.push(child);
                    }
                }
            }
        }
        entries.sort_by(|a, b| {
            match (a.is_dir, b.is_dir) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
            }
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
