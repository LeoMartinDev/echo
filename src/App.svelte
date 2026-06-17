<script lang="ts">
  import { onMount } from "svelte";
  import { listen } from "@tauri-apps/api/event";
  import { writeText } from "@tauri-apps/plugin-clipboard-manager";
  import {
    accessibilityStatus,
    clearHistory,
    deleteHistoryEntry,
    deleteModel,
    downloadModel,
    getHistory,
    getProcessMemory,
    getSettings,
    installUpdate,
    listModels,
    setHotkeyCapture,
    setSettings,
    type AppSettings,
    type DownloadEvent,
    type HistoryEntry,
    type ModelInfo,
  } from "./lib/api";
  import { t, setLocale, resolveLocale } from "./lib/i18n.svelte";
  import { keyFromEvent } from "./lib/utils";

  import Header from "./components/Header.svelte";
  import ErrorBanner from "./components/ErrorBanner.svelte";
  import AccessibilityBanner from "./components/AccessibilityBanner.svelte";
  import UpdateBanner from "./components/UpdateBanner.svelte";
  import Nav from "./components/Nav.svelte";
  import HotkeyCapture from "./components/settings/HotkeyCapture.svelte";
  import InsertionSection from "./components/settings/InsertionSection.svelte";
  import SystemSection from "./components/settings/SystemSection.svelte";
  import ModelList from "./components/settings/ModelList.svelte";
  import HistorySection from "./components/history/HistorySection.svelte";

  let settings = $state<AppSettings | null>(null);
  let models = $state<ModelInfo[]>([]);
  let history = $state<HistoryEntry[]>([]);
  let copiedTs = $state<number | null>(null);
  let view = $state<"settings" | "history">("settings");
  let query = $state("");
  let memBytes = $state<number | null>(null);
  let progress = $state<Record<string, { pct: number | null; status: string }>>(
    {},
  );
  let errorMsg = $state<string | null>(null);
  let capturingHotkey = $state(false);
  let accessibilityOk = $state(true);
  let updateVersion = $state<string | null>(null);
  const isMac = navigator.userAgent.includes("Mac");

  // UI language tracks the setting (or the system locale if unset).
  $effect(() => {
    setLocale(resolveLocale(settings?.ui_language));
  });

  const RETRY_DELAYS = [500, 1000, 1500];

  async function refresh() {
    for (let i = 0; i <= RETRY_DELAYS.length; i++) {
      try {
        [settings, models] = await Promise.all([getSettings(), listModels()]);
        return;
      } catch (e) {
        if (i === RETRY_DELAYS.length) {
          errorMsg = String(e);
          return;
        }
        await new Promise((r) => setTimeout(r, RETRY_DELAYS[i]));
      }
    }
  }

  onMount(() => {
    refresh();
    getHistory().then((h) => (history = h));
    if (isMac) accessibilityStatus(false).then((ok) => (accessibilityOk = ok));

    // Affiche le bandeau de mise à jour en dev
    if (import.meta.env.DEV) {
      setTimeout(() => (updateVersion = "0.2.0-test"), 2000);
    }

    const unsubs: Array<() => void> = [];
    listen("echo://history", async () => {
      history = await getHistory();
    }).then((u) => unsubs.push(u));
    // Process RAM: light poll, only while the window is visible.
    const pollMem = () => getProcessMemory().then((b) => (memBytes = b));
    pollMem();
    let memTimer = setInterval(pollMem, 2500);
    const onVisibility = () => {
      clearInterval(memTimer);
      if (document.visibilityState === "visible") {
        pollMem();
        memTimer = setInterval(pollMem, 2500);
      }
    };
    document.addEventListener("visibilitychange", onVisibility);
    unsubs.push(() => {
      clearInterval(memTimer);
      document.removeEventListener("visibilitychange", onVisibility);
    });

    listen<DownloadEvent>("echo://download", async (e) => {
      const { id, received, total, status, error } = e.payload;
      if (status === "done" || status === "error") {
        delete progress[id];
        progress = { ...progress };
        if (status === "error" && error) errorMsg = error;
        models = await listModels();
      } else {
        progress = {
          ...progress,
          [id]: {
            pct: total ? Math.round((received / total) * 100) : null,
            status,
          },
        };
      }
    }).then((u) => unsubs.push(u));

    listen<string>("echo://update-ready", (e) => {
      updateVersion = e.payload;
    }).then((u) => unsubs.push(u));

    return () => unsubs.forEach((u) => u());
  });

  async function save(patch: Partial<AppSettings>) {
    if (!settings) return;
    const next = { ...settings, ...patch };
    errorMsg = null;
    try {
      await setSettings(next);
      settings = next;
      models = await listModels();
    } catch (e) {
      errorMsg = String(e);
      settings = await getSettings();
    }
  }

  async function startDownload(id: string) {
    errorMsg = null;
    progress = { ...progress, [id]: { pct: 0, status: "downloading" } };
    models = models.map((m) => (m.id === id ? { ...m, downloading: true } : m));
    try {
      await downloadModel(id);
    } catch (e) {
      errorMsg = String(e);
    }
    models = await listModels();
  }

  async function removeModel(id: string) {
    errorMsg = null;
    try {
      await deleteModel(id);
      models = await listModels();
    } catch (e) {
      errorMsg = String(e);
    }
  }

  // --- Keyboard shortcut capture ---

  // Enter capture mode and suspend the global hotkey, so pressing the current
  // shortcut (or any key) is recorded here instead of starting a dictation.
  async function startCapture() {
    capturingHotkey = true;
    try {
      await setHotkeyCapture(true);
    } catch (e) {
      errorMsg = String(e);
      capturingHotkey = false;
    }
  }

  // Leave capture mode and re-arm the global hotkey from the stored settings.
  async function endCapture() {
    if (!capturingHotkey) return;
    capturingHotkey = false;
    try {
      await setHotkeyCapture(false);
    } catch (e) {
      errorMsg = String(e);
    }
  }

  function onHotkeyKeydown(e: KeyboardEvent) {
    if (!capturingHotkey) return;
    e.preventDefault();
    e.stopPropagation();
    if (e.key === "Escape") {
      endCapture();
      return;
    }
    const key = keyFromEvent(e);
    if (!key) return; // modifier alone: wait for the main key
    const parts: string[] = [];
    if (e.ctrlKey) parts.push("Ctrl");
    if (e.metaKey) parts.push(isMac ? "Cmd" : "Super");
    if (e.altKey) parts.push("Alt");
    if (e.shiftKey) parts.push("Shift");
    parts.push(key);
    capturingHotkey = false;
    // Persist the new shortcut, then re-arm the global hotkey. `endCapture`
    // also covers re-selecting the same key, which `set_settings` skips.
    save({ hotkey: parts.join("+") }).finally(() => setHotkeyCapture(false));
  }

  async function copyEntry(entry: HistoryEntry) {
    try {
      await writeText(entry.text);
      copiedTs = entry.ts_ms;
      setTimeout(() => {
        if (copiedTs === entry.ts_ms) copiedTs = null;
      }, 1500);
    } catch (e) {
      errorMsg = String(e);
    }
  }

  async function wipeHistory() {
    try {
      await clearHistory();
      history = [];
    } catch (e) {
      errorMsg = String(e);
    }
  }

  async function deleteEntry(entry: HistoryEntry) {
    try {
      await deleteHistoryEntry(entry.ts_ms);
      history = history.filter((e) => e.ts_ms !== entry.ts_ms);
    } catch (e) {
      errorMsg = String(e);
    }
  }

  const activeModel = $derived(models.find((m) => m.active));
