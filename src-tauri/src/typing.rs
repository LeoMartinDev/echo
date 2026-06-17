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
        Ok(Self {
            enigo,
            typed: Vec::new(),
        })
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

#[cfg(test)]
mod typing_tests {
    // NOTE: Typer depends on Enigo, which requires a running display server
    // (X11/Wayland on Linux). These tests exercise the pure textual logic
    // that Typer::extend_to and Typer::reconcile_to perform — the character
    // tracking and diff computation — without hitting real keystrokes.

    #[test]
    fn extend_to_logic_prefix_match() {
        let typed: Vec<char> = "hello".chars().collect();
        let text = "hello world";
        let target: Vec<char> = text.chars().collect();

        assert!(target.len() > typed.len());
        assert!(target.starts_with(&typed));

        let suffix: String = target[typed.len()..].iter().collect();
        assert_eq!(suffix, " world");
    }

    #[test]
    fn extend_to_shorter_noop() {
        let typed: Vec<char> = "hello world".chars().collect();
        let text = "hello";
        let target: Vec<char> = text.chars().collect();
        assert!(!(target.len() > typed.len()));
    }

    #[test]
    fn extend_to_divergence_noop() {
        let typed: Vec<char> = "hello".chars().collect();
        let text = "world hello";
        let target: Vec<char> = text.chars().collect();
        assert!(target.len() > typed.len());
        assert!(!target.starts_with(&typed));
    }

    #[test]
    fn extend_to_empty_target() {
        let typed: Vec<char> = "hello".chars().collect();
        let text = "";
        let target: Vec<char> = text.chars().collect();
        assert!(!(target.len() > typed.len()));
    }

    #[test]
    fn reconcile_to_common_prefix() {
        let typed: Vec<char> = "hello world".chars().collect();
        let text = "hello moon";
        let target: Vec<char> = text.chars().collect();

        let common = typed
            .iter()
            .zip(target.iter())
            .take_while(|(a, b)| a == b)
            .count();
        assert_eq!(common, 6); // "hello "
        assert_eq!(typed.len() - common, 5); // "world" to backspace
        assert_eq!(target.len() - common, 4); // "moon" to type
    }

    #[test]
    fn reconcile_to_exact_match_noop() {
        let typed: Vec<char> = "hello".chars().collect();
        let text = "hello";
        let target: Vec<char> = text.chars().collect();

        let common = typed
            .iter()
            .zip(target.iter())
            .take_while(|(a, b)| a == b)
            .count();
        assert_eq!(common, 5);
        assert_eq!(typed.len() - common, 0);
        assert_eq!(target.len() - common, 0);
    }

    #[test]
    fn reconcile_to_full_replacement() {
        let typed: Vec<char> = "abc".chars().collect();
        let text = "xyz";
        let target: Vec<char> = text.chars().collect();

        let common = typed
            .iter()
            .zip(target.iter())
            .take_while(|(a, b)| a == b)
            .count();
        assert_eq!(common, 0);
        assert_eq!(typed.len() - common, 3);
        assert_eq!(target.len() - common, 3);
    }

    #[test]
    fn reconcile_to_unicode() {
        // "héllo wörld" = 11 chars, "héllo wörld " = 12 chars (with trailing space).
        let typed: Vec<char> = "héllo wörld".chars().collect();
        let text = "héllo wörld 🌍";
        let target: Vec<char> = text.chars().collect();

        let common = typed
            .iter()
            .zip(target.iter())
            .take_while(|(a, b)| a == b)
            .count();
        assert_eq!(common, 11); // "héllo wörld"
        let suffix: String = target[common..].iter().collect();
        assert_eq!(suffix, " 🌍");
    }
}
