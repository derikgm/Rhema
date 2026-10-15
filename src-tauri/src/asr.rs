use sherpa_rs::silero_vad::SileroVad;
use sherpa_rs::silero_vad::SileroVadConfig;
use sherpa_rs::dolphin::{DolphinConfig, DolphinRecognizer};
use std::sync::Mutex;
use tauri::{AppHandle, Emitter};

struct State {
    vad: SileroVad,
    recognizer: DolphinRecognizer,
    app: AppHandle,
}

unsafe impl Send for State {}

static STATE: Mutex<Option<State>> = Mutex::new(None);

pub fn start_session(app: AppHandle) -> Result<(), String> {
    let base = "src-tauri/models";
    let mut vad_cfg = SileroVadConfig::default();
    vad_cfg.model = format!("{}/silero_vad.onnx", base);
    let vad = SileroVad::new(vad_cfg, 30.0).map_err(|e| e.to_string())?;

    let rec_cfg = DolphinConfig {
        model: format!("{}/model.int8.onnx", base),
        tokens: format!("{}/tokens.txt", base),
        ..Default::default()
    };
    let recognizer = DolphinRecognizer::new(rec_cfg).map_err(|e| e.to_string())?;

    let mut guard = STATE.lock().map_err(|e| e.to_string())?;
    *guard = Some(State { vad, recognizer, app });
    Ok(())
}

pub fn accept_samples(samples: &[f32]) {
    let mut guard = STATE.lock().unwrap();
    let Some(st) = guard.as_mut() else { return; };
    st.vad.accept_waveform(samples.to_vec());
    while !st.vad.is_empty() {
        let seg = st.vad.front();
        if !seg.samples.is_empty() {
            let res = st.recognizer.transcribe(16000, &seg.samples);
            if !res.text.trim().is_empty() {
                let _ = st.app.emit("final-transcript", res.text);
            }
        }
    }
    st.vad.flush();
}

pub fn stop_session() {
    let _ = STATE.lock().map_err(|_| ()).map(|mut g| g.take());
}

pub fn emit_partial(_app: &AppHandle, _text: &str) {}

pub fn emit_final(_app: &AppHandle, _text: &str) {}
