/**
 * Mock minimal de l'IPC Tauri pour prévisualiser l'UI dans un navigateur
 * (vite dev hors `tauri dev`). Inactif dès que l'app tourne dans Tauri.
 */
export function installDevMockIfNeeded() {
  if (!import.meta.env.DEV || "__TAURI_INTERNALS__" in window) return;

  let settings = {
    model_id: "parakeet-tdt-0.6b-v3",
    language: null as string | null,
    hotkey: "Ctrl+Alt+Space",
    insertion_mode: "on_release",
    history_enabled: true,
    autostart: false,
    ui_language: null as string | null,
  };

  let history = [
    {
      ts_ms: Date.now() - 5 * 60_000,
      text: "On se retrouve demain à 14 h pour faire le point sur la maquette.",
      model_id: "parakeet-tdt-0.6b-v3",
      duration_secs: 4.2,
    },
    {
      ts_ms: Date.now() - 26 * 3_600_000,
      text: "Pense à renvoyer le contrat signé avant vendredi.",
      model_id: "parakeet-tdt-0.6b-v3",
      duration_secs: 3.1,
    },
  ];

  const models = [
    {
      id: "parakeet-tdt-0.6b-v3",
      name: "Parakeet TDT 0.6B v3",
      description:
        "Recommandé — ultra rapide en CPU (~20-30x temps réel), excellent en dictée, 25 langues européennes.",
      engine: "parakeet",
      size_mb: 670,
      downloaded: true,
      downloading: false,
    },
    {
      id: "whisper-small",
      name: "Whisper Small",
      description:
        "Léger et réactif — assez rapide pour le mode direct (CPU/Metal). Multilingue (~100 langues), précision correcte.",
      engine: "whisper",
      size_mb: 488,
      downloaded: false,
      downloading: false,
    },
    {
      id: "whisper-medium-q5",
      name: "Whisper Medium",
      description: "Précision élevée, ~100 langues. Plus lourd que Small — idéal hors mode direct.",
      engine: "whisper",
      size_mb: 539,
      downloaded: false,
      downloading: false,
    },
  ];

  let nextCallbackId = 1;

  (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__ = {
    transformCallback(cb: (payload: unknown) => void) {
      const id = nextCallbackId++;
      (window as unknown as Record<string, unknown>)[`_${id}`] = cb;
      return id;
    },
    // deno-lint-ignore require-await
    async invoke(cmd: string, args: Record<string, unknown> = {}) {
      switch (cmd) {
        case "get_settings":
          return settings;
        case "set_settings":
          settings = args.newSettings as typeof settings;
          return;
        case "list_models":
          return models.map((m) => ({ ...m, active: m.id === settings.model_id }));
        case "download_model":
        case "delete_model":
          return;
        case "get_history":
          return history;
        case "get_process_memory":
          return 1_180_000_000;
        case "clear_history":
          history = [];
          return;
        case "delete_history_entry":
          history = history.filter((e) => e.ts_ms !== args.tsMs);
          return;
        case "plugin:clipboard-manager|write_text":
          return;
        case "accessibility_status":
          return false; // affiche le bandeau pour vérifier son design
        case "set_hotkey_capture":
          return;
        case "install_update":
          return;
        case "plugin:event|listen":
        case "plugin:event|unlisten":
          return 0;
        default:
          console.warn(`[devMock] commande non mockée : ${cmd}`);
          return;
      }
    },
  };
}
