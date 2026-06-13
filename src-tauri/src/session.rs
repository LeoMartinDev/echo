use crate::settings::InsertionMode;
use crate::state::AppState;
use crate::{audio, typing};
use serde::Serialize;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition};
use transcribe_rs::TranscribeOptions;

/// Minimum interval between two partial decodes.
const PARTIAL_INTERVAL: Duration = Duration::from_millis(1000);
/// Maximum dictation duration.
const MAX_RECORDING: Duration = Duration::from_secs(10 * 60);
/// Below this amplitude peak the buffer is considered silence
/// (avoids Whisper hallucinations on empty audio).
const SILENCE_PEAK: f32 = 0.012;
/// Beyond this window length, look for a pause to commit decoded text and
/// shorten the window — otherwise decode cost grows with dictation length
/// and partials fall behind.
const MAX_WINDOW_SECS: f32 = 8.0;
/// Beyond this, cut at the quietest trough even without a real pause:
/// an unbounded window would freeze live mode.
const HARD_WINDOW_SECS: f32 = 14.0;
/// The most recent audio is never committed (the current sentence may
/// still change the decode).
const KEEP_TAIL_SECS: f32 = 1.2;
/// Duration of a trough considered a speech pause, and the RMS floor
/// (the actual threshold adapts to the window level, see find_silence_cut).
const GAP_SECS: f32 = 0.25;
const GAP_RMS: f32 = 0.008;

#[derive(Clone, Serialize)]
struct Phase<'a> {
    phase: &'a str, // "recording" | "loading_model" | "transcribing" | "idle" | "error"
    message: Option<&'a str>,
}

fn emit_phase(app: &AppHandle, phase: &str, message: Option<&str>) {
    let _ = app.emit("greffe://phase", &Phase { phase, message });
}

fn emit_partial(app: &AppHandle, text: &str) {
    let _ = app.emit("greffe://partial", text);
}

/// Whether the worker for `generation` is still the active session. A later
/// `start()` bumps the counter, so a lagging worker uses this to stop emitting
/// phases / hiding the overlay over the session that replaced it.
fn is_current(state: &AppState, generation: u32) -> bool {
    state.generation.load(Ordering::SeqCst) == generation
}

