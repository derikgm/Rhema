use sherpa_rs::dolphin::{DolphinConfig, DolphinRecognizer};
use sherpa_rs::silero_vad::{SileroVad, SileroVadConfig};
use std::path::PathBuf;
use std::sync::mpsc::{self, Sender};
use std::sync::Mutex;
use std::thread;
use tauri::{AppHandle, Emitter, Manager};

/// Sesión de reconocimiento activa.
///
/// El VAD corre en el hilo de audio (rápido) y envía cada segmento de voz a un
/// hilo trabajador que posee el reconocedor (lento), para no bloquear la captura.
struct Session {
    vad: SileroVad,
    tx: Sender<Vec<f32>>,
}

static STATE: Mutex<Option<Session>> = Mutex::new(None);

/// Busca el directorio que contiene `silero_vad.onnx`, `tokens.txt` y el modelo ONNX.
/// Prueba varias ubicaciones (datos de la app, recursos, CWD de desarrollo, junto al binario).
fn resolve_model_paths(app: &AppHandle) -> Result<(PathBuf, PathBuf, PathBuf), String> {
    let mut dirs: Vec<PathBuf> = Vec::new();
    if let Ok(d) = app.path().app_data_dir() {
        dirs.push(d.join("models"));
    }
    if let Ok(d) = app.path().resource_dir() {
        dirs.push(d.join("models"));
    }
    if let Ok(cwd) = std::env::current_dir() {
        dirs.push(cwd.join("models"));
        dirs.push(cwd.join("src-tauri").join("models"));
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(p) = exe.parent() {
            dirs.push(p.join("models"));
        }
    }

    for dir in &dirs {
        let vad = dir.join("silero_vad.onnx");
        let tokens = dir.join("tokens.txt");
        if !vad.exists() || !tokens.exists() {
            continue;
        }

        // Localiza el modelo acústico: preferimos model.int8.onnx, si no,
        // cualquier .onnx que no sea el VAD.
        let mut model = dir.join("model.int8.onnx");
        if !model.exists() {
            if let Ok(rd) = std::fs::read_dir(dir) {
                for e in rd.flatten() {
                    let p = e.path();
                    let is_onnx = p.extension().and_then(|x| x.to_str()) == Some("onnx");
                    let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
                    if is_onnx && name != "silero_vad.onnx" {
                        model = p;
                        break;
                    }
                }
            }
        }

        if model.exists() {
            return Ok((vad, model, tokens));
        }
    }

    Err("No se encontró el modelo. Descárgalo en Ajustes › Modelos.".to_string())
}

/// Carga el modelo y arranca el hilo trabajador de transcripción.
pub fn start_session(app: AppHandle) -> Result<(), String> {
    let (vad_path, model_path, tokens_path) = resolve_model_paths(&app)?;

    let mut vad_cfg = SileroVadConfig::default();
    vad_cfg.model = vad_path.to_string_lossy().to_string();
    vad_cfg.sample_rate = 16000;
    // El valor por defecto de sherpa-rs (0.5 s) corta el habla en trozos diminutos.
    vad_cfg.max_speech_duration = 20.0;
    let vad = SileroVad::new(vad_cfg, 30.0).map_err(|e| e.to_string())?;

    let rec_cfg = DolphinConfig {
        model: model_path.to_string_lossy().to_string(),
        tokens: tokens_path.to_string_lossy().to_string(),
        ..Default::default()
    };
    let mut recognizer = DolphinRecognizer::new(rec_cfg).map_err(|e| e.to_string())?;

    // Hilo trabajador: posee el reconocedor y transcribe los segmentos recibidos.
    let (tx, rx) = mpsc::channel::<Vec<f32>>();
    let worker_app = app.clone();
    thread::spawn(move || {
        while let Ok(samples) = rx.recv() {
            if samples.is_empty() {
                continue;
            }
            let res = recognizer.transcribe(16000, &samples);
            let text = res.text.trim().to_string();
            if !text.is_empty() {
                let _ = worker_app.emit("final-transcript", text);
            }
        }
    });

    let mut guard = STATE.lock().map_err(|e| e.to_string())?;
    *guard = Some(Session { vad, tx });
    Ok(())
}

/// Entrega muestras de audio (16 kHz, mono, f32) al VAD.
/// Cada segmento de voz detectado se reenvía al hilo trabajador.
pub fn accept_samples(samples: &[f32]) {
    if samples.is_empty() {
        return;
    }
    let mut guard = match STATE.lock() {
        Ok(g) => g,
        Err(_) => return,
    };
    let Some(st) = guard.as_mut() else {
        return;
    };

    st.vad.accept_waveform(samples.to_vec());

    while !st.vad.is_empty() {
        let seg = st.vad.front();
        st.vad.pop(); // imprescindible: si no, el bucle nunca termina
        if !seg.samples.is_empty() {
            let _ = st.tx.send(seg.samples);
        }
    }
}

/// Vacía el segmento pendiente y detiene la sesión (el hilo trabajador termina
/// al soltarse el `Sender`).
pub fn stop_session() {
    if let Ok(mut guard) = STATE.lock() {
        if let Some(mut st) = guard.take() {
            st.vad.flush();
            while !st.vad.is_empty() {
                let seg = st.vad.front();
                st.vad.pop();
                if !seg.samples.is_empty() {
                    let _ = st.tx.send(seg.samples);
                }
            }
        }
    }
}
