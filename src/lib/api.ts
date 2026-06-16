import { invoke } from "@tauri-apps/api/core";
import type { TKey } from "./i18n.svelte";

export type InsertionMode = "live" | "on_release";

export interface AppSettings {
  model_id: string;
  language: string | null;
  hotkey: string;
  insertion_mode: InsertionMode;
  history_enabled: boolean;
  autostart: boolean;
  /** Interface language: "fr", "en", or null to follow the system locale. */
  ui_language: string | null;
}

export interface HistoryEntry {
  ts_ms: number;
  text: string;
  model_id: string;
  duration_secs: number;
}

export interface ModelInfo {
  id: string;
  name: string;
  description: string;
  engine: "whisper" | "parakeet";
  size_mb: number;
  downloaded: boolean;
  downloading: boolean;
  active: boolean;
}

export interface DownloadEvent {
  id: string;
  received: number;
  total: number | null;
  status: "downloading" | "extracting" | "done" | "error";
  error: string | null;
}

export interface PhaseEvent {
  phase: "recording" | "loading_model" | "transcribing" | "idle" | "error";
  message: string | null;
}

export const getSettings = () => invoke<AppSettings>("get_settings");
export const setSettings = (settings: AppSettings) =>
  invoke<void>("set_settings", { newSettings: settings });
export const listModels = () => invoke<ModelInfo[]>("list_models");
export const downloadModel = (id: string) => invoke<void>("download_model", { id });
export const deleteModel = (id: string) => invoke<void>("delete_model", { id });
export const accessibilityStatus = (prompt: boolean) =>
  invoke<boolean>("accessibility_status", { prompt });
// Suspends the global hotkey while the UI captures a new one (so pressing the
// current shortcut doesn't start a dictation); re-arms it when capture ends.
export const setHotkeyCapture = (capturing: boolean) =>
  invoke<void>("set_hotkey_capture", { capturing });
export const getHistory = () => invoke<HistoryEntry[]>("get_history");
export const clearHistory = () => invoke<void>("clear_history");
export const deleteHistoryEntry = (tsMs: number) => invoke<void>("delete_history_entry", { tsMs });
export const getProcessMemory = () => invoke<number>("get_process_memory");
export const installUpdate = () => invoke<void>("install_update");

// Labels are resolved at render time via i18n (using `key`), to follow the
// current interface language.
export const LANGUAGES: Array<{ code: string | null; key: TKey }> = [
  { code: null, key: "lang_auto" },
  { code: "fr", key: "lang_fr" },
  { code: "en", key: "lang_en" },
  { code: "de", key: "lang_de" },
  { code: "es", key: "lang_es" },
  { code: "it", key: "lang_it" },
  { code: "pt", key: "lang_pt" },
  { code: "nl", key: "lang_nl" },
  { code: "pl", key: "lang_pl" },
  { code: "uk", key: "lang_uk" },
  { code: "ru", key: "lang_ru" },
  { code: "ja", key: "lang_ja" },
  { code: "zh", key: "lang_zh" },
  { code: "ko", key: "lang_ko" },
  { code: "ar", key: "lang_ar" },
];
