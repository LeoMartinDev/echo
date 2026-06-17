use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Manager};

/// Maximum number of history entries to keep.
const MAX_ENTRIES: usize = 300;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub ts_ms: u64,
    pub text: String,
    pub model_id: String,
    pub duration_secs: f32,
}

fn history_path(app: &AppHandle) -> anyhow::Result<PathBuf> {
    let dir = app.path().app_data_dir()?;
    fs::create_dir_all(&dir)?;
    Ok(dir.join("history.jsonl"))
}

fn load(app: &AppHandle) -> Vec<HistoryEntry> {
    let Ok(path) = history_path(app) else {
        return Vec::new();
    };
    let Ok(raw) = fs::read_to_string(path) else {
        return Vec::new();
    };
    raw.lines()
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect()
}

fn save(app: &AppHandle, entries: &[HistoryEntry]) -> anyhow::Result<()> {
    let path = history_path(app)?;
    let mut out = String::new();
    for e in entries {
        out.push_str(&serde_json::to_string(e)?);
        out.push('\n');
    }
    fs::write(path, out)?;
    Ok(())
}

pub fn append(app: &AppHandle, text: &str, model_id: &str, duration_secs: f32) {
    let entry = HistoryEntry {
        ts_ms: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0),
        text: text.to_string(),
        model_id: model_id.to_string(),
        duration_secs,
    };
    let mut entries = load(app);
    entries.push(entry);
    if entries.len() > MAX_ENTRIES {
        entries.drain(..entries.len() - MAX_ENTRIES);
    }
    if let Err(e) = save(app, &entries) {
        eprintln!("[echo] history not saved: {e}");
    }
}

/// Entries from newest to oldest.
pub fn list(app: &AppHandle) -> Vec<HistoryEntry> {
    let mut entries = load(app);
    entries.reverse();
    entries
}

pub fn delete(app: &AppHandle, ts_ms: u64) -> Result<(), String> {
    let mut entries = load(app);
    entries.retain(|e| e.ts_ms != ts_ms);
    save(app, &entries).map_err(|e| e.to_string())
}

pub fn clear(app: &AppHandle) -> Result<(), String> {
    let path = history_path(app).map_err(|e| e.to_string())?;
    if path.exists() {
        fs::remove_file(path).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[cfg(test)]
mod history_tests {
    use super::*;

    #[test]
    fn entry_json_roundtrip() {
        let entry = HistoryEntry {
            ts_ms: 1718198400123,
            text: "Hello world".to_string(),
            model_id: "parakeet-tdt-0.6b-v3".to_string(),
            duration_secs: 3.5,
        };
        let json = serde_json::to_string(&entry).unwrap();
        let rt: HistoryEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(rt.ts_ms, entry.ts_ms);
        assert_eq!(rt.text, entry.text);
        assert_eq!(rt.model_id, entry.model_id);
        assert!((rt.duration_secs - entry.duration_secs).abs() < f32::EPSILON);
    }

    #[test]
    fn entry_deser_minimal() {
        let json = r#"{"ts_ms":0,"text":"","model_id":"whisper-small","duration_secs":0.0}"#;
        let entry: HistoryEntry = serde_json::from_str(json).unwrap();
        assert_eq!(entry.ts_ms, 0);
        assert_eq!(entry.text, "");
        assert_eq!(entry.model_id, "whisper-small");
        assert!(entry.duration_secs < f32::EPSILON);
    }

    #[test]
    fn entry_serde_keys_present() {
        let entry = HistoryEntry {
            ts_ms: 1,
            text: "x".into(),
            model_id: "m".into(),
            duration_secs: 2.0,
        };
        let json = serde_json::to_string(&entry).unwrap();
        assert!(json.contains("ts_ms"));
        assert!(json.contains("text"));
        assert!(json.contains("model_id"));
        assert!(json.contains("duration_secs"));
    }
}
