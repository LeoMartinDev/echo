use crate::settings::InsertionMode;
use crate::state::AppState;
use crate::{audio, typing};
use serde::Serialize;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition};
use transcribe_rs::transcriber::{Transcriber, VadChunked, VadChunkedConfig};
use transcribe_rs::vad::{EnergyVad, SmoothedVad};
use transcribe_rs::TranscribeOptions;

/// Maximum dictation duration.
const MAX_RECORDING: Duration = Duration::from_secs(10 * 60);


#[derive(Clone, Serialize)]
struct Phase<'a> {
    phase: &'a str,
    message: Option<&'a str>,
}

fn emit_phase(app: &AppHandle, phase: &str, message: Option<&str>) {
    let _ = app.emit("echo://phase", &Phase { phase, message });
}

fn emit_partial(app: &AppHandle, text: &str) {
    let _ = app.emit("echo://partial", text);
}

fn is_current(state: &AppState, generation: u32) -> bool {
    state.generation.load(Ordering::SeqCst) == generation
}

pub fn start(app: &AppHandle) {
    let state = app.state::<AppState>();

    if state.recording.swap(true, Ordering::SeqCst) {
        return;
    }
    let generation = state.generation.fetch_add(1, Ordering::SeqCst).wrapping_add(1);

    state.samples.lock().unwrap_or_else(|e| e.into_inner()).clear();

    let stop_tx = match audio::start_capture(
        app.clone(),
        state.samples.clone(),
        state.src_rate.clone(),
    ) {
        Ok(tx) => tx,
        Err(e) => {
            state.recording.store(false, Ordering::SeqCst);
            show_error(app, &e);
            return;
        }
    };
    *state.capture_stop.lock().unwrap_or_else(|e| e.into_inner()) = Some(stop_tx);

    show_overlay(app);
    emit_phase(app, "recording", None);
    emit_partial(app, "");

    let app = app.clone();
    std::thread::spawn(move || worker(app, generation));
}

pub fn stop(app: &AppHandle) {
    let state = app.state::<AppState>();
    state.recording.store(false, Ordering::SeqCst);
    let tx = state.capture_stop.lock().unwrap_or_else(|e| e.into_inner()).take();
    if let Some(tx) = tx {
        let _ = tx.send(());
    }
}

fn worker(app: AppHandle, generation: u32) {
    let state = app.state::<AppState>();
    let (model_id, language, insertion_mode, history_enabled) = {
        let s = state.settings.lock().unwrap_or_else(|e| e.into_inner());
        (s.model_id.clone(), s.language.clone(), s.insertion_mode, s.history_enabled)
    };

    let focus_bad = crate::focus::editable_focused() == Some(false);
    let _ = app.emit("echo://focus", !focus_bad);

    let mut slot = state.engine.lock().unwrap_or_else(|e| e.into_inner());

    let needs_load = !(slot.model_id.as_deref() == Some(model_id.as_str()) && slot.engine.is_some());
    if needs_load && is_current(&state, generation) {
        emit_phase(&app, "loading_model", None);
    }
    if let Err(e) = slot.ensure_loaded(&app, &model_id) {
        if is_current(&state, generation) {
            stop(&app);
            show_error(&app, &e);
        }
        return;
    }
    if state.recording.load(Ordering::SeqCst) && is_current(&state, generation) {
        emit_phase(&app, "recording", None);
    }

    let engine = slot.engine.as_mut().expect("engine loaded above");
    let options = TranscribeOptions { language, ..Default::default() };

    let mut typer = match typing::Typer::new() {
        Ok(t) => Some(t),
        Err(e) => {
            eprintln!("[echo] {e}");
            None
        }
    };
    let live = insertion_mode == InsertionMode::Live;

    let vad = SmoothedVad::new(Box::new(EnergyVad::new(480, 0.012)), 5, 15, 2);
    let vad_config = VadChunkedConfig {
        min_chunk_secs: 0.3,
        max_chunk_secs: 8.0,
        padding_secs: 0.25,
        smart_split_search_secs: Some(2.0),
        merge_separator: " ".into(),
    };
    let mut transcriber = VadChunked::new(Box::new(vad), vad_config, options);

    let started = Instant::now();
    chunked_loop(&app, &state, engine, &mut transcriber, live, &mut typer, started);

    if is_current(&state, generation) {
        emit_phase(&app, "transcribing", None);
    }

    let final_text = match transcriber.finish(&mut **engine) {
        Ok(r) => r.text.trim().to_string(),
        Err(e) => {
            if is_current(&state, generation) {
                show_error(&app, &format!("Transcription impossible : {e}"));
            }
            return;
        }
    };
    drop(slot);

    emit_partial(&app, &final_text);

    if history_enabled && !final_text.is_empty() {
        let total_samples = state.samples.lock().unwrap_or_else(|e| e.into_inner()).len();
        let rate = state.src_rate.load(Ordering::Relaxed).max(8000);
        crate::history::append(&app, &final_text, &model_id, total_samples as f32 / rate as f32);
        let _ = app.emit("echo://history", ());
    }

    let mut typed_ok = false;
    if let Some(t) = typer.as_mut() {
        let result = if live {
            t.reconcile_to(&final_text)
        } else if !final_text.is_empty() {
            t.extend_to(&final_text)
        } else {
            Ok(())
        };
        match result {
            Ok(()) => typed_ok = true,
            Err(e) => eprintln!("[echo] {e}"),
        }
    }

    if !final_text.is_empty() && (focus_bad || !typed_ok) {
        use tauri_plugin_clipboard_manager::ClipboardExt;
        if let Err(e) = app.clipboard().write_text(final_text.clone()) {
            eprintln!("[echo] clipboard: {e}");
        }
    }

    if is_current(&state, generation) {
        emit_phase(&app, "idle", None);
        hide_overlay(&app);
    }
}

