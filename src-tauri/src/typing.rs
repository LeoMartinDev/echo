use enigo::{Direction, Enigo, Key, Keyboard, Settings as EnigoSettings};

/// Types text into the focused application following successive decodes.
///
/// During dictation (partials), only extensions are written: if a new decode
/// contradicts what was already typed, we wait for the final decode.
/// At the end, we reconcile with backspaces as needed.
pub struct Typer {
    enigo: Enigo,
    typed: Vec<char>,
}

impl Typer {
    pub fn new() -> Result<Self, String> {
        let enigo = Enigo::new(&EnigoSettings::default())
            .map_err(|e| format!("Saisie clavier indisponible : {e}"))?;
        Ok(Self { enigo, typed: Vec::new() })
    }

    /// Only writes `text` if it extends what has already been typed.
    pub fn extend_to(&mut self, text: &str) -> Result<(), String> {
        let target: Vec<char> = text.chars().collect();
        if target.len() <= self.typed.len() {
            return Ok(());
        }
        if !target.starts_with(&self.typed) {
            return Ok(()); // divergence: let the final decode fix it
        }
        let suffix: String = target[self.typed.len()..].iter().collect();
        self.enigo
            .text(&suffix)
            .map_err(|e| format!("Échec de la saisie : {e}"))?;
        self.typed = target;
        Ok(())
    }

    /// Aligns the typed text exactly to `text` (backspaces as needed).
    pub fn reconcile_to(&mut self, text: &str) -> Result<(), String> {
        let target: Vec<char> = text.chars().collect();
        let common = self
            .typed
            .iter()
            .zip(target.iter())
            .take_while(|(a, b)| a == b)
            .count();
        let to_delete = self.typed.len() - common;
        for _ in 0..to_delete {
            self.enigo
                .key(Key::Backspace, Direction::Click)
                .map_err(|e| format!("Échec de la saisie : {e}"))?;
        }
        if common < target.len() {
            let suffix: String = target[common..].iter().collect();
            self.enigo
                .text(&suffix)
                .map_err(|e| format!("Échec de la saisie : {e}"))?;
        }
        self.typed = target;
        Ok(())
    }
}
