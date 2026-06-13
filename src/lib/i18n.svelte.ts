/**
 * Minimal reactive i18n (Svelte 5). Two languages: French and English.
 * The default locale follows the system language (navigator.language); falls
 * back to English when a translation is missing. The explicit preference lives
 * in settings (AppSettings.ui_language) — see App.svelte.
 */

const en = {
  // — header —
  no_model: "no model",
  lede_pre: "Hold",
  lede_post:
    "and speak — the text is typed into the active application. Everything stays on your machine.",

  // — banners —
  a11y_before: "The",
  a11y_perm: "Accessibility",
  a11y_after: "permission is required to type into other applications.",
  authorize: "authorize",
  close: "Close",

  // — tabs —
  tab_settings: "settings",
  tab_history: "history",
  loading: "loading…",

  // — shortcut section —
  sec_shortcut: "shortcut",
  ptt_label: "Push-to-talk",
  ptt_hint: "hold to dictate, release to finish",
  ptt_change: "click to change",
  press_key: "press a key",

  // — insertion section —
  sec_insertion: "insertion",
  write_mode: "Writing mode",
  write_mode_hint: "“on release” is safer when the shortcut uses Cmd/Ctrl",
  mode_live: "live",
  mode_on_release: "on release",
  lang_label: "Language",
  lang_hint: "hint given to the model",

  // — system section —
  sec_system: "system",
  autostart_label: "Launch at startup",
  autostart_hint: "Echo starts with your session, in the tray",
  ui_lang_label: "Interface language",
  ui_lang_hint: "language of this window and the menu",
  ui_lang_auto: "system",
  on: "on",
  off: "off",

  // — models section —
  sec_models: "models",
  model_active_title: "Active model",
  use_model: "use",
  delete_short: "del.",
  active: "active",
  download: "download",
  extracting: "extracting…",
  downloading: "downloading…",
  unit_gb: "GB",
  unit_mb: "MB",

  // — history —
  keep_label: "Keep transcriptions",
  keep_hint: "stored only on this machine",
  filter_placeholder: "filter…",
  clear_all: "clear",
  empty_none: "no transcription",
  empty_no_results: "no result",
  copy: "copy",
  copied: "copied",
  delete_entry: "Delete this transcription",
  today: "today",
  yesterday: "yesterday",

  // — overlay —
  overlay_no_field:
    "No text field detected — the text will be copied to the clipboard",

  // — transcription languages (LANGUAGES) —
  lang_auto: "Automatic detection",
  lang_fr: "French",
  lang_en: "English",
  lang_de: "German",
  lang_es: "Spanish",
  lang_it: "Italian",
  lang_pt: "Portuguese",
  lang_nl: "Dutch",
  lang_pl: "Polish",
  lang_uk: "Ukrainian",
  lang_ru: "Russian",
  lang_ja: "Japanese",
  lang_zh: "Chinese",
  lang_ko: "Korean",
  lang_ar: "Arabic",

  // — model descriptions (key by id, fallback to backend value) —
  "model_desc.parakeet-tdt-0.6b-v3":
    "Recommended — ultra fast on CPU (~20-30× real time), excellent for dictation, 25 European languages.",
  "model_desc.whisper-small":
    "Lightweight and responsive — fast enough for live mode (CPU/Metal). Multilingual (~100 languages), decent accuracy.",
  "model_desc.whisper-medium-q5":
    "High accuracy, ~100 languages. Heavier than Small — best outside live mode.",
} as const;

type Dict = Record<keyof typeof en, string>;

