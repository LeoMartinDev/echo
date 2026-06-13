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

    pub fn typed_chars(&self) -> usize {
        self.typed.len()
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

/// Common prefix (in whole words) between two successive decodes: the "stable"
/// part that can be typed without risking contradiction later.
pub fn stable_prefix(prev: &str, current: &str) -> String {
    let prev_words: Vec<&str> = prev.split_whitespace().collect();
    let cur_words: Vec<&str> = current.split_whitespace().collect();
    let mut stable: Vec<&str> = Vec::new();
    for (p, c) in prev_words.iter().zip(cur_words.iter()) {
        if p == c {
            stable.push(c);
        } else {
            break;
        }
    }
    stable.join(" ")
}
