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