const fr: Dict = {
  no_model: "aucun modèle",
  lede_pre: "Maintenez",
  lede_post:
    "et parlez — le texte s'écrit dans l'application active. Tout reste sur votre machine.",

  a11y_before: "L'autorisation",
  a11y_perm: "Accessibilité",
  a11y_after: "est requise pour écrire dans les autres applications.",
  authorize: "autoriser",
  close: "Fermer",

  tab_settings: "réglages",
  tab_history: "historique",
  loading: "chargement…",

  sec_shortcut: "raccourci",
  ptt_label: "Push-to-talk",
  ptt_hint: "maintenir pour dicter, relâcher pour terminer",
  ptt_change: "cliquer pour changer",
  press_key: "appuyez sur une touche",

  sec_insertion: "insertion",
  write_mode: "Mode d'écriture",
  write_mode_hint: "« à la fin » est plus sûr si le raccourci contient Cmd/Ctrl",
  mode_live: "en direct",
  mode_on_release: "à la fin",
  lang_label: "Langue",
  lang_hint: "indice donné au modèle",

  sec_system: "système",
  autostart_label: "Lancer au démarrage",
  autostart_hint: "Echo démarre avec votre session, dans le tray",
  ui_lang_label: "Langue de l'interface",
  ui_lang_hint: "langue de cette fenêtre et du menu",
  ui_lang_auto: "système",
  on: "activé",
  off: "désactivé",

  sec_models: "modèles",
  model_active_title: "Modèle actif",
  use_model: "utiliser",
  delete_short: "suppr.",
  active: "actif",
  download: "télécharger",
  extracting: "extraction…",
  downloading: "téléchargement…",
  unit_gb: "Go",
  unit_mb: "Mo",

  keep_label: "Conserver les transcriptions",
  keep_hint: "stockées uniquement sur cette machine",
  filter_placeholder: "filtrer…",
  clear_all: "vider",
  empty_none: "aucune transcription",
  empty_no_results: "aucun résultat",
  copy: "copier",
  copied: "copié",
  delete_entry: "Supprimer cette transcription",
  today: "aujourd'hui",
  yesterday: "hier",

  overlay_no_field:
    "Aucun champ texte détecté — le texte sera copié dans le presse-papiers",

  lang_auto: "Détection automatique",
  lang_fr: "Français",
  lang_en: "Anglais",
  lang_de: "Allemand",
  lang_es: "Espagnol",
  lang_it: "Italien",
  lang_pt: "Portugais",
  lang_nl: "Néerlandais",
  lang_pl: "Polonais",
  lang_uk: "Ukrainien",
  lang_ru: "Russe",
  lang_ja: "Japonais",
  lang_zh: "Chinois",
  lang_ko: "Coréen",
  lang_ar: "Arabe",

  "model_desc.parakeet-tdt-0.6b-v3":
    "Recommandé — ultra rapide en CPU (~20-30x temps réel), excellent en dictée, 25 langues européennes.",
  "model_desc.whisper-small":
    "Léger et réactif — assez rapide pour le mode direct (CPU/Metal). Multilingue (~100 langues), précision correcte.",
  "model_desc.whisper-medium-q5":
    "Précision élevée, ~100 langues. Plus lourd que Small — idéal hors mode direct.",
};

const DICTS = { en, fr };
export type Locale = keyof typeof DICTS;
export const SUPPORTED: Locale[] = ["en", "fr"];
export type TKey = keyof typeof en;

function isSupported(code: string): code is Locale {
  return (SUPPORTED as string[]).includes(code);
}

/** System language, mapped to a supported locale (English fallback). */
export function systemLocale(): Locale {
  const code = (navigator.language || "en").slice(0, 2).toLowerCase();
  return isSupported(code) ? code : "en";
}

/**
 * Resolves the stored preference (AppSettings.ui_language): an explicit supported
 * value takes priority, otherwise falls back to the system locale.
 */
export function resolveLocale(pref: string | null | undefined): Locale {
  return pref && isSupported(pref) ? pref : systemLocale();
}

let current = $state<Locale>(systemLocale());

export function getLocale(): Locale {
  return current;
}

export function setLocale(loc: Locale) {
  current = loc;
}

/** BCP-47 tag for Intl APIs (date/time formatting). */
export function localeTag(): string {
  return current === "fr" ? "fr-FR" : "en-US";
}

/**
 * Translates `key` in the current locale. Falls back to English then to the raw
 * key if a translation is missing. `params` interpolates `{name}` tokens.
 */
export function t(key: TKey, params?: Record<string, string | number>): string {
  let s: string = DICTS[current][key] ?? en[key] ?? key;
  if (params) {
    for (const [k, v] of Object.entries(params)) {
      s = s.replaceAll(`{${k}}`, String(v));
    }
  }
  return s;
}
