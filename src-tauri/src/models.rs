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
            let p = e.path();
            if p.is_dir() {
                let folder_name = p
                    .file_name()
                    .and_then(|n| n.to_str())
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
                                .and_then(|n| n.to_str())
                                .unwrap_or("")
                                .to_string();
                            let sz = fe.metadata().map(|m| m.len()).unwrap_or(0);
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
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    if dest_path.exists() {
        let sz = fs::metadata(dest_path).map(|m| m.len()).unwrap_or(0);
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
        let downloaded_b = fs::metadata(dest_path).map(|m| m.len() as f64).unwrap_or(0.0);
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
    let status = child.wait().map_err(|e| e.to_string())?;
    if !status.success() {
        let _ = fs::remove_file(dest_path);
        return Err(format!("curl terminó con código {:?}", status.code()));
    }
    let sz_final = fs::metadata(dest_path).map(|m| m.len()).unwrap_or(0);
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
    fs::create_dir_all(&mdir).map_err(|e| e.to_string())?;
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
            .and_then(|n| n.to_str())
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
    let mut i = 1;
    while mdir.exists() {
        mdir = root.join(format!("{}-{}", folder_name_base, i));
        i += 1;
    }
    fs::create_dir_all(&mdir).map_err(|e| e.to_string())?;
    fn cp_file(a: &Path, b: &Path) -> Result<(), String> {
        fs::copy(a, b).map(|_| ()).map_err(|e| e.to_string())
    }
    cp_file(&encoder, &mdir.join(encoder.file_name().unwrap_or_default()))?;
    cp_file(&decoder, &mdir.join(decoder.file_name().unwrap_or_default()))?;
    cp_file(&tokens, &mdir.join(tokens.file_name().unwrap_or_default()))?;
    if let Some(v) = vad {
        let _ = cp_file(&v, &mdir.join("silero_vad.onnx"));
    }
    let mut cfg = load_config(app);
    let folder_name_final = mdir
        .file_name()
        .and_then(|n| n.to_str())
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
        fs::remove_dir_all(&mdir).map_err(|e| e.to_string())?;
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
    let rd = fs::read_dir(folder).map_err(|e| e.to_string())?;
    for e in rd.flatten() {
        let p = e.path();
        if !p.is_file() {
            continue;
        }
        let n = p
            .file_name()
            .and_then(|x| x.to_str())
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