/// Start dictation (shortcut pressed).
pub fn start(app: &AppHandle) {
    let state = app.state::<AppState>();

    // Key auto-repeat fires "Pressed" bursts: ignore them.
    if state.recording.swap(true, Ordering::SeqCst) {
        return;
    }
    let generation = state.generation.fetch_add(1, Ordering::SeqCst).wrapping_add(1);

    state.samples.lock().unwrap_or_else(|e| e.into_inner()).clear();

    // Capture starts immediately: no waiting for model loading,
    // audio spoken during that time is not lost.
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

/// Stop dictation (shortcut released).
pub fn stop(app: &AppHandle) {
    let state = app.state::<AppState>();
    state.recording.store(false, Ordering::SeqCst);
    let tx = state.capture_stop.lock().unwrap_or_else(|e| e.into_inner()).take();
    if let Some(tx) = tx {
        let _ = tx.send(());
    }
}

/// Session worker thread: loads the engine, decodes partials during recording,
/// then produces and types the final text.
fn worker(app: AppHandle, generation: u32) {
    let state = app.state::<AppState>();
    let (model_id, language, insertion_mode, history_enabled) = {
        let s = state.settings.lock().unwrap_or_else(|e| e.into_inner());
        (s.model_id.clone(), s.language.clone(), s.insertion_mode, s.history_enabled)
    };

    // Best-effort detection: is there a focused text field to type into?
    // Some(false) → warning in the overlay + clipboard fallback at the end.
    let focus_bad = crate::focus::editable_focused() == Some(false);
    let _ = app.emit("greffe://focus", !focus_bad);

    let mut slot = state.engine.lock().unwrap_or_else(|e| e.into_inner());

    let needs_load = !(slot.model_id.as_deref() == Some(model_id.as_str()) && slot.engine.is_some());
    if needs_load && is_current(&state, generation) {
        emit_phase(&app, "loading_model", None);
    }
    if let Err(e) = slot.ensure_loaded(&app, &model_id) {
        // A superseded worker must not stop/error the session that replaced it.
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
            eprintln!("[greffe] {e}");
            None
        }
    };
    let live = insertion_mode == InsertionMode::Live;
    // Whisper decodes in fixed 30 s passes: re-decoding a growing window every
    // second is costly and unstable. For it we segment on speech pauses and
    // decode each segment exactly once. Parakeet keeps its sliding-partials
    // loop, which suits a streaming model and already works well.
    let segment_mode = matches!(
        crate::models::spec(&model_id).map(|s| s.engine),
        Some(crate::models::EngineKind::Whisper)
    );

    let started = Instant::now();
    let (committed_text, committed_src) = if segment_mode {
        segmented_loop(&app, &state, engine, &options, live, &mut typer, started)
    } else {
        streaming_loop(&app, &state, engine, &options, live, &mut typer, started)
    };

    // --- Final decode (uncommitted window only) ---
    if is_current(&state, generation) {
        emit_phase(&app, "transcribing", None);
    }
    let (window, rate) = snapshot_from(&state, committed_src);

    let final_text = if peak(&window) < SILENCE_PEAK {
        committed_text
    } else {
        let audio16k = audio::resample_to_16k(&window, rate);
        match engine.transcribe(&audio16k, &options) {
            Ok(r) => join_text(&committed_text, r.text.trim()),
            Err(e) => {
                if is_current(&state, generation) {
                    show_error(&app, &format!("Transcription impossible : {e}"));
                }
                return;
            }
        }
    };
    drop(slot);

    emit_partial(&app, &final_text);

    if history_enabled && !final_text.is_empty() {
        let total_samples = committed_src + window.len();
        crate::history::append(
            &app,
            &final_text,
            &model_id,
            total_samples as f32 / rate as f32,
        );
        let _ = app.emit("greffe://history", ());
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
            Err(e) => eprintln!("[greffe] {e}"),
        }
    }

    // Safety net: if no focused field was detected, or typing failed,
    // also put the text on the clipboard.
    if !final_text.is_empty() && (focus_bad || !typed_ok) {
        use tauri_plugin_clipboard_manager::ClipboardExt;
        if let Err(e) = app.clipboard().write_text(final_text.clone()) {
            eprintln!("[greffe] clipboard: {e}");
        }
    }

    // A newer session may have started while this worker finished its final
    // decode: only it owns the overlay now, so don't reset it to idle/hidden.
    if is_current(&state, generation) {
        emit_phase(&app, "idle", None);
        hide_overlay(&app);
    }
}

