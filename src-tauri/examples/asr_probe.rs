// Diagnóstico: ejecuta el VAD + reconocedor sobre un WAV y muestra el texto.
// Uso: cargo run --example asr_probe -- [dir_modelos] [wav]
use sherpa_rs::dolphin::{DolphinConfig, DolphinRecognizer};
use sherpa_rs::silero_vad::{SileroVad, SileroVadConfig};
use std::fs;

/// Lee un WAV PCM 16-bit mono y lo convierte a f32 en el rango [-1, 1].
fn read_wav_16k_mono(path: &str) -> Vec<f32> {
    let bytes = fs::read(path).expect("no se pudo leer el WAV");
    let mut i = 12usize;
    let mut data: Option<(usize, usize)> = None;
    while i + 8 <= bytes.len() {
        let id = &bytes[i..i + 4];
        let sz = u32::from_le_bytes([bytes[i + 4], bytes[i + 5], bytes[i + 6], bytes[i + 7]]) as usize;
        if id == b"data" {
            data = Some((i + 8, sz));
            break;
        }
        i += 8 + sz + (sz & 1);
    }
    let (start, len) = data.expect("no se encontró el chunk de datos");
    let end = (start + len).min(bytes.len());
    let mut out = Vec::with_capacity((end - start) / 2);
    let mut j = start;
    while j + 1 < end {
        out.push(i16::from_le_bytes([bytes[j], bytes[j + 1]]) as f32 / 32768.0);
        j += 2;
    }
    out
}

fn main() {
    let base = std::env::args().nth(1).unwrap_or_else(|| "models".to_string());
    let wav = std::env::args()
        .nth(2)
        .unwrap_or_else(|| format!("{base}/test_wavs/0.wav"));

    let mut vcfg = SileroVadConfig::default();
    vcfg.model = format!("{base}/silero_vad.onnx");
    vcfg.sample_rate = 16000;
    vcfg.max_speech_duration = 20.0;
    let mut vad = SileroVad::new(vcfg, 30.0).expect("no se pudo crear el VAD");

    let rcfg = DolphinConfig {
        model: format!("{base}/model.int8.onnx"),
        tokens: format!("{base}/tokens.txt"),
        ..Default::default()
    };
    let mut rec = DolphinRecognizer::new(rcfg).expect("no se pudo crear el reconocedor");

    let samples = read_wav_16k_mono(&wav);
    println!(
        "muestras: {} ({:.2} s)",
        samples.len(),
        samples.len() as f32 / 16000.0
    );

    // Alimenta en bloques de 512 (window_size) e imprime cada segmento detectado.
    let mut i = 0usize;
    let mut nseg = 0;
    while i < samples.len() {
        let end = (i + 512).min(samples.len());
        vad.accept_waveform(samples[i..end].to_vec());
        while !vad.is_empty() {
            let seg = vad.front();
            vad.pop();
            if seg.samples.is_empty() {
                continue;
            }
            nseg += 1;
            let res = rec.transcribe(16000, &seg.samples);
            println!(
                "segmento {nseg}: {:.2} s -> {}",
                seg.samples.len() as f32 / 16000.0,
                res.text
            );
        }
        i = end;
    }
    vad.flush();
    while !vad.is_empty() {
        let seg = vad.front();
        vad.pop();
        if seg.samples.is_empty() {
            continue;
        }
        nseg += 1;
        let res = rec.transcribe(16000, &seg.samples);
        println!("segmento {nseg} (flush): {}", res.text);
    }
    println!("total de segmentos: {nseg}");
}
