//! Reconocedor Whisper forzado a un idioma (español).
//!
//! `sherpa-rs` expone `WhisperConfig` pero fija `feature_dim: 512` de forma
//! fija; sherpa-onnx usa ese valor como número de filtros mel
//! (`features.cc: InitWhisper() -> opts_.mel_opts.num_bins`), y Whisper espera
//! **80**. Con 512 la normalización del modelo fallaría, por lo que aquí
//! construimos el reconocedor directamente contra `sherpa_rs_sys` y lo
//! correctamos a 80, además de poder forzar el idioma.

use sherpa_rs::get_default_provider;
use sherpa_rs::sherpa_rs_sys as sys;
use std::ffi::{c_char, CStr, CString};

pub struct WhisperRecognizer {
    recognizer: *const sys::SherpaOnnxOfflineRecognizer,
}

#[derive(Debug, Clone, Default)]
pub struct WhisperResult {
    pub text: String,
}

impl WhisperRecognizer {
    /// Carga un Whisper en formato encoder/decoder ONNX y fija el idioma.
    pub fn new(
        encoder: &str,
        decoder: &str,
        tokens: &str,
        language: &str,
    ) -> Result<Self, String> {
        let c = |s: &str| CString::new(s).map_err(|e| format!("cadena C inválida ({s}): {e}"));

        let provider_c = c(&get_default_provider())?;
        let encoder_c = c(encoder)?;
        let decoder_c = c(decoder)?;
        let tokens_c = c(tokens)?;
        let language_c = c(language)?;
        let task_c = c("transcribe")?;
        let decoding_method_c = c("greedy_search")?;
        // sherpa-rs pasa una cadena vacía (no null) en `bpe_vocab`; con null el
        // C++ la interpreta como std::string y lanza una excepción.
        let empty_c = c("")?;

        let num_threads: i32 = 2;

        // SAFETY: `SherpaOnnxOfflineModelConfig` y
        // `SherpaOnnxOfflineRecognizerConfig` son structs `repr(C)` de punteros
        // y enteros; todo a cero es una representación válida (punteros null =
        // modelos no usados). Sobrescribimos después solo los campos de Whisper.
        let mut model_config: sys::SherpaOnnxOfflineModelConfig = unsafe { std::mem::zeroed() };
        model_config.whisper = sys::SherpaOnnxOfflineWhisperModelConfig {
            decoder: decoder_c.as_ptr(),
            encoder: encoder_c.as_ptr(),
            language: language_c.as_ptr(),
            task: task_c.as_ptr(),
            tail_paddings: 0,
        };
        model_config.tokens = tokens_c.as_ptr();
        model_config.num_threads = num_threads;
        model_config.provider = provider_c.as_ptr();
        model_config.bpe_vocab = empty_c.as_ptr();

        let mut config: sys::SherpaOnnxOfflineRecognizerConfig = unsafe { std::mem::zeroed() };
        config.model_config = model_config;
        config.decoding_method = decoding_method_c.as_ptr();
        // 80 filtros mel: lo que espera Whisper.
        config.feat_config = sys::SherpaOnnxFeatureConfig {
            sample_rate: 16000,
            feature_dim: 80,
        };

        let recognizer = unsafe { sys::SherpaOnnxCreateOfflineRecognizer(&config) };
        if recognizer.is_null() {
            return Err("No se pudo crear el reconocedor Whisper".to_string());
        }

        Ok(Self { recognizer })
    }

    /// Transcribe un bloque de audio (16 kHz, mono, f32).
    pub fn transcribe(&mut self, sample_rate: u32, samples: &[f32]) -> WhisperResult {
        // SAFETY: la secuencia de llamadas replica la de sherpa-rs; el puntero
        // del resultado se libera tras copiar el texto.
        unsafe {
            let stream = sys::SherpaOnnxCreateOfflineStream(self.recognizer);
            sys::SherpaOnnxAcceptWaveformOffline(
                stream,
                sample_rate as i32,
                samples.as_ptr(),
                samples.len() as i32,
            );
            sys::SherpaOnnxDecodeOfflineStream(self.recognizer, stream);

            let result_ptr = sys::SherpaOnnxGetOfflineStreamResult(stream);
            let raw = result_ptr.read();
            let text = if raw.text.is_null() {
                String::new()
            } else {
                CStr::from_ptr(raw.text as *const c_char)
                    .to_string_lossy()
                    .into_owned()
            };

            sys::SherpaOnnxDestroyOfflineRecognizerResult(result_ptr);
            sys::SherpaOnnxDestroyOfflineStream(stream);

            WhisperResult { text }
        }
    }
}

unsafe impl Send for WhisperRecognizer {}
unsafe impl Sync for WhisperRecognizer {}

impl Drop for WhisperRecognizer {
    fn drop(&mut self) {
        unsafe { sys::SherpaOnnxDestroyOfflineRecognizer(self.recognizer) }
    }
}
