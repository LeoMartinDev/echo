use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{mpsc, Arc, Mutex};
use tauri::{AppHandle, Emitter};

pub const TARGET_RATE: u32 = 16_000;

/// Starts microphone capture on a dedicated thread (cpal::Stream is not Send).
/// Mono samples at the native rate accumulate in `samples`; the native rate is
/// published in `src_rate`. Sending () on the returned Sender stops capture.
pub fn start_capture(
    app: AppHandle,
    samples: Arc<Mutex<Vec<f32>>>,
    src_rate: Arc<AtomicU32>,
) -> Result<Sender<()>, String> {
    let (stop_tx, stop_rx) = mpsc::channel::<()>();
    let (ready_tx, ready_rx) = mpsc::channel::<Result<(), String>>();

    std::thread::spawn(move || {
        capture_thread(app, samples, src_rate, stop_rx, ready_tx);
    });

    ready_rx
        .recv()
        .map_err(|_| "Le thread audio s'est arrêté de façon inattendue.".to_string())??;
    Ok(stop_tx)
}

fn capture_thread(
    app: AppHandle,
    samples: Arc<Mutex<Vec<f32>>>,
    src_rate: Arc<AtomicU32>,
    stop_rx: Receiver<()>,
    ready_tx: Sender<Result<(), String>>,
) {
    let host = cpal::default_host();
    let Some(device) = host.default_input_device() else {
        let _ = ready_tx.send(Err("Aucun micro détecté.".to_string()));
        return;
    };
    let config = match device.default_input_config() {
        Ok(c) => c,
        Err(e) => {
            let _ = ready_tx.send(Err(format!("Micro inaccessible : {e}")));
            return;
        }
    };

    src_rate.store(config.sample_rate(), Ordering::Relaxed);
    let channels = config.channels() as usize;
    let sample_format = config.sample_format();
    let stream_config: cpal::StreamConfig = config.into();

    let err_fn = |e| eprintln!("[greffe] erreur du flux audio : {e}");

    let stream = match sample_format {
        cpal::SampleFormat::F32 => build_stream::<f32>(&device, stream_config, app, samples, channels, err_fn),
        cpal::SampleFormat::I16 => build_stream::<i16>(&device, stream_config, app, samples, channels, err_fn),
        cpal::SampleFormat::U16 => build_stream::<u16>(&device, stream_config, app, samples, channels, err_fn),
        other => Err(format!("Format audio non géré : {other:?}")),
    };

    let stream = match stream {
        Ok(s) => s,
        Err(e) => {
            let _ = ready_tx.send(Err(e));
            return;
        }
    };

    if let Err(e) = stream.play() {
        let _ = ready_tx.send(Err(format!("Impossible de démarrer la capture : {e}")));
        return;
    }
    let _ = ready_tx.send(Ok(()));

    // Block until the stop signal (or channel close).
    let _ = stop_rx.recv();
    drop(stream);
}

fn build_stream<T>(
    device: &cpal::Device,
    config: cpal::StreamConfig,
    app: AppHandle,
    samples: Arc<Mutex<Vec<f32>>>,
    channels: usize,
    err_fn: fn(cpal::Error),
) -> Result<cpal::Stream, String>
where
    T: cpal::SizedSample,
    f32: cpal::FromSample<T>,
{
    let mut emit_counter: u32 = 0;
    device
        .build_input_stream(
            config,
            move |data: &[T], _| {
                let mut sum_sq = 0.0f32;
                let mut count = 0usize;
                {
                    let mut buf = samples.lock().unwrap_or_else(|e| e.into_inner());
                    for frame in data.chunks(channels) {
                        let mono: f32 = frame
                            .iter()
                            .map(|s| <f32 as cpal::FromSample<T>>::from_sample_(*s))
                            .sum::<f32>()
                            / channels as f32;
                        buf.push(mono);
                        sum_sq += mono * mono;
                        count += 1;
                    }
                }
                // RMS level for the overlay animation, throttled (~every other chunk).
                emit_counter = emit_counter.wrapping_add(1);
                if count > 0 && emit_counter % 2 == 0 {
                    let rms = (sum_sq / count as f32).sqrt();
                    let _ = app.emit("greffe://level", rms);
                }
            },
            err_fn,
            None,
        )
        .map_err(|e| format!("Impossible d'ouvrir le micro : {e}"))
}

/// Linear resampling to 16 kHz mono (sufficient for speech).
pub fn resample_to_16k(input: &[f32], src_rate: u32) -> Vec<f32> {
    if src_rate == TARGET_RATE || input.is_empty() {
        return input.to_vec();
    }
    let ratio = src_rate as f64 / TARGET_RATE as f64;
    let out_len = (input.len() as f64 / ratio).floor() as usize;
    let mut out = Vec::with_capacity(out_len);
    for i in 0..out_len {
        let pos = i as f64 * ratio;
        let idx = pos as usize;
        let frac = (pos - idx as f64) as f32;
        let a = input[idx];
        let b = if idx + 1 < input.len() { input[idx + 1] } else { a };
        out.push(a + (b - a) * frac);
    }
    out
}
