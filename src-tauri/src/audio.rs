use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::Mutex;
use tauri::{AppHandle, Emitter};

static STREAM: Mutex<Option<cpal::Stream>> = Mutex::new(None);

pub fn start_listening(app: AppHandle) -> Result<(), String> {
    let mut guard = STREAM.lock().map_err(|e| e.to_string())?;
    if guard.is_some() {
        return Ok(());
    }

    let host = cpal::default_host();
    let device = host
        .default_input_device()
        .ok_or_else(|| "No input device available".to_string())?;

    let config = device
        .default_input_config()
        .map_err(|e| e.to_string())?;

    let stream = device
        .build_input_stream(
            config.into(),
            move |data: &[f32], _| {
                if data.is_empty() {
                    return;
                }
                let sum: f32 = data.iter().map(|s| s * s).sum();
                let rms = (sum / data.len() as f32).sqrt();
                let level = (rms * 4.0).clamp(0.0, 1.0);
                let _ = app.emit("audio-level", level);
                crate::asr::accept_samples(data);
            },
            |err| eprintln!("Audio stream error: {err}"),
            None,
        )
        .map_err(|e| e.to_string())?;

    stream.play().map_err(|e| e.to_string())?;
    *guard = Some(stream);
    Ok(())
}

pub fn stop_listening() -> Result<(), String> {
    let mut guard = STREAM.lock().map_err(|e| e.to_string())?;
    *guard = None;
    Ok(())
}