/// Streaming loop (Parakeet): decodes the uncommitted window at a regular
/// interval for word-by-word output, and commits past MAX_WINDOW_SECS on a
/// pause. This is the original behavior — kept intact for streaming models.
fn streaming_loop(
    app: &AppHandle,
    state: &AppState,
    engine: &mut Box<dyn transcribe_rs::SpeechModel + Send>,
    options: &TranscribeOptions,
    live: bool,
    typer: &mut Option<typing::Typer>,
    started: Instant,
) -> (String, usize) {
    let mut last_decode = Instant::now() - PARTIAL_INTERVAL;
    // If a decode takes longer than the interval, space them accordingly.
    let mut min_wait = PARTIAL_INTERVAL;
    let mut last_decoded_len = 0usize;
    let mut prev_partial = String::new();
    // Committed text + position (in source samples) of the start
    // of the current decode window.
    let mut committed_text = String::new();
    let mut committed_src = 0usize;

    while state.recording.load(Ordering::SeqCst) {
        std::thread::sleep(Duration::from_millis(80));

        if started.elapsed() > MAX_RECORDING {
            stop(app);
            break;
        }
        if last_decode.elapsed() < min_wait {
            continue;
        }
        let (mut window, rate) = snapshot_from(state, committed_src);
        // Only decode if at least 400 ms of new audio has accumulated.
        if committed_src + window.len() < last_decoded_len + (rate as usize * 2 / 5) {
            continue;
        }
        last_decoded_len = committed_src + window.len();
        last_decode = Instant::now();

        // Window too long: commit everything up to the last speech pause,
        // and this cycle decodes only up to that pause.
        let mut committing = false;
        let window_secs = window.len() as f32 / rate as f32;
        if window_secs > MAX_WINDOW_SECS {
            let keep_tail = (KEEP_TAIL_SECS * rate as f32) as usize;
            let cut = find_silence_cut(&window, rate, keep_tail).or_else(|| {
                (window_secs > HARD_WINDOW_SECS)
                    .then(|| find_quietest_cut(&window, rate, keep_tail))
                    .flatten()
            });
            if let Some(cut) = cut {
                window.truncate(cut);
                committing = true;
            }
        }

        if peak(&window) < SILENCE_PEAK {
            if committing {
                // Silent window: nothing to transcribe, advance the cursor.
                committed_src += window.len();
            }
            continue;
        }
        let audio16k = audio::resample_to_16k(&window, rate);
        let decode_start = Instant::now();
        let Ok(result) = engine.transcribe(&audio16k, options) else {
            continue; // a failed partial is fine, the final decode will fix it
        };
        min_wait = PARTIAL_INTERVAL.max(decode_start.elapsed());

        let text = result.text.trim().to_string();
        let full_text = join_text(&committed_text, &text);
        if committing {
            committed_text = full_text.clone();
            committed_src += window.len();
        }
        if full_text.is_empty() {
            continue;
        }
        emit_partial(app, &full_text);

        if live {
            // Only type the stable prefix between two successive decodes.
            // `reconcile_to` (not `extend_to`): Whisper often rewrites punctuation
            // of already-typed words; we need to correct the tail rather than
            // stopping at the first divergence. The length guard prevents
            // trimming text on a hesitant decode.
            let stable = typing::stable_prefix(&prev_partial, &full_text);
            if let Some(t) = typer.as_mut() {
                if stable.chars().count() >= t.typed_chars() {
                    if let Err(e) = t.reconcile_to(&stable) {
                        eprintln!("[greffe] {e}");
                    }
                }
            }
        }
        prev_partial = full_text;
    }

    (committed_text, committed_src)
}

/// Segmented loop (Whisper): does NOT re-decode a growing window (costly and
/// unstable for a batch model). Instead it waits for a speech pause, decodes
/// the finished segment exactly once, commits and types it, then repeats.
/// Text appears phrase by phrase. Short utterances with no internal pause
/// commit nothing here and are handled by the single final decode.
fn segmented_loop(
    app: &AppHandle,
    state: &AppState,
    engine: &mut Box<dyn transcribe_rs::SpeechModel + Send>,
    options: &TranscribeOptions,
    live: bool,
    typer: &mut Option<typing::Typer>,
    started: Instant,
) -> (String, usize) {
    let mut committed_text = String::new();
    let mut committed_src = 0usize;

    while state.recording.load(Ordering::SeqCst) {
        std::thread::sleep(Duration::from_millis(80));

        if started.elapsed() > MAX_RECORDING {
            stop(app);
            break;
        }

        let (window, rate) = snapshot_from(state, committed_src);
        let window_secs = window.len() as f32 / rate as f32;
        let keep_tail = (KEEP_TAIL_SECS * rate as f32) as usize;

        // Cut at the last speech pause; past the hard window, force the
        // quietest dip so a long pauseless phrase doesn't accumulate forever.
        let cut = find_silence_cut(&window, rate, keep_tail).or_else(|| {
            (window_secs > HARD_WINDOW_SECS)
                .then(|| find_quietest_cut(&window, rate, keep_tail))
                .flatten()
        });
        let Some(cut) = cut else { continue };

        let segment = &window[..cut];
        // The cursor advances whether or not the segment held speech.
        committed_src += cut;
        if peak(segment) < SILENCE_PEAK {
            continue;
        }
        let audio16k = audio::resample_to_16k(segment, rate);
        let Ok(result) = engine.transcribe(&audio16k, options) else {
            continue; // a missed segment is recovered by the final decode tail
        };
        let text = result.text.trim();
        if text.is_empty() {
            continue;
        }
        committed_text = join_text(&committed_text, text);
        emit_partial(app, &committed_text);

        // The committed text is final and monotonic, so a plain append is
        // enough — no backspacing, no stable-prefix churn.
        if live {
            if let Some(t) = typer.as_mut() {
                if let Err(e) = t.extend_to(&committed_text) {
                    eprintln!("[greffe] {e}");
                }
            }
        }
    }

    (committed_text, committed_src)
}

