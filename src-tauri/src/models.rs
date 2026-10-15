use std::fs;
use std::process::Command;
use std::thread;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct DownloadProgress {
    pub filename: String,
    pub downloaded_mb: f64,
    pub total_mb: f64,
    pub done: bool,
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct ModelInfo {
    pub id: String,
    pub name: String,
    pub language: String,
    pub streaming: bool,
    pub url: String,
    pub filename: String,
    pub size_mb: f64,
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

pub fn download_model(app: &AppHandle, url: &str, filename: &str, total_mb: f64) -> Result<String, String> {
    let base = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let dir = base.join("models");
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let dest = dir.join(filename);

    let mut child = Command::new("curl")
        .arg("-L")
        .arg("-o")
        .arg(&dest)
        .arg(url)
        .spawn()
        .map_err(|e| e.to_string())?;

    loop {
        let done = match child.try_wait() {
            Ok(Some(_)) => true,
            Ok(None) => false,
            Err(e) => return Err(e.to_string()),
        };
        let downloaded = fs::metadata(&dest).map(|m| m.len() as f64 / 1_000_000.0).unwrap_or(0.0);
        let _ = app.emit("model-download-progress", DownloadProgress {
            filename: filename.to_string(),
            downloaded_mb: downloaded,
            total_mb,
            done,
        });
        if done {
            break;
        }
        thread::sleep(Duration::from_millis(500));
    }

    let status = child.wait().map_err(|e| e.to_string())?;
    if status.success() {
        Ok(dest.to_string_lossy().to_string())
    } else {
        Err(format!("curl exit {:?}", status.code()))
    }
}
