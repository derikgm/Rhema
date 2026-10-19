use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::thread;
use std::time::Duration;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager};

#[derive(Serialize, Deserialize, Clone)]
pub struct DownloadProgress {
    pub filename: String,
    pub downloaded_mb: f64,
    pub total_mb: f64,
    pub done: bool,
}

/// Un archivo que forma parte de un modelo (algunos modelos son varios archivos).
#[derive(Serialize, Deserialize, Clone)]
pub struct ModelFile {
    pub url: String,
    pub filename: String,
    pub size_mb: f64,
}

/// Entrada estática del catálogo de modelos disponibles.
#[derive(Serialize, Deserialize, Clone)]
pub struct ModelInfo {
    pub id: String,
    pub name: String,
    pub language: String,
    pub streaming: bool,
    pub files: Vec<ModelFile>,
}

impl ModelInfo {
    /// Tamaño total de todos los archivos del modelo.
    pub fn total_mb(&self) -> f64 {
        self.files.iter().map(|f| f.size_mb).sum()
    }
}

/// Modelo del catálogo junto con su estado en disco y si está activo.
#[derive(Serialize, Deserialize, Clone)]
pub struct ModelStatus {
    pub id: String,
    pub name: String,
    pub language: String,
    pub streaming: bool,
    pub files: Vec<ModelFileStatus>,
    pub size_mb: f64,
    pub downloaded: bool,
    pub active: bool,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ModelFileStatus {
    pub url: String,
    pub filename: String,
    pub size_mb: f64,
    pub downloaded: bool,
}

/// Catálogo de modelos. Whisper tiny int8 es el modelo principal: soporta
/// español y es el que usa `asr.rs` (identificable por su nombre de archivos).
pub fn catalog() -> Vec<ModelInfo> {
    const HF_BASE: &str = "https://huggingface.co/csukuangfj/sherpa-onnx-whisper-tiny/resolve/main";
    const VAD_URL: &str =
        "https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/silero_vad.onnx";
    vec![
        ModelInfo {
            id: "whisper-tiny-es".into(),
            name: "Whisper tiny int8 (español)".into(),
            language: "es".into(),
            streaming: false,
            files: vec![
                ModelFile {
                    url: format!("{HF_BASE}/tiny-encoder.int8.onnx"),
                    filename: "tiny-encoder.int8.onnx".into(),
                    size_mb: 12.9,
                },
                ModelFile {
                    url: format!("{HF_BASE}/tiny-decoder.int8.onnx"),
                    filename: "tiny-decoder.int8.onnx".into(),
                    size_mb: 89.9,
                },
                ModelFile {
                    url: format!("{HF_BASE}/tiny-tokens.txt"),
                    filename: "tiny-tokens.txt".into(),
                    size_mb: 0.8,
                },
                // VAD compartido: sin él no se puede segmentar el habla.
                ModelFile {
                    url: VAD_URL.into(),
                    filename: "silero_vad.onnx".into(),
                    size_mb: 0.6,
                },
            ],
        },
    ]
}

/// Directorio donde se guardan los modelos descargados (appDataDir/models).
pub fn models_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let base = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let dir = base.join("models");
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}

fn config_path(app: &AppHandle) -> Result<PathBuf, String> {
    let base = app.path().app_data_dir().map_err(|e| e.to_string())?;
    fs::create_dir_all(&base).map_err(|e| e.to_string())?;
    Ok(base.join("rhema-config.json"))
}

#[derive(Serialize, Deserialize, Default)]
struct AppConfig {
    active_model: Option<String>,
}

/// Id del modelo activo seleccionado por el usuario, si lo hay.
pub fn get_active(app: &AppHandle) -> Option<String> {
    let path = config_path(app).ok()?;
    let data = fs::read_to_string(path).ok()?;
    let cfg: AppConfig = serde_json::from_str(&data).ok()?;
    cfg.active_model
}

pub fn set_active(app: &AppHandle, id: &str) -> Result<(), String> {
    let path = config_path(app)?;
    let cfg = AppConfig {
        active_model: Some(id.to_string()),
    };
    let json = serde_json::to_string_pretty(&cfg).map_err(|e| e.to_string())?;
    fs::write(path, json).map_err(|e| e.to_string())
}

