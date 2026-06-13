use crate::models::{self, EngineKind};
use tauri::AppHandle;
use transcribe_rs::SpeechModel;

/// STT engine loaded in memory. Loading is expensive (several seconds),
/// so the model stays resident between dictation sessions.
pub struct EngineSlot {
    pub model_id: Option<String>,
    pub engine: Option<Box<dyn SpeechModel + Send>>,
}

impl EngineSlot {
    pub fn empty() -> Self {
        Self { model_id: None, engine: None }
    }

    pub fn clear(&mut self) {
        self.model_id = None;
        self.engine = None;
    }

    /// Loads the requested model if it is not already resident.
    /// Returns true if a (slow) load actually happened.
    pub fn ensure_loaded(&mut self, app: &AppHandle, model_id: &str) -> Result<bool, String> {
        if self.model_id.as_deref() == Some(model_id) && self.engine.is_some() {
            return Ok(false);
        }
        self.clear();

        let spec = models::spec(model_id).ok_or_else(|| format!("Modèle inconnu : {model_id}"))?;
        let path = models::model_path(app, spec)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("Le modèle « {} » n'est pas téléchargé.", spec.name))?;

        let engine: Box<dyn SpeechModel + Send> = match spec.engine {
            EngineKind::Whisper => {
                let engine = transcribe_rs::whisper_cpp::WhisperEngine::load(&path)
                    .map_err(|e| format!("Échec du chargement Whisper : {e}"))?;
                Box::new(engine)
            }
            EngineKind::Parakeet => {
                let engine = transcribe_rs::onnx::parakeet::ParakeetModel::load(
                    &path,
                    &transcribe_rs::onnx::Quantization::Int8,
                )
                .map_err(|e| format!("Échec du chargement Parakeet : {e}"))?;
                Box::new(engine)
            }
        };

        self.model_id = Some(model_id.to_string());
        self.engine = Some(engine);
        Ok(true)
    }
}
