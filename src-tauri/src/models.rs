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

/// Entrada estática del catálogo de modelos disponibles.
#[derive(Serialize, Deserialize, Clone)]
pub struct ModelInfo {
    pub id: String,
    pub name: String,
    pub language: String,
    pub streaming: bool,
    pub url: String,
    pub filename: String,
    pub size_mb: f64,
}

/// Modelo del catálogo junto con su estado en disco y si está activo.
#[derive(Serialize, Deserialize, Clone)]
pub struct ModelStatus {
    pub id: String,
    pub name: String,
    pub language: String,
    pub streaming: bool,
    pub url: String,
    pub filename: String,
    pub size_mb: f64,
    pub downloaded: bool,
    pub active: bool,
}

pub fn catalog() -> Vec<ModelInfo> {
    vec![
        ModelInfo {
            id: "zipformer-en-20m".into(),
            name: "Streaming Zipformer EN (20M)".into(),
            language: "en".into(),
            streaming: true,
            url: "https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/sherpa-onnx-streaming-zipformer-en-20M-2023-02-17.tar.bz2".into(),
            filename: "sherpa-onnx-streaming-zipformer-en-20M-2023-02-17.tar.bz2".into(),
            size_mb: 70.0,
        },
        ModelInfo {
            id: "dolphin-multi".into(),
            name: "Dolphin base CTC multi-lang (int8)".into(),
            language: "multi".into(),
            streaming: false,
            url: "https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/sherpa-onnx-dolphin-base-ctc-multi-lang-int8-2025-04-02.tar.bz2".into(),
            filename: "sherpa-onnx-dolphin-base-ctc-multi-lang-int8-2025-04-02.tar.bz2".into(),
            size_mb: 99.0,
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
            let downloaded = dir
                .as_ref()
                .map(|d| d.join(&m.filename).exists())
                .unwrap_or(false);
            let is_active = active.as_deref() == Some(m.id.as_str());
            ModelStatus {
                id: m.id,
                name: m.name,
                language: m.language,
                streaming: m.streaming,
                url: m.url,
                filename: m.filename,
                size_mb: m.size_mb,
                downloaded,
                active: is_active,
            }
        })
        .collect()
}

pub fn delete_model(app: &AppHandle, filename: &str) -> Result<(), String> {
    let dir = models_dir(app)?;
    let path = dir.join(filename);
    if path.exists() {
        fs::remove_file(&path).map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Descarga un modelo con `curl`, emitiendo `model-download-progress` durante el proceso.
/// Si es un `.tar.bz2`, intenta extraerlo en el directorio de modelos.
pub fn download_model(
    app: &AppHandle,
    url: &str,
    filename: &str,
    total_mb: f64,
) -> Result<String, String> {
    let dir = models_dir(app)?;
    let dest = dir.join(filename);

    let mut child = Command::new("curl")
        .arg("-L")
        .arg("--fail")
        .arg("-o")
        .arg(&dest)
        .arg(url)
        .spawn()
        .map_err(|e| format!("No se pudo iniciar curl: {e}"))?;

    loop {
        let done = match child.try_wait() {
            Ok(Some(_)) => true,
            Ok(None) => false,
            Err(e) => return Err(e.to_string()),
        };
        let downloaded = fs::metadata(&dest)
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
        let _ = fs::remove_file(&dest);
        return Err(format!("curl terminó con código {:?}", status.code()));
    }

    // Extracción best-effort de archivos comprimidos.
    if filename.ends_with(".tar.bz2") {
        let _ = Command::new("tar")
            .arg("-xjf")
            .arg(&dest)
            .arg("-C")
            .arg(&dir)
            .arg("--strip-components=1")
            .status();
    }

    Ok(dest.to_string_lossy().to_string())
}