/// Devuelve el catálogo con el estado de descarga y cuál está activo.
pub fn status_list(app: &AppHandle) -> Vec<ModelStatus> {
    let dir = models_dir(app).ok();
    let active = get_active(app);
    catalog()
        .into_iter()
        .map(|m| {
            let size_mb = m.total_mb();
            let files: Vec<ModelFileStatus> = m
                .files
                .into_iter()
                .map(|f| {
                    let downloaded = dir
                        .as_ref()
                        .map(|d| d.join(&f.filename).exists())
                        .unwrap_or(false);
                    ModelFileStatus {
                        url: f.url,
                        filename: f.filename,
                        size_mb: f.size_mb,
                        downloaded,
                    }
                })
                .collect();
            let downloaded = !files.is_empty() && files.iter().all(|f| f.downloaded);
            ModelStatus {
                id: m.id.clone(),
                name: m.name,
                language: m.language,
                streaming: m.streaming,
                files,
                size_mb,
                downloaded,
                active: active.as_deref() == Some(m.id.as_str()),
            }
        })
        .collect()
}

/// Elimina todos los archivos de un modelo del catálogo.
pub fn delete_model(app: &AppHandle, id: &str) -> Result<(), String> {
    let dir = models_dir(app)?;
    let Some(model) = catalog().into_iter().find(|m| m.id == id) else {
        return Err(format!("Modelo desconocido: {id}"));
    };
    for f in model.files {
        let path = dir.join(&f.filename);
        if path.exists() {
            fs::remove_file(&path).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

/// Descarga todos los archivos pendientes de un modelo con `curl`, emitiendo
/// `model-download-progress` durante el proceso. Los `.tar.bz2` se extraen.
pub fn download_model(app: &AppHandle, id: &str) -> Result<String, String> {
    let dir = models_dir(app)?;
    let Some(model) = catalog().into_iter().find(|m| m.id == id) else {
        return Err(format!("Modelo desconocido: {id}"));
    };

    for f in &model.files {
        let dest = dir.join(&f.filename);
        if dest.exists() {
            continue;
        }
        download_file(app, &f.url, &dest, &f.filename, f.size_mb)?;
    }

    Ok(dir.join(&model.files[0].filename).to_string_lossy().to_string())
}

/// Descarga un archivo con `curl`, emitiendo progreso cada 500 ms.
fn download_file(
    app: &AppHandle,
    url: &str,
    dest: &std::path::Path,
    filename: &str,
    total_mb: f64,
) -> Result<(), String> {
    let mut child = Command::new("curl")
        .arg("-L")
        .arg("--fail")
        .arg("-o")
        .arg(dest)
        .arg(url)
        .spawn()
        .map_err(|e| format!("No se pudo iniciar curl: {e}"))?;

    loop {
        let done = match child.try_wait() {
            Ok(Some(_)) => true,
            Ok(None) => false,
            Err(e) => return Err(e.to_string()),
        };
        let downloaded = fs::metadata(dest)
            .map(|m| m.len() as f64 / 1_000_000.0)
            .unwrap_or(0.0);
        let _ = app.emit(
            "model-download-progress",
            DownloadProgress {
                filename: filename.to_string(),
                downloaded_mb: downloaded,
                total_mb,
                done,
            },
        );
        if done {
            break;
        }
        thread::sleep(Duration::from_millis(500));
    }

    let status = child.wait().map_err(|e| e.to_string())?;
    if !status.success() {
        let _ = fs::remove_file(dest);
        return Err(format!("curl terminó con código {:?}", status.code()));
    }

    // Extracción best-effort de archivos comprimidos.
    if filename.ends_with(".tar.bz2") {
        if let Some(parent) = dest.parent() {
            let _ = Command::new("tar")
                .arg("-xjf")
                .arg(dest)
                .arg("-C")
                .arg(parent)
                .arg("--strip-components=1")
                .status();
        }
    }

    Ok(())
}
