use crate::engine::EngineSlot;
use crate::settings::AppSettings;
use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, AtomicU32};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};

pub struct AppState {
    pub settings: Mutex<AppSettings>,
    pub engine: Arc<Mutex<EngineSlot>>,
    /// true between shortcut press and release.
    pub recording: Arc<AtomicBool>,
    /// Incremented on each session start. A worker captures the value at spawn
    /// and stops touching the overlay once a newer session supersedes it,
    /// preventing a lagging final decode from clobbering the next session.
    pub generation: Arc<AtomicU32>,
    /// Mono samples at the native mic rate, accumulated during dictation.
    pub samples: Arc<Mutex<Vec<f32>>>,
    /// Native sample rate of the mic (published by the capture thread).
    pub src_rate: Arc<AtomicU32>,
    /// Channel to stop the current capture thread.
    pub capture_stop: Mutex<Option<Sender<()>>>,
    /// Model downloads in progress (ids).
    pub downloads: Mutex<HashSet<String>>,
}

impl AppState {
    pub fn new(settings: AppSettings) -> Self {
        Self {
            settings: Mutex::new(settings),
            engine: Arc::new(Mutex::new(EngineSlot::empty())),
            recording: Arc::new(AtomicBool::new(false)),
            generation: Arc::new(AtomicU32::new(0)),
            samples: Arc::new(Mutex::new(Vec::new())),
            src_rate: Arc::new(AtomicU32::new(48_000)),
            capture_stop: Mutex::new(None),
            downloads: Mutex::new(HashSet::new()),
        }
    }
}

#[cfg(test)]
mod state_tests {
    use super::*;
    use std::sync::atomic::Ordering;

    #[test]
    fn new_state_defaults() {
        let s = AppState::new(AppSettings::default());
        assert!(!s.recording.load(Ordering::SeqCst));
        assert_eq!(s.generation.load(Ordering::SeqCst), 0);
        assert_eq!(s.src_rate.load(Ordering::Relaxed), 48_000);
        assert!(s.samples.lock().unwrap().is_empty());
        assert!(s.capture_stop.lock().unwrap().is_none());
        assert!(s.downloads.lock().unwrap().is_empty());
    }

    #[test]
    fn new_state_preserves_settings() {
        let mut settings = AppSettings::default();
        settings.model_id = "whisper-small".to_string();
        let s = AppState::new(settings);
        assert_eq!(s.settings.lock().unwrap().model_id, "whisper-small");
    }

    #[test]
    fn engine_slot_starts_empty() {
        let s = AppState::new(AppSettings::default());
        let slot = s.engine.lock().unwrap();
        assert!(slot.model_id.is_none());
        assert!(slot.engine.is_none());
    }
}