/// Chunked loop (all batch models): feeds audio to a VAD-based transcriber
/// that segments on speech pauses and decodes each segment once.
/// Text appears phrase by phrase with constant decode time.
fn chunked_loop(
    app: &AppHandle,
    state: &AppState,
    engine: &mut Box<dyn transcribe_rs::SpeechModel + Send>,
    transcriber: &mut VadChunked,
    live: bool,
    typer: &mut Option<typing::Typer>,
    started: Instant,
) {
    let mut committed_text = String::new();
    let mut committed_src = 0usize;

    while state.recording.load(Ordering::SeqCst) {
        std::thread::sleep(Duration::from_millis(80));

        if started.elapsed() > MAX_RECORDING {
            stop(app);
            break;
        }

        let (window, rate) = snapshot_from(state, committed_src);
        if window.is_empty() {
            continue;
        }
        committed_src += window.len();

        let audio16k = audio::resample_to_16k(&window, rate);
        let results = match transcriber.feed(&mut **engine, &audio16k) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("[echo] chunk error: {e}");
                continue;
            }
        };

        for result in results {
            let text = result.text.trim().to_string();
            if text.is_empty() {
                continue;
            }
            committed_text = join_text(&committed_text, &text);
            emit_partial(app, &committed_text);

            if live {
                if let Some(t) = typer.as_mut() {
                    if let Err(e) = t.extend_to(&committed_text) {
                        eprintln!("[echo] {e}");
                    }
                }
            }
        }
    }
}

fn snapshot_from(state: &AppState, from: usize) -> (Vec<f32>, u32) {
    let buf = state.samples.lock().unwrap_or_else(|e| e.into_inner());
    let chunk = buf.get(from..).unwrap_or(&[]).to_vec();
    let rate = state.src_rate.load(Ordering::Relaxed).max(8000);
    (chunk, rate)
}

fn join_text(a: &str, b: &str) -> String {
    let (a, b) = (a.trim(), b.trim());
    if a.is_empty() {
        b.to_string()
    } else if b.is_empty() {
        a.to_string()
    } else {
        format!("{a} {b}")
    }
}

fn show_error(app: &AppHandle, message: &str) {
    eprintln!("[echo] error: {message}");
    show_overlay(app);
    emit_phase(app, "error", Some(message));
    let app = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(2800));
        emit_phase(&app, "idle", None);
        hide_overlay(&app);
    });
}

fn show_overlay(app: &AppHandle) {
    let Some(win) = app.get_webview_window("overlay") else { return };

    let monitor = app
        .cursor_position()
        .ok()
        .and_then(|pos| app.monitor_from_point(pos.x, pos.y).ok().flatten())
        .or_else(|| win.primary_monitor().ok().flatten());

    if let (Some(monitor), Ok(size)) = (monitor, win.outer_size()) {
        let mpos = monitor.position();
        let msize = monitor.size();
        let margin = (40.0 * monitor.scale_factor()) as i32;
        let x = mpos.x + ((msize.width as i32 - size.width as i32) / 2);
        let y = mpos.y + msize.height as i32 - size.height as i32 - margin;
        let _ = win.set_position(PhysicalPosition::new(x, y));
    }

    #[cfg(target_os = "linux")]
    {
        let _ = win.show();
        let _ = win.set_ignore_cursor_events(true);
    }

    #[cfg(not(target_os = "linux"))]
    {
        let _ = win.set_ignore_cursor_events(true);
        let _ = win.show();
    }

    let _ = win.set_always_on_top(true);
}

fn hide_overlay(app: &AppHandle) {
    if let Some(win) = app.get_webview_window("overlay") {
        let _ = win.hide();
    }
}