</script>

<svelte:window onkeydown={onHotkeyKeydown} onblur={endCapture} />

<Header {activeModel} {memBytes} />

<main class="max-w-[448px] mx-auto pt-[52px] px-6 pb-14">
  <p
    class="my-[14px] mb-[30px] text-fg-muted max-w-[46ch] animate-[rise_0.45s_cubic-bezier(0.2,0.7,0.2,1)_both]"
    style="animation-delay: calc(var(--d) * 55ms); --d: 1"
  >
    {t("lede_pre")}
    <kbd
      class="font-mono text-[11px] tracking-[0.02em] px-[7px] py-[1.5px] pb-[2.5px] rounded border border-border-strong border-b-2 bg-bg-raised text-fg whitespace-nowrap"
      >{settings?.hotkey ?? "…"}</kbd
    >
    {t("lede_post")}
  </p>

  {#if errorMsg}
    <ErrorBanner message={errorMsg} onclose={() => (errorMsg = null)} />
  {/if}

  {#if isMac && !accessibilityOk}
    <AccessibilityBanner
      onauthorize={async () => {
        await accessibilityStatus(true);
        accessibilityOk = await accessibilityStatus(false);
      }}
    />
  {/if}

  {#if updateVersion}
    <UpdateBanner version={updateVersion} oninstall={() => installUpdate()} />
  {/if}

  <Nav {view} historyCount={history.length} onchange={(v) => (view = v)} />

  {#if view === "settings" && settings}
    <HotkeyCapture
      {settings}
      capturing={capturingHotkey}
      onstartcapture={startCapture}
      onendcapture={endCapture}
    />
    <InsertionSection {settings} onsave={save} />
    <SystemSection {settings} onsave={save} />
    <ModelList
      {models}
      {progress}
      onsave={save}
      onstartdownload={startDownload}
      onremovemodel={removeModel}
    />
  {:else if settings}
    <HistorySection
      {settings}
      {history}
      {query}
      {copiedTs}
      onsave={save}
      onquerychange={(q) => (query = q)}
      onwipe={wipeHistory}
      oncopy={copyEntry}
      ondelete={deleteEntry}
    />
  {/if}
</main>
