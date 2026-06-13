use serde::Serialize;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Emitter, Manager};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EngineKind {
    Whisper,
    Parakeet,
}

pub struct ModelSpec {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub engine: EngineKind,
    pub url: &'static str,
    /// true if the URL points to a .tar.gz archive to extract.
    pub archive: bool,
    pub size_mb: u32,
}

// Three models, three roles: fast default (live) / lightweight responsive
// Whisper / accurate Whisper. Whisper decodes in 30 s passes: Small stays fast
// enough for live mode, Medium targets accuracy outside live mode.
pub const CATALOG: &[ModelSpec] = &[
    ModelSpec {
        id: "parakeet-tdt-0.6b-v3",
        name: "Parakeet TDT 0.6B v3",
        description: "Recommandé — ultra rapide en CPU (~20-30x temps réel), excellent en dictée, 25 langues européennes.",
        engine: EngineKind::Parakeet,
        url: "https://blob.handy.computer/parakeet-v3-int8.tar.gz",
        archive: true,
        size_mb: 670,
    },
    ModelSpec {
        id: "whisper-small",
        name: "Whisper Small",
        description: "Léger et réactif — assez rapide pour le mode direct (CPU/Metal). Multilingue (~100 langues), précision correcte.",
        engine: EngineKind::Whisper,
        url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small.bin",
        archive: false,
        size_mb: 488,
    },
    ModelSpec {
        id: "whisper-medium-q5",
        name: "Whisper Medium",
        description: "Précision élevée, ~100 langues. Plus lourd que Small — idéal hors mode direct.",
        engine: EngineKind::Whisper,
        url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-medium-q5_0.bin",
        archive: false,
        size_mb: 539,
    },
];

pub fn spec(id: &str) -> Option<&'static ModelSpec> {
    CATALOG.iter().find(|m| m.id == id)
}

pub fn models_dir(app: &AppHandle) -> anyhow::Result<PathBuf> {
    let dir = app.path().app_data_dir()?.join("models");
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

pub fn model_dir(app: &AppHandle, id: &str) -> anyhow::Result<PathBuf> {
    Ok(models_dir(app)?.join(id))
}

/// For Whisper: path to the GGML file. For Parakeet: directory containing vocab.txt.
pub fn model_path(app: &AppHandle, spec: &ModelSpec) -> anyhow::Result<Option<PathBuf>> {
    let dir = model_dir(app, spec.id)?;
    match spec.engine {
        EngineKind::Whisper => {
            let file = dir.join("model.bin");
            Ok(file.exists().then_some(file))
        }
        EngineKind::Parakeet => Ok(find_parakeet_root(&dir, 0)),
    }
}

fn find_parakeet_root(dir: &Path, depth: u8) -> Option<PathBuf> {
    if depth > 3 || !dir.is_dir() {
        return None;
    }
    if dir.join("vocab.txt").exists() {
        return Some(dir.to_path_buf());
    }
    for entry in fs::read_dir(dir).ok()?.flatten() {
        if let Some(found) = find_parakeet_root(&entry.path(), depth + 1) {
            return Some(found);
        }
    }
    None
}

pub fn is_downloaded(app: &AppHandle, spec: &ModelSpec) -> bool {
    model_path(app, spec).ok().flatten().is_some()
}

#[derive(Clone, Serialize)]
struct DownloadProgress<'a> {
    id: &'a str,
    received: u64,
    total: Option<u64>,
    status: &'a str, // "downloading" | "extracting" | "done" | "error"
    error: Option<String>,
}

fn emit_progress(app: &AppHandle, p: DownloadProgress) {
    let _ = app.emit("greffe://download", &p);
}

pub async fn download(app: AppHandle, id: String) -> Result<(), String> {
    let spec = spec(&id).ok_or_else(|| format!("Modèle inconnu : {id}"))?;
    let dir = model_dir(&app, spec.id).map_err(|e| e.to_string())?;
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;

    let result = download_inner(&app, spec, &dir).await;
    match &result {
        Ok(()) => emit_progress(
            &app,
            DownloadProgress { id: spec.id, received: 0, total: None, status: "done", error: None },
        ),
        Err(e) => {
            let _ = fs::remove_dir_all(&dir);
            emit_progress(
                &app,
                DownloadProgress {
                    id: spec.id,
                    received: 0,
                    total: None,
                    status: "error",
                    error: Some(e.clone()),
                },
            );
        }
    }
    result
}

async fn download_inner(app: &AppHandle, spec: &ModelSpec, dir: &Path) -> Result<(), String> {
    use futures_util::StreamExt;

    let target = if spec.archive { dir.join("download.tar.gz") } else { dir.join("model.bin") };
    let tmp = dir.join("download.part");

    let client = reqwest::Client::new();
    let resp = client
        .get(spec.url)
        .send()
        .await
        .map_err(|e| format!("Téléchargement impossible : {e}"))?
        .error_for_status()
        .map_err(|e| format!("Téléchargement refusé : {e}"))?;

    let total = resp.content_length();
    let mut file = fs::File::create(&tmp).map_err(|e| e.to_string())?;
    let mut stream = resp.bytes_stream();
    let mut received: u64 = 0;
    let mut last_emit: u64 = 0;

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| format!("Téléchargement interrompu : {e}"))?;
        file.write_all(&chunk).map_err(|e| e.to_string())?;
        received += chunk.len() as u64;
        // Throttle: one event per MB.
        if received - last_emit > 1024 * 1024 {
            last_emit = received;
            emit_progress(
                app,
                DownloadProgress {
                    id: spec.id,
                    received,
                    total,
                    status: "downloading",
                    error: None,
                },
            );
        }
    }
    drop(file);
    fs::rename(&tmp, &target).map_err(|e| e.to_string())?;

    if spec.archive {
        emit_progress(
            app,
            DownloadProgress { id: spec.id, received, total, status: "extracting", error: None },
        );
        let archive_path = target.clone();
        let extract_dir = dir.to_path_buf();
        tauri::async_runtime::spawn_blocking(move || -> Result<(), String> {
            let file = fs::File::open(&archive_path).map_err(|e| e.to_string())?;
            let gz = flate2::read::GzDecoder::new(file);
            let mut archive = tar::Archive::new(gz);
            archive.unpack(&extract_dir).map_err(|e| format!("Extraction impossible : {e}"))?;
            fs::remove_file(&archive_path).map_err(|e| e.to_string())?;
            Ok(())
        })
        .await
        .map_err(|e| e.to_string())??;
    }

    Ok(())
}

pub fn delete(app: &AppHandle, id: &str) -> Result<(), String> {
    let dir = model_dir(app, id).map_err(|e| e.to_string())?;
    if dir.exists() {
        fs::remove_dir_all(&dir).map_err(|e| e.to_string())?;
    }
    Ok(())
}
