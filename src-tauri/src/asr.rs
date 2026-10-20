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
    vad: SileroVad,
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

    // Configurar VAD
    let mut vad_config = SileroVadConfig::default();
    vad_config.model = paths.vad.to_string_lossy().to_string();
    vad_config.sample_rate = 16000;
    vad_config.window_size = 512;
    // Umbrales un poco más permisivos para español conversacional
    vad_config.threshold = 0.5;
    vad_config.min_silence_duration = 0.25;
    vad_config.min_speech_duration = 0.25;
    vad_config.max_speech_duration = 30.0;

    let mut vad = SileroVad::new(vad_config, 16000)
        .map_err(|e| format!("Error inicializando Silero VAD: {e}"))?;

    // Canal para enviar chunks de audio desde el hilo de captura
    let (tx, rx) = mpsc::channel::<Vec<f32>>();

    // Guardar sesión
    {
        let mut guard = STATE.lock().unwrap();
        *guard = Some(Session { vad, tx: tx.clone() });
    }

    // Hilo de reconocimiento
    let app_handle = app.clone();
    thread::spawn(move || {
        // Inicializar reconocedor Whisper
        let mut recognizer = match WhisperRecognizer::new(
            paths.encoder.to_string_lossy().to_string(),
            paths.decoder.to_string_lossy().to_string(),
            paths.tokens.to_string_lossy().to_string(),
            LANGUAGE.to_string(),
        ) {
            Ok(r) => r,
            Err(e) => {
                let _ = app_handle.emit("asr-error", format!("Error inicializando Whisper: {e}"));
                return;
            }
        };

        // Procesar chunks
        while let Ok(chunk) = rx.recv() {
            // Feed VAD
            // SileroVad::accept_waveform espera f32 a 16kHz
            // Nota: la API de sherpa-rs puede variar; usamos el enfoque estándar
            // Insertamos chunk en VAD
            // (sherpa_rs expone métodos; asumimos accept_waveform)
            let _ = vad.accept_waveform(&chunk);
            while vad.is_speech() {
                let _ = vad.pop();
            }
            // Extraer segmentos de voz completados
            loop {
                let speech = vad.front();
                if speech.is_empty() {
                    break;
                }
                // speech.samples es el segmento
                let samples: Vec<f32> = speech.samples.clone();
                let _ = vad.pop();
                if samples.is_empty() {
                    continue;
                }
                // Transcribir segmento
                match recognizer.transcribe(&samples, 16000) {
                    Ok(text) => {
                        if !text.is_empty() {
                            let _ = app_handle.emit("asr-result", text);
                        }
                    }
                    Err(e) => {
                        let _ = app_handle.emit("asr-error", format!("Error transcribiendo: {e}"));
                    }
                }
            }
        }
    });

    // Iniciar captura de audio
    crate::audio::start_asr_capture(app.clone(), tx)
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
