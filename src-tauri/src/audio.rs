use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, Sample, SampleFormat, SizedSample};
use std::sync::Mutex;
use tauri::{AppHandle, Emitter};

/// Muestras objetivo: el modelo ASR y el VAD de sherpa-onnx trabajan a 16 kHz.
const TARGET_RATE: u32 = 16000;

static STREAM: Mutex<Option<cpal::Stream>> = Mutex::new(None);

/// Remuestreador lineal con estado: convierte la frecuencia del dispositivo (p. ej.
/// 48 kHz) a 16 kHz manteniendo la fase entre bloques para no producir cortes.
struct LinearResampler {
    /// Muestras de entrada por cada muestra de salida.
    ratio: f64,
    /// Posición fraccionaria dentro del bloque actual.
    pos: f64,
    /// Última muestra del bloque anterior (interpolación continua).
    prev: f32,
}

impl LinearResampler {
    fn new(src_rate: u32, dst_rate: u32) -> Self {
        Self {
            ratio: src_rate as f64 / dst_rate.max(1) as f64,
            pos: 0.0,
            prev: 0.0,
        }
    }

    fn process(&mut self, input: &[f32]) -> Vec<f32> {
        let n = input.len();
        if n == 0 || self.ratio <= 0.0 {
            return Vec::new();
        }
        // Índice virtual: 0 -> muestra anterior, k>=1 -> input[k-1].
        let virt = |idx: usize, prev: f32| -> f32 {
            if idx == 0 {
                prev
            } else {
                input[idx - 1]
            }
        };

        let mut out = Vec::with_capacity((n as f64 / self.ratio) as usize + 2);
        while self.pos < n as f64 {
            let i = self.pos.floor() as usize;
            let frac = self.pos - i as f64;
            let a = virt(i, self.prev);
            let b = virt(i + 1, self.prev); // i+1 <= n, siempre válido
            out.push((a as f64 + (b as f64 - a as f64) * frac) as f32);
            self.pos += self.ratio;
        }
        self.pos -= n as f64;
        if self.pos < 0.0 {
            self.pos = 0.0;
        }
        self.prev = input[n - 1];
        out
    }
}

/// Construye el stream tipado para el formato nativo del dispositivo, lo convierte
/// a mono f32, lo remuestrea a 16 kHz y lo entrega al VAD/ASR.
fn build_stream<T>(
    device: &cpal::Device,
    config: cpal::StreamConfig,
    channels: usize,
    app: AppHandle,
) -> Result<cpal::Stream, String>
where
    T: SizedSample + Sample,
    f32: FromSample<T>,
{
    let mut resampler = LinearResampler::new(config.sample_rate, TARGET_RATE);
    let ch = channels.max(1);
    let mut ticks: u32 = 0;

    device
        .build_input_stream(
            config,
            move |data: &[T], _: &cpal::InputCallbackInfo| {
                if data.is_empty() {
                    return;
                }

                // Mezcla a mono (promedio de canales).
                let mono: Vec<f32> = if ch > 1 {
                    data.chunks(ch)
                        .map(|frame| {
                            frame.iter().map(|s| f32::from_sample(*s)).sum::<f32>()
                                / frame.len() as f32
                        })
                        .collect()
                } else {
                    data.iter().map(|s| f32::from_sample(*s)).collect()
                };

                // Nivel RMS para la UI (limitado a ~10 Hz).
                ticks = ticks.wrapping_add(1);
                if ticks % 8 == 0 {
                    let sum: f32 = mono.iter().map(|s| s * s).sum();
                    let rms = (sum / mono.len().max(1) as f32).sqrt();
                    let _ = app.emit("audio-level", (rms * 4.0).clamp(0.0, 1.0));
                }

                let resampled = resampler.process(&mono);
                crate::asr::accept_samples(&resampled);
            },
            |err| eprintln!("Error en el stream de audio: {err}"),
            None,
        )
        .map_err(|e| e.to_string())
}

pub fn start_listening(app: AppHandle) -> Result<(), String> {
    let mut guard = STREAM.lock().map_err(|e| e.to_string())?;
    if guard.is_some() {
        return Ok(());
    }

    let host = cpal::default_host();
    let device = host
        .default_input_device()
        .ok_or_else(|| "No se encontró ningún micrófono".to_string())?;

    let supported = device
        .default_input_config()
        .map_err(|e| format!("No se pudo leer la configuración del micrófono: {e}"))?;

    let sample_format = supported.sample_format();
    let channels = supported.channels() as usize;
    let config: cpal::StreamConfig = supported.config();

    let stream = match sample_format {
        SampleFormat::F32 => build_stream::<f32>(&device, config, channels, app),
        SampleFormat::I16 => build_stream::<i16>(&device, config, channels, app),
        SampleFormat::U16 => build_stream::<u16>(&device, config, channels, app),
        SampleFormat::I32 => build_stream::<i32>(&device, config, channels, app),
        SampleFormat::U32 => build_stream::<u32>(&device, config, channels, app),
        SampleFormat::I8 => build_stream::<i8>(&device, config, channels, app),
        SampleFormat::U8 => build_stream::<u8>(&device, config, channels, app),
        SampleFormat::F64 => build_stream::<f64>(&device, config, channels, app),
        other => Err(format!("Formato de audio no soportado: {other}")),
    }?;

    stream.play().map_err(|e| e.to_string())?;
    *guard = Some(stream);
    Ok(())
}

pub fn stop_listening() -> Result<(), String> {
    let mut guard = STREAM.lock().map_err(|e| e.to_string())?;
    *guard = None;
    Ok(())
}
