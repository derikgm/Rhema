use crate::whisper_es::WhisperRecognizer;
use sherpa_rs::silero_vad::{SileroVad, SileroVadConfig};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Sender};
use std::sync::Mutex;
use std::thread;
use tauri::{AppHandle, Emitter, Manager};

/// Idioma fijado para la transcripción.
const LANGUAGE: &str = "es";

/// Sesión de reconocimiento activa.
///
/// El VAD corre en el hilo de audio (rápido) y envía cada segmento de voz a un
/// hilo trabajador que posee el reconocedor (lento), para no bloquear la captura.
struct Session {
    vad: SileroVad,
    tx: Sender<Vec<f32>>,
}

static STATE: Mutex<Option<Session>> = Mutex::new(None);

/// Busca en `root` y sus subdirectorios (profundidad limitada) un archivo cuyo
/// nombre cumple `pred`.
fn find_file(root: &Path, depth: u32, pred: &dyn Fn(&str) -> bool) -> Option<PathBuf> {
    if depth > 2 {
        return None;
    }
    let entries = std::fs::read_dir(root).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if let Some(found) = find_file(&path, depth + 1, pred) {
                return Some(found);
            }
            continue;
        }
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if pred(name) {
            return Some(path);
        }
    }
    None
}

/// Localiza el VAD, el encoder/decoder de Whisper y sus tokens en varios
/// directorios candidatos (datos de la app, recursos, CWD de desarrollo,
/// junto al binario).
struct ModelPaths {
    vad: PathBuf,
    encoder: PathBuf,
    decoder: PathBuf,
    tokens: PathBuf,
}

fn resolve_model_paths(app: &AppHandle) -> Result<ModelPaths, String> {
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
        if let Some(parent) = exe.parent() {
            dirs.push(parent.join("models"));
        }
    }

    let is_encoder = |n: &str| n.contains("encoder") && n.ends_with(".onnx");
    let is_decoder = |n: &str| n.contains("decoder") && n.ends_with(".onnx");
    let is_vad = |n: &str| n == "silero_vad.onnx";
    let is_tokens = |n: &str| n.ends_with("tokens.txt");

    for dir in &dirs {
        // El encoder/decoder identifica la carpeta del modelo Whisper.
        let Some(encoder) = find_file(dir, 0, &is_encoder) else {
            continue;
        };
        let Some(decoder) = find_file(dir, 0, &is_decoder) else {
            continue;
        };
        let Some(vad) = find_file(dir, 0, &is_vad) else {
            continue;
        };
        // Los tokens deben ser los del propio Whisper (su orden de vocabulario
        // no es intercambiable con los de otro modelo).
        let encoder_dir = encoder.parent().unwrap_or(dir);
        let Some(tokens) = find_file(encoder_dir, 0, &is_tokens)
            .or_else(|| find_file(dir, 0, &is_tokens))
        else {
            continue;
        };

        return Ok(ModelPaths {
            vad,
            encoder,
            decoder,
            tokens,
        });
    }

    Err("No se encontró el modelo Whisper. Descárgalo en Ajustes › Modelos.".to_string())
}

/// Carga el modelo y arranca el hilo trabajador de transcripción.
pub fn start_session(app: AppHandle) -> Result<(), String> {
    let paths = resolve_model_paths(&app)?;

    let mut vad_cfg = SileroVadConfig::default();
    vad_cfg.model = paths.vad.to_string_lossy().to_string();
    vad_cfg.sample_rate = 16000;
    // El valor por defecto de sherpa-rs (0.5 s) corta el habla en trozos diminutos.
    vad_cfg.max_speech_duration = 20.0;
    let vad = SileroVad::new(vad_cfg, 30.0).map_err(|e| e.to_string())?;

    let mut recognizer = WhisperRecognizer::new(
        &paths.encoder.to_string_lossy(),
        &paths.decoder.to_string_lossy(),
        &paths.tokens.to_string_lossy(),
        LANGUAGE,
    )?;

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
