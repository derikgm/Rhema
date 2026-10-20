use serde::{Deserialize, Serialize};
use std::ffi::OsStr;
use std::fs;
use std::io::Error as IoError;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DownloadProgress {
    pub filename: String,
    pub downloaded_mb: f64,
    pub total_mb: f64,
    pub done: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ModelFile {
    pub url: String,
    pub filename: String,
    pub size_bytes: u64,
    pub size_mb: f64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ModelInfo {
    pub id: String,
    pub folder_name: String,
    pub name: String,
    pub description: String,
    pub language: String,
    pub streaming: bool,
    pub files: Vec<ModelFile>,
    pub size_mb: f64,
}

impl ModelInfo {
    pub fn total_mb(&self) -> f64 {
        self.files.iter().map(|f| f.size_mb).sum()
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LocalModelFile {
    pub filename: String,
    pub exists: bool,
    pub size_bytes: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LocalModelStatus {
    pub id: Option<String>,
    pub folder_name: String,
    pub folder_path: String,
    pub name: String,
    pub description: String,
    pub language: String,
    pub streaming: bool,
    pub files: Vec<LocalModelFile>,
    pub size_mb: f64,
    pub complete: bool,
    pub active: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct AppConfig {
    pub active_model_folder: Option<String>,
    pub models_dir: Option<String>,
}

fn config_path(app: &AppHandle) -> Result<PathBuf, String> {
    let base = app.path().app_data_dir().map_err(|e| e.to_string())?;
    fs::create_dir_all(&base).map_err(|e: IoError| e.to_string())?;
    Ok(base.join("rhema-config.json"))
}

pub fn load_config(app: &AppHandle) -> AppConfig {
    let path = match config_path(app) {
        Ok(p) => p,
        Err(_) => return AppConfig::default(),
    };
    match fs::read_to_string(path) {
        Ok(data) => serde_json::from_str(&data).unwrap_or_default(),
        Err(_) => AppConfig::default(),
    }
}

pub fn save_config(app: &AppHandle, cfg: &AppConfig) -> Result<(), String> {
    let path = config_path(app)?;
    let json = serde_json::to_string_pretty(cfg).map_err(|e| e.to_string())?;
    fs::write(path, json).map_err(|e: IoError| e.to_string())
}

fn is_writable_dir(p: &Path) -> bool {
    if fs::create_dir_all(p).is_err() {
        return false;
    }
    let tmp = p.join(".rhema-writable-test.tmp");
    match fs::write(&tmp, b"x") {
        Ok(_) => {
            let _ = fs::remove_file(&tmp);
            true
        }
        Err(_) => false,
    }
}

pub fn models_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let cfg = load_config(app);
    if let Some(ref md) = cfg.models_dir {
        let pb = PathBuf::from(md);
        let _ = fs::create_dir_all(&pb);
        if is_writable_dir(&pb) {
            return Ok(pb);
        }
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            let pb = parent.join("modelos");
            if is_writable_dir(&pb) {
                return Ok(pb);
            }
        }
    }
    let base = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let pb = base.join("modelos");
    fs::create_dir_all(&pb).map_err(|e: IoError| e.to_string())?;
    Ok(pb)
}

pub fn set_models_dir_path(app: &AppHandle, dir: &str) -> Result<String, String> {
    let pb = PathBuf::from(dir);
    fs::create_dir_all(&pb).map_err(|e: IoError| e.to_string())?;
    let mut cfg = load_config(app);
    cfg.models_dir = Some(pb.to_string_lossy().to_string());
    save_config(app, &cfg)?;
    Ok(pb.to_string_lossy().to_string())
}

pub fn catalog() -> Vec<ModelInfo> {
    const HF_TINY: &str = "https://huggingface.co/csukuangfj/sherpa-onnx-whisper-tiny/resolve/main";
    const HF_BASE: &str = "https://huggingface.co/csukuangfj/sherpa-onnx-whisper-base/resolve/main";
    const HF_SMALL: &str = "https://huggingface.co/csukuangfj/sherpa-onnx-whisper-small/resolve/main";
    const HF_MEDIUM: &str = "https://huggingface.co/csukuangfj/sherpa-onnx-whisper-medium/resolve/main";
    const VAD_URL: &str = "https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/silero_vad.onnx";
    let vad_file = ModelFile {
        url: VAD_URL.into(),
        filename: "silero_vad.onnx".into(),
        size_bytes: 643_854,
        size_mb: 0.614,
    };
    vec![
        ModelInfo {
            id: "whisper-tiny-es".into(),
            folder_name: "whisper-tiny-es".into(),
            name: "Whisper tiny int8 (español)".into(),
            description: "Muy ligero (~104 MB). Ideal para internet limitado o dispositivos con menos recursos. Precisión básica.".into(),
            language: "es".into(),
            streaming: false,
            files: vec![
                ModelFile { url: format!("{HF_TINY}/tiny-encoder.int8.onnx"), filename: "tiny-encoder.int8.onnx".into(), size_bytes: 12_937_772, size_mb: 12.337 },
                ModelFile { url: format!("{HF_TINY}/tiny-decoder.int8.onnx"), filename: "tiny-decoder.int8.onnx".into(), size_bytes: 89_855_401, size_mb: 85.687 },
                ModelFile { url: format!("{HF_TINY}/tiny-tokens.txt"), filename: "tiny-tokens.txt".into(), size_bytes: 816_730, size_mb: 0.779 },
                vad_file.clone(),
            ],
            size_mb: 0.0,
        },
        ModelInfo {
            id: "whisper-base-es".into(),
            folder_name: "whisper-base-es".into(),
            name: "Whisper base int8 (español)".into(),
            description: "Mejor equilibrio entre precisión y tamaño (~160 MB). Recomendado para la mayoría de casos.".into(),
            language: "es".into(),
            streaming: false,
            files: vec![
                ModelFile { url: format!("{HF_BASE}/base-encoder.int8.onnx"), filename: "base-encoder.int8.onnx".into(), size_bytes: 29_120_534, size_mb: 27.777 },
                ModelFile { url: format!("{HF_BASE}/base-decoder.int8.onnx"), filename: "base-decoder.int8.onnx".into(), size_bytes: 130_672_026, size_mb: 124.612 },
                ModelFile { url: format!("{HF_BASE}/base-tokens.txt"), filename: "base-tokens.txt".into(), size_bytes: 816_730, size_mb: 0.779 },
                vad_file.clone(),
            ],
            size_mb: 0.0,
        },
        ModelInfo {
            id: "whisper-small-es".into(),
            folder_name: "whisper-small-es".into(),
            name: "Whisper small int8 (español)".into(),
            description: "Precisión notable (~375 MB). Bueno para dictados largos o entornos con ruido.".into(),
            language: "es".into(),
            streaming: false,
            files: vec![
                ModelFile { url: format!("{HF_SMALL}/small-encoder.int8.onnx"), filename: "small-encoder.int8.onnx".into(), size_bytes: 112_442_483, size_mb: 107.244 },
                ModelFile { url: format!("{HF_SMALL}/small-decoder.int8.onnx"), filename: "small-decoder.int8.onnx".into(), size_bytes: 262_226_114, size_mb: 250.034 },
                ModelFile { url: format!("{HF_SMALL}/small-tokens.txt"), filename: "small-tokens.txt".into(), size_bytes: 816_730, size_mb: 0.779 },
                vad_file.clone(),
            ],
            size_mb: 0.0,
        },
        ModelInfo {
            id: "whisper-medium-es".into(),
            folder_name: "whisper-medium-es".into(),
            name: "Whisper medium int8 (español)".into(),
            description: "Máxima precisión entre los int8 (~946 MB). Requiere más CPU/RAM y espacio.".into(),
            language: "es".into(),
            streaming: false,
            files: vec![
                ModelFile { url: format!("{HF_MEDIUM}/medium-encoder.int8.onnx"), filename: "medium-encoder.int8.onnx".into(), size_bytes: 374_196_283, size_mb: 356.795 },
                ModelFile { url: format!("{HF_MEDIUM}/medium-decoder.int8.onnx"), filename: "medium-decoder.int8.onnx".into(), size_bytes: 571_059_257, size_mb: 544.522 },
                ModelFile { url: format!("{HF_MEDIUM}/medium-tokens.txt"), filename: "medium-tokens.txt".into(), size_bytes: 816_730, size_mb: 0.779 },
                vad_file.clone(),
            ],
            size_mb: 0.0,
        },
    ]
    .into_iter()
    .map(|mut m| {
        m.size_mb = m.total_mb();
        m
    })
    .collect()
}

pub fn list_catalog() -> Vec<ModelInfo> {
    catalog()
}

pub fn find_model_info(id: &str) -> Option<ModelInfo> {
    catalog().into_iter().find(|m| m.id == id)
}

pub fn get_active_model_folder(app: &AppHandle) -> Option<String> {
    load_config(app).active_model_folder
}

pub fn set_active_model_folder(app: &AppHandle, folder_name: &str) -> Result<(), String> {
    let mut cfg = load_config(app);
    cfg.active_model_folder = Some(folder_name.to_string());
    save_config(app, &cfg)
}

pub fn scan_local_models(app: &AppHandle) -> Vec<LocalModelStatus> {
    let root = match models_dir(app) {
        Ok(r) => r,
        Err(_) => return vec![],
    };
    let cat = catalog();
    let cfg = load_config(app);
    let active = cfg.active_model_folder.as_deref();
    let mut out: Vec<LocalModelStatus> = Vec::new();
    if let Ok(rd) = fs::read_dir(&root) {
        for e in rd.flatten() {
            let p: PathBuf = e.path();
            if p.is_dir() {
                let folder_name = p
                    .file_name()
                    .and_then(|n: &OsStr| n.to_str())
                    .unwrap_or("")
                    .to_string();
                if folder_name.is_empty() {
                    continue;
                }
                let mut files: Vec<LocalModelFile> = Vec::new();
                let mut has_encoder = false;
                let mut has_decoder = false;
                let mut has_tokens = false;
                let mut has_vad = false;
                if let Ok(rf) = fs::read_dir(&p) {
                    for fe in rf.flatten() {
                        let fp = fe.path();
                        if fp.is_file() {
                            let fname = fp
                                .file_name()
                                .and_then(|n: &OsStr| n.to_str())
                                .unwrap_or("")
                                .to_string();
                            let sz: u64 = fe.metadata().map(|m| m.len()).unwrap_or(0);
                            if fname.contains("encoder") && fname.ends_with(".onnx") {
                                has_encoder = true;
                            }
                            if fname.contains("decoder") && fname.ends_with(".onnx") {
                                has_decoder = true;
                            }
                            if fname.ends_with("tokens.txt") {
                                has_tokens = true;
                            }
                            if fname == "silero_vad.onnx" {
                                has_vad = true;
                            }
                            files.push(LocalModelFile {
                                filename: fname,
                                exists: true,
                                size_bytes: sz,
                            });
                        }
                    }
                }
                let complete = has_encoder && has_decoder && has_tokens && has_vad;
                let (cname, cdesc, clang, cstream, cid) = cat
                    .iter()
                    .find(|m| m.folder_name == folder_name)
                    .map(|m| {
                        (
                            m.name.clone(),
                            m.description.clone(),
                            m.language.clone(),
                            m.streaming,
                            Some(m.id.clone()),
                        )
                    })
                    .unwrap_or_else(|| {
                        let name = folder_name.clone();
                        (name.clone(), "Modelo importado/local".into(), "es".into(), false, None)
                    });
                let total_mb: f64 = files.iter().map(|f| (f.size_bytes as f64) / 1_000_000.0).sum();
                out.push(LocalModelStatus {
                    id: cid,
                    folder_name: folder_name.clone(),
                    folder_path: p.to_string_lossy().to_string(),
                    name: cname,
                    description: cdesc,
                    language: clang,
                    streaming: cstream,
                    files,
                    size_mb: total_mb,
                    complete,
                    active: active == Some(folder_name.as_str()),
                });
            }
        }
    }
    out.sort_by(|a, b| {
        b.active
            .cmp(&a.active)
            .then(b.complete.cmp(&a.complete))
            .then(a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    out
}

pub fn list_models_combined(app: &AppHandle) -> Vec<LocalModelStatus> {
    scan_local_models(app)
}

fn download_file(
    app: &AppHandle,
    url: &str,
    dest_path: &Path,
    filename: &str,
    total_bytes: u64,
    total_mb: f64,
) -> Result<(), String> {
    if let Some(parent) = dest_path.parent() {
        fs::create_dir_all(parent).map_err(|e: IoError| e.to_string())?;
    }
    if dest_path.exists() {
        let sz: u64 = fs::metadata(dest_path).map(|m| m.len()).unwrap_or(0);
        if sz == total_bytes {
            return Ok(());
        }
    }
    let mut child = Command::new("curl")
        .arg("-L")
        .arg("--fail")
        .arg("-o")
        .arg(dest_path)
        .arg(url)
        .spawn()
        .map_err(|e| format!("No se pudo iniciar curl: {e}"))?;
    loop {
        let done = match child.try_wait() {
            Ok(Some(_)) => true,
            Ok(None) => false,
            Err(e) => return Err(e.to_string()),
        };
        let downloaded_b: f64 = fs::metadata(dest_path).map(|m| m.len() as f64).unwrap_or(0.0);
        let _ = app.emit(
            "model-download-progress",
            DownloadProgress {
                filename: filename.to_string(),
                downloaded_mb: downloaded_b / 1_000_000.0,
                total_mb,
                done,
            },
        );
        if done {
            break;
        }
        thread::sleep(Duration::from_millis(500));
    }
    let status = child.wait().map_err(|e: IoError| e.to_string())?;
    if !status.success() {
        let _ = fs::remove_file(dest_path);
        return Err(format!("curl terminó con código {:?}", status.code()));
    }
    let sz_final: u64 = fs::metadata(dest_path).map(|m| m.len()).unwrap_or(0);
    if total_bytes > 0 && sz_final != total_bytes {
        let _ = fs::remove_file(dest_path);
        return Err(format!(
            "Tamaño incorrecto al descargar {}: {} vs esperado {}",
            filename, sz_final, total_bytes
        ));
    }
    if filename.ends_with(".tar.bz2") {
        if let Some(parent) = dest_path.parent() {
            let _ = Command::new("tar")
                .arg("-xjf")
                .arg(dest_path)
                .arg("-C")
                .arg(parent)
                .arg("--strip-components=1")
                .status();
        }
    }
    Ok(())
}

pub fn download_model(app: &AppHandle, id: &str) -> Result<String, String> {
    let m = find_model_info(id).ok_or_else(|| format!("Modelo desconocido: {id}"))?;
    let root = models_dir(app)?;
    let mdir = root.join(&m.folder_name);
    fs::create_dir_all(&mdir).map_err(|e: IoError| e.to_string())?;
    for f in &m.files {
        let dest = mdir.join(&f.filename);
        download_file(app, &f.url, &dest, &f.filename, f.size_bytes, f.size_mb)?;
    }
    let mut cfg = load_config(app);
    if cfg.active_model_folder.is_none() {
        cfg.active_model_folder = Some(m.folder_name.clone());
        save_config(app, &cfg)?;
    }
    Ok(mdir.to_string_lossy().to_string())
}

pub fn import_model_files(app: &AppHandle, paths: Vec<String>) -> Result<LocalModelStatus, String> {
    if paths.is_empty() {
        return Err("No se seleccionaron archivos".into());
    }
    let root = models_dir(app)?;
    let mut encoder: Option<PathBuf> = None;
    let mut decoder: Option<PathBuf> = None;
    let mut tokens: Option<PathBuf> = None;
    let mut vad: Option<PathBuf> = None;
    for s in &paths {
        let p = PathBuf::from(s);
        if !p.is_file() {
            continue;
        }
        let name = p
            .file_name()
            .and_then(|n: &OsStr| n.to_str())
            .unwrap_or("");
        if name.contains("encoder") && name.ends_with(".onnx") {
            encoder = Some(p.clone());
        }
        if name.contains("decoder") && name.ends_with(".onnx") {
            decoder = Some(p.clone());
        }
        if name.ends_with("tokens.txt") {
            tokens = Some(p.clone());
        }
        if name == "silero_vad.onnx" {
            vad = Some(p.clone());
        }
    }
    let encoder = encoder.ok_or("Falta encoder (*.encoder*.onnx)")?;
    let decoder = decoder.ok_or("Falta decoder (*.decoder*.onnx)")?;
    let tokens = tokens.ok_or("Falta tokens (*.tokens.txt)")?;
    let folder_name_base = "imported-whisper";
    let mut mdir = root.join(folder_name_base);
    let mut i: u32 = 1;
    while mdir.exists() {
        mdir = root.join(format!("{}-{}", folder_name_base, i));
        i += 1;
    }
    fs::create_dir_all(&mdir).map_err(|e: IoError| e.to_string())?;
    fn cp_file(a: &Path, b: &Path) -> Result<(), String> {
        fs::copy(a, b).map(|_| ()).map_err(|e: IoError| e.to_string())
    }
    if let Some(enc_name) = encoder.file_name() {
        cp_file(&encoder, &mdir.join(enc_name))?;
    }
    if let Some(dec_name) = decoder.file_name() {
        cp_file(&decoder, &mdir.join(dec_name))?;
    }
    if let Some(tok_name) = tokens.file_name() {
        cp_file(&tokens, &mdir.join(tok_name))?;
    }
    if let Some(v) = vad {
        let _ = cp_file(&v, &mdir.join("silero_vad.onnx"));
    }
    let mut cfg = load_config(app);
    let folder_name_final = mdir
        .file_name()
        .and_then(|n: &OsStr| n.to_str())
        .unwrap_or(folder_name_base)
        .to_string();
    cfg.active_model_folder = Some(folder_name_final.clone());
    save_config(app, &cfg)?;
    let locals = scan_local_models(app);
    for x in locals {
        if x.folder_name == folder_name_final {
            return Ok(x);
        }
    }
    Ok(LocalModelStatus {
        id: None,
        folder_name: folder_name_final.clone(),
        folder_path: mdir.to_string_lossy().to_string(),
        name: "Modelo importado".into(),
        description: "Modelo importado/local".into(),
        language: "es".into(),
        streaming: false,
        files: vec![],
        size_mb: 0.0,
        complete: true,
        active: true,
    })
}

pub fn delete_model_folder(app: &AppHandle, folder_name: &str) -> Result<(), String> {
    let root = models_dir(app)?;
    let mdir = root.join(folder_name);
    if mdir.exists() && mdir.is_dir() {
        fs::remove_dir_all(&mdir).map_err(|e: IoError| e.to_string())?;
    }
    let mut cfg = load_config(app);
    if cfg.active_model_folder.as_deref() == Some(folder_name) {
        cfg.active_model_folder = None;
        save_config(app, &cfg)?;
    }
    Ok(())
}

pub fn active_model_folder_path(app: &AppHandle) -> Option<PathBuf> {
    let cfg = load_config(app);
    let folder = cfg.active_model_folder.as_ref()?;
    let root = models_dir(app).ok()?;
    let p = root.join(folder);
    if p.exists() && p.is_dir() { Some(p) } else { None }
}

#[derive(Debug, Clone)]
pub struct ModelFolderFiles {
    pub encoder: PathBuf,
    pub decoder: PathBuf,
    pub tokens: PathBuf,
    pub vad: PathBuf,
}

pub fn resolve_model_files_in_folder(folder: &Path) -> Result<ModelFolderFiles, String> {
    let mut encoder: Option<PathBuf> = None;
    let mut decoder: Option<PathBuf> = None;
    let mut tokens: Option<PathBuf> = None;
    let mut vad: Option<PathBuf> = None;
    let rd = fs::read_dir(folder).map_err(|e: IoError| e.to_string())?;
    for e in rd.flatten() {
        let p: PathBuf = e.path();
        if !p.is_file() {
            continue;
        }
        let n = p
            .file_name()
            .and_then(|x: &OsStr| x.to_str())
            .unwrap_or("");
        if n.contains("encoder") && n.ends_with(".onnx") {
            encoder = Some(p.clone());
        }
        if n.contains("decoder") && n.ends_with(".onnx") {
            decoder = Some(p.clone());
        }
        if n.ends_with("tokens.txt") {
            tokens = Some(p.clone());
        }
        if n == "silero_vad.onnx" {
            vad = Some(p.clone());
        }
    }
    Ok(ModelFolderFiles {
        encoder: encoder.ok_or("Falta encoder en carpeta del modelo")?,
        decoder: decoder.ok_or("Falta decoder en carpeta del modelo")?,
        tokens: tokens.ok_or("Falta tokens en carpeta del modelo")?,
        vad: vad.ok_or("Falta silero_vad.onnx en carpeta del modelo")?,
    })
}