/// Snapshot of samples starting at `from` (earlier ones are already committed).
fn snapshot_from(state: &AppState, from: usize) -> (Vec<f32>, u32) {
    let buf = state.samples.lock().unwrap_or_else(|e| e.into_inner());
    let chunk = buf.get(from..).unwrap_or(&[]).to_vec();
    let rate = state.src_rate.load(Ordering::Relaxed).max(8000);
    (chunk, rate)
}

fn peak(samples: &[f32]) -> f32 {
    samples.iter().fold(0.0f32, |m, s| m.max(s.abs()))
}

fn rms(slice: &[f32]) -> f32 {
    (slice.iter().map(|s| s * s).sum::<f32>() / slice.len().max(1) as f32).sqrt()
}

/// Scans from newest to oldest for a `GAP_SECS` trough in the window (excluding
/// the last `keep_tail` samples). The threshold adapts to the window level:
/// with an AGC mic the background noise can exceed any fixed absolute threshold.
/// Returns the cut position.
fn find_silence_cut(window: &[f32], rate: u32, keep_tail: usize) -> Option<usize> {
    let gap = (GAP_SECS * rate as f32) as usize;
    let scan_end = window.len().saturating_sub(keep_tail);
    // A cut too close to the start would not shorten anything useful.
    let min_cut = gap * 2;
    if scan_end < min_cut + gap {
        return None;
    }
    let threshold = GAP_RMS.max(rms(window) * 0.25);
    let mut pos = scan_end - gap;
    while pos >= min_cut {
        if rms(&window[pos..pos + gap]) < threshold {
            return Some(pos + gap / 2);
        }
        pos -= gap;
    }
    None
}

/// When no real pause exists (continuous speech), returns the quietest trough
/// in the window — cutting at the least likely word boundary is better than
/// letting the window freeze live mode.
fn find_quietest_cut(window: &[f32], rate: u32, keep_tail: usize) -> Option<usize> {
    let gap = (GAP_SECS * rate as f32) as usize;
    let scan_end = window.len().saturating_sub(keep_tail);
    let min_cut = gap * 2;
    if scan_end < min_cut + gap {
        return None;
    }
    let mut best_rms = f32::MAX;
    let mut best_pos = 0usize;
    let mut pos = min_cut;
    while pos + gap <= scan_end {
        let r = rms(&window[pos..pos + gap]);
        if r < best_rms {
            best_rms = r;
            best_pos = pos;
        }
        pos += gap;
    }
    (best_pos > 0).then(|| best_pos + gap / 2)
}

/// Concatenates two transcription fragments with a single space.
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
    eprintln!("[greffe] error: {message}");
    show_overlay(app);
    emit_phase(app, "error", Some(message));
    let app = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(2800));
        emit_phase(&app, "idle", None);
        hide_overlay(&app);
    });
}

/// Positions the overlay at the bottom-center of the screen containing the cursor
/// (falls back to the primary monitor), then shows it without giving it focus.
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
        // GTK only creates the native GdkWindow once the window is shown.
        // Calling set_ignore_cursor_events() earlier panics inside tao/wry.
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
