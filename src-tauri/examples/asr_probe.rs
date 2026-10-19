// Diagnóstico: ejecuta el VAD + reconocedor Whisper sobre un WAV y muestra el
// texto. Usa el mismo `WhisperRecognizer` que la app (idioma "es").
//
// Uso: cargo build --example asr_probe
//      LD_LIBRARY_PATH=target/debug/examples \
//        target/debug/examples/asr_probe <dir_modelo> <audio.wav>
use rhema_lib::whisper_es::WhisperRecognizer;
use sherpa_rs::silero_vad::{SileroVad, SileroVadConfig};
use std::fs;
use std::path::{Path, PathBuf};

/// Idioma por defecto de la prueba.
const LANGUAGE: &str = "es";

/// Idioma de la prueba (se puede sobreescribir con el 3.er argumento).
fn language_arg() -> String {
    std::env::args().nth(3).unwrap_or_else(|| LANGUAGE.to_string())
}

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

/// Busca `silero_vad.onnx` en `dir` y, si no está, en su carpeta padre.
fn find_vad(dir: &Path) -> PathBuf {
    let local = dir.join("silero_vad.onnx");
    if local.exists() {
        return local;
    }
    if let Some(parent) = dir.parent() {
        let up = parent.join("silero_vad.onnx");
        if up.exists() {
            return up;
        }
    }
    panic!("no se encontró silero_vad.onnx en {} ni en su padre", dir.display());
}

fn main() {
    let base = std::env::args().nth(1).unwrap_or_else(|| "models".to_string());
    let wav = std::env::args()
        .nth(2)
        .unwrap_or_else(|| format!("{base}/test_wavs/0.wav"));
    let dir = PathBuf::from(&base);

    let mut vcfg = SileroVadConfig::default();
    vcfg.model = find_vad(&dir).to_string_lossy().to_string();
    vcfg.sample_rate = 16000;
    vcfg.max_speech_duration = 20.0;
    eprintln!("[probe] creando VAD con {}", vcfg.model);
    let mut vad = SileroVad::new(vcfg, 30.0).expect("no se pudo crear el VAD");
    eprintln!("[probe] VAD ok");

    // Localiza encoder/decoder/tokens del Whisper dentro del directorio.
    let find = |pred: &dyn Fn(&str) -> bool| -> Option<PathBuf> {
        fs::read_dir(&dir).ok()?.flatten().map(|e| e.path()).find(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .map(|n| pred(n))
                .unwrap_or(false)
        })
    };
    let encoder = find(&|n| n.contains("encoder") && n.ends_with(".onnx"))
        .expect("falta el encoder ONNX del Whisper");
    let decoder = find(&|n| n.contains("decoder") && n.ends_with(".onnx"))
        .expect("falta el decoder ONNX del Whisper");
    let tokens = find(&|n| n.ends_with("tokens.txt")).expect("faltan los tokens del Whisper");

    eprintln!("[probe] encoder={encoder:?} decoder={decoder:?} tokens={tokens:?}");
    let mut rec = WhisperRecognizer::new(
        &encoder.to_string_lossy(),
        &decoder.to_string_lossy(),
        &tokens.to_string_lossy(),
        &language_arg(),
    )
    .expect("no se pudo crear el reconocedor Whisper");
    eprintln!("[probe] reconocedor ok");

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
