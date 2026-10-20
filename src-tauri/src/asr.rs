use crate::models;
use crate::whisper_es::WhisperRecognizer;
use sherpa_rs::silero_vad::{SileroVad, SileroVadConfig};
use std::path::PathBuf;
use std::sync::mpsc::{self, Sender};
use std::sync::Mutex;
use std::thread;
use tauri::{AppHandle, Emitter, Manager};

/// Idioma fijado para la transcripción.
const LANGUAGE: &str = "es";

/// Sesión de reconocimiento activa.
struct Session {
    tx: Sender<Vec<f32>>,
}

static STATE: Mutex<Option<Session>> = Mutex::new(None);

struct ModelPaths {
    vad: PathBuf,
    encoder: PathBuf,
    decoder: PathBuf,
    tokens: PathBuf,
}

/// Localiza los archivos del modelo activo (carpeta seleccionada en Ajustes).
fn resolve_model_paths(app: &AppHandle) -> Result<ModelPaths, String> {
    let folder = models::active_model_folder_path(app).ok_or_else(|| {
        "No hay ningún modelo activo. Descarga o importa uno en Ajustes › Modelos.".to_string()
    })?;
    let mf = models::resolve_model_files_in_folder(&folder)?;
    Ok(ModelPaths {
        vad: mf.vad,
        encoder: mf.encoder,
        decoder: mf.decoder,
        tokens: mf.tokens,
    })
}

/// Inicia una sesión de ASR con VAD + Whisper.
pub fn start_session(app: AppHandle) -> Result<(), String> {
    // Detener sesión previa si existe
    {
        let mut guard = STATE.lock().unwrap();
        *guard = None;
    }

    let paths = resolve_model_paths(&app)?;

    // Canal para enviar chunks de audio desde el hilo de captura
    let (tx, rx) = mpsc::channel::<Vec<f32>>();

    // Guardar sesión con canal
    {
        let mut guard = STATE.lock().unwrap();
        *guard = Some(Session { tx: tx.clone() });
    }
    // Hilo de reconocimiento
    let app_handle = app.clone();
    let paths_clone = paths;
    thread::spawn(move || {
        // Inicializar VAD
        let mut vad_config = SileroVadConfig::default();
        vad_config.model = paths_clone.vad.to_string_lossy().to_string();
        vad_config.sample_rate = 16000;
        vad_config.window_size = 512;
        vad_config.threshold = 0.5;
        vad_config.min_silence_duration = 0.25;
        vad_config.min_speech_duration = 0.25;
        vad_config.max_speech_duration = 30.0;
        let mut vad_thread = match SileroVad::new(vad_config, 16000.0) {
            Ok(v) => v,
            Err(e) => {
                let _ = app_handle.emit("asr-error", format!("Error inicializando Silero VAD: {e}"));
                return;
            }
        };
        // Inicializar reconocedor Whisper
        let mut recognizer = match WhisperRecognizer::new(
            &paths_clone.encoder.to_string_lossy(),
            &paths_clone.decoder.to_string_lossy(),
            &paths_clone.tokens.to_string_lossy(),
            LANGUAGE,
        ) {
            Ok(r) => r,
            Err(e) => {
                let _ = app_handle.emit("asr-error", format!("Error inicializando Whisper: {e}"));
                return;
            }
        };

        // Procesar chunks
        while let Ok(chunk) = rx.recv() {
            let _ = vad_thread.accept_waveform(chunk.clone());
            while vad_thread.is_speech() {
                let _ = vad_thread.pop();
            }
            loop {
                let speech = vad_thread.front();
                let samples: Vec<f32> = speech.samples.clone();
                if samples.is_empty() {
                    break;
                }
                let _ = vad_thread.pop();
                if samples.is_empty() {
                    continue;
                }
                let result = recognizer.transcribe(16000, &samples);
                if !result.text.is_empty() {
                    let _ = app_handle.emit("asr-result", result.text);
                }
            }
        }
    });

    crate::audio::start_listening(app.clone()).map_err(|e| e.to_string())
}

/// Acepta muestras de audio desde el hilo de captura (llamado por audio.rs)
pub fn accept_samples(samples: Vec<f32>) {
    if let Ok(guard) = STATE.lock() {
        if let Some(ref session) = *guard {
            let _ = session.tx.send(samples);
        }
    }
}

/// Detiene la sesión de ASR
pub fn stop_session() {
    let mut guard = STATE.lock().unwrap();
    *guard = None;
}
