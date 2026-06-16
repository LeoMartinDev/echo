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
    LANGUAGES,
    setHotkeyCapture,
    setSettings,
    type AppSettings,
    type DownloadEvent,
    type HistoryEntry,
    type ModelInfo,
  } from "./lib/api";
  import {
    t,
    setLocale,
    resolveLocale,
    localeTag,
    SUPPORTED,
    type Locale,
    type TKey,
  } from "./lib/i18n.svelte";

  let settings = $state<AppSettings | null>(null);
  let models = $state<ModelInfo[]>([]);
  let history = $state<HistoryEntry[]>([]);
  let copiedTs = $state<number | null>(null);
  let view = $state<"settings" | "history">("settings");
  let query = $state("");
  let memBytes = $state<number | null>(null);
  let progress = $state<Record<string, { pct: number | null; status: string }>>({});
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
        if (i === RETRY_DELAYS.length) { errorMsg = String(e); return; }
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
          [id]: { pct: total ? Math.round((received / total) * 100) : null, status },
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
  const MODIFIER_KEYS = new Set(["Control", "Shift", "Alt", "Meta"]);

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

  function keyFromEvent(e: KeyboardEvent): string | null {
    if (MODIFIER_KEYS.has(e.key)) return null;
    const code = e.code;
    if (/^F\d{1,2}$/.test(e.key)) return e.key;
    if (code.startsWith("Key")) return code.slice(3);
    if (code.startsWith("Digit")) return code.slice(5);
    const named: Record<string, string> = {
      Space: "Space",
      Tab: "Tab",
      Enter: "Enter",
      Backquote: "`",
      Minus: "-",
      Equal: "=",
      Comma: ",",
      Period: ".",
      Slash: "/",
      Semicolon: ";",
      Quote: "'",
      BracketLeft: "[",
      BracketRight: "]",
      Backslash: "\\",
      ArrowUp: "Up",
      ArrowDown: "Down",
      ArrowLeft: "Left",
      ArrowRight: "Right",
      Home: "Home",
      End: "End",
      PageUp: "PageUp",
      PageDown: "PageDown",
      Insert: "Insert",
      Delete: "Delete",
    };
    return named[code] ?? null;
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

  function fmtSize(mb: number): string {
    return mb >= 1000
      ? `${(mb / 1000).toFixed(1)} ${t("unit_gb")}`
      : `${mb} ${t("unit_mb")}`;
  }

  function fmtRam(bytes: number): string {
    const mb = bytes / 1_048_576;
    return mb >= 1000 ? `${(mb / 1024).toFixed(1)} Go` : `${Math.round(mb)} Mo`;
  }

  function fmtTime(tsMs: number): string {
    return new Date(tsMs).toLocaleTimeString(localeTag(), {
      hour: "2-digit",
      minute: "2-digit",
    });
  }

  function dayLabel(tsMs: number): string {
    const d = new Date(tsMs);
    const now = new Date();
    if (d.toDateString() === now.toDateString()) return t("today");
    if (d.toDateString() === new Date(now.getTime() - 86_400_000).toDateString())
      return t("yesterday");
    return d.toLocaleDateString(localeTag(), { day: "numeric", month: "long" });
  }

  const filteredHistory = $derived(
    query.trim()
      ? history.filter((e) =>
          e.text.toLowerCase().includes(query.trim().toLowerCase()),
        )
      : history,
  );

  const groupedHistory = $derived.by(() => {
    const groups: Array<{ label: string; entries: HistoryEntry[] }> = [];
    for (const e of filteredHistory) {
      const label = dayLabel(e.ts_ms);
      const last = groups[groups.length - 1];
      if (last && last.label === label) last.entries.push(e);
      else groups.push({ label, entries: [e] });
    }
    return groups;
  });

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

  // Translated description by id; falls back to the value provided by the backend.
  function modelDesc(model: ModelInfo): string {
    const key = `model_desc.${model.id}` as TKey;
    const translated = t(key);
    return translated === key ? model.description : translated;
  }
</script>

<svelte:window onkeydown={onHotkeyKeydown} onblur={endCapture} />

<header class="fixed top-0 left-0 right-0 z-100 bg-bg border-b border-border animate-[rise_0.45s_cubic-bezier(0.2,0.7,0.2,1)_both]" style="animation-delay: calc(var(--d) * 55ms); --d: 0">
  <div class="max-w-[448px] mx-auto py-2.5 px-6 flex items-baseline justify-between gap-4">
    <div class="flex items-baseline gap-2">
      <svg class="w-[15px] h-[15px] text-accent translate-y-0.5" viewBox="0 0 16 16" aria-hidden="true">
        <!-- graft glyph: stem + insertion -->
        <path
          d="M8 14V7M8 7C8 4.5 9.5 3 12 3C12 5.5 10.5 7 8 7ZM8 10C8 8 6.8 6.8 4.5 6.8C4.5 9 5.8 10 8 10Z"
          fill="none"
          stroke="currentColor"
          stroke-width="1.2"
          stroke-linecap="round"
          stroke-linejoin="round"
        />
      </svg>
      <h1 class="m-0 font-mono text-base font-semibold tracking-[-0.01em]">echo</h1>
      <span class="font-mono text-[10.5px] text-fg-faint">0.1.0</span>
    </div>
    <p class="m-0 flex items-center gap-[7px] text-fg-muted">
      {#if activeModel}
        <span class="w-1.5 h-1.5 rounded-full shrink-0 bg-accent shadow-[0_0_6px_rgba(122,162,247,0.55)]"></span>
        <span class="font-mono text-[11px] tracking-[0.02em]">{activeModel.name.toLowerCase()}</span>
      {:else}
        <span class="w-1.5 h-1.5 rounded-full shrink-0 bg-[#c9a35a]"></span>
        <span class="font-mono text-[11px] tracking-[0.02em]">{t("no_model")}</span>
      {/if}
      {#if memBytes !== null}
        <span class="font-mono text-[11px] tracking-[0.02em] inline-flex items-center gap-1 text-fg-faint tabular-nums">
          <svg class="w-[11px] h-[11px]" viewBox="0 0 16 16" aria-hidden="true">
            <rect
              x="3.25"
              y="4.75"
              width="9.5"
              height="6.5"
              rx="1.25"
              fill="none"
              stroke="currentColor"
              stroke-width="1.1"
            />
            <path
              d="M5.5 4.75V2.9M8 4.75V2.9M10.5 4.75V2.9M5.5 13.1v-1.85M8 13.1v-1.85M10.5 13.1v-1.85"
              stroke="currentColor"
              stroke-width="1.1"
              stroke-linecap="round"
            />
          </svg>
          {fmtRam(memBytes)}
        </span>
      {/if}
    </p>
  </div>
</header>

<main class="max-w-[448px] mx-auto pt-[52px] px-6 pb-14">
  <p class="my-[14px] mb-[30px] text-fg-muted max-w-[46ch] animate-[rise_0.45s_cubic-bezier(0.2,0.7,0.2,1)_both]" style="animation-delay: calc(var(--d) * 55ms); --d: 1">
    {t("lede_pre")} <kbd class="font-mono text-[11px] tracking-[0.02em] px-[7px] py-[1.5px] pb-[2.5px] rounded border border-border-strong border-b-2 bg-bg-raised text-fg whitespace-nowrap">{settings?.hotkey ?? "…"}</kbd> {t("lede_post")}
  </p>

  {#if errorMsg}
    <div class="flex items-center justify-between gap-[14px] px-[14px] py-2.5 mb-[18px] border border-border border-l-2 border-l-danger rounded-md bg-bg-raised text-fg" role="alert">
      <span>{errorMsg}</span>
      <button class="bg-transparent border-none text-fg-faint cursor-pointer text-[11px] p-1 hover:text-fg" onclick={() => (errorMsg = null)} aria-label={t("close")}>✕</button>
    </div>
  {/if}

  {#if isMac && !accessibilityOk}
    <div class="flex items-center justify-between gap-[14px] px-[14px] py-2.5 mb-[18px] border border-border border-l-2 border-l-accent rounded-md bg-bg-raised text-fg-muted">
      <span>
        {t("a11y_before")} <strong class="text-fg font-semibold">{t("a11y_perm")}</strong> {t("a11y_after")}
      </span>
      <button
        class="font-mono text-[11px] tracking-[0.02em] px-3 py-[5px] rounded-[5px] border border-border-strong bg-transparent text-fg cursor-pointer transition-[background,border-color] duration-120 ease-[ease] hover:bg-bg-hover hover:border-white/24 focus-visible:outline focus-visible:outline-1 focus-visible:outline-accent focus-visible:outline-offset-1"
        onclick={async () => {
          await accessibilityStatus(true);
          accessibilityOk = await accessibilityStatus(false);
        }}>{t("authorize")}</button
      >
    </div>
  {/if}

  {#if updateVersion}
    <div class="flex items-center justify-between gap-[14px] px-[14px] py-2.5 mb-[18px] border border-border border-l-2 border-l-accent rounded-md bg-bg-raised text-fg-muted">
      <span>{t("update_available", { version: updateVersion })}</span>
      <button class="font-mono text-[11px] tracking-[0.02em] px-3 py-[5px] rounded-[5px] border border-accent/50 bg-transparent text-accent cursor-pointer transition-[background,border-color] duration-120 ease-[ease] hover:bg-accent-dim focus-visible:outline focus-visible:outline-1 focus-visible:outline-accent focus-visible:outline-offset-1" onclick={() => installUpdate()}>{t("update_restart")}</button>
    </div>
  {/if}

  <nav class="flex gap-5 mt-[22px] border-b border-border animate-[rise_0.45s_cubic-bezier(0.2,0.7,0.2,1)_both]" style="animation-delay: calc(var(--d) * 55ms); --d: 2">
    <button
      class="font-mono text-[11px] tracking-[0.04em] px-px pt-[7px] pb-[9px] bg-transparent border-none border-b -mb-px cursor-pointer transition-[color] duration-120 ease-[ease] {view === 'settings' ? 'text-fg border-b-fg' : 'border-b-transparent text-fg-faint hover:text-fg-muted'}"
      onclick={() => (view = "settings")}
    >
      {t("tab_settings")}
    </button>
    <button
      class="font-mono text-[11px] tracking-[0.04em] px-px pt-[7px] pb-[9px] bg-transparent border-none border-b -mb-px cursor-pointer transition-[color] duration-120 ease-[ease] {view === 'history' ? 'text-fg border-b-fg' : 'border-b-transparent text-fg-faint hover:text-fg-muted'}"
      onclick={() => (view = "history")}
    >
      {t("tab_history")}{#if history.length > 0}<span class="ml-[6px] text-fg-faint tabular-nums">{history.length}</span>{/if}
    </button>
  </nav>

  {#if view === "settings" && settings}
    <section class="mt-[30px] animate-[rise_0.45s_cubic-bezier(0.2,0.7,0.2,1)_both]" style="animation-delay: calc(var(--d) * 55ms); --d: 3">
      <button
        type="button"
        class="relative flex flex-col items-center gap-[18px] w-full px-6 pt-[34px] pb-[26px] border rounded-xl bg-bg-raised cursor-pointer transition-[border-color,background] duration-150 ease-[ease] {capturingHotkey ? 'border-accent bg-accent-dim' : 'border-border hover:border-border-strong hover:bg-bg-hover'} focus-visible:outline focus-visible:outline-1 focus-visible:outline-accent focus-visible:outline-offset-2"
        onclick={() => (capturingHotkey ? endCapture() : startCapture())}
        aria-label={t("ptt_label")}
      >
        {#if !capturingHotkey}
          <span class="absolute top-3 right-[14px] font-mono text-[10px] tracking-[0.04em] text-fg-faint opacity-0 transition-opacity duration-150 ease-[ease] group-hover:opacity-100">{t("ptt_change")}</span>
        {/if}
        <div class="flex items-center gap-[10px] min-h-14">
          {#if capturingHotkey}
            <span class="w-[9px] h-[9px] rounded-full bg-accent animate-[blink_1s_ease-in-out_infinite]"></span>
            <span class="font-mono text-[19px] tracking-[0.01em] text-accent">{t("press_key")}</span>
          {:else}
            {#each settings.hotkey.split("+") as part, i}
              {#if i > 0}<span class="font-mono text-[17px] text-fg-faint">+</span>{/if}
              <kbd class="font-mono text-[26px] font-medium px-[18px] py-[9px] pb-3 rounded-[10px] border border-border-strong border-b-[3px] bg-bg text-fg leading-none whitespace-nowrap">{part}</kbd>
            {/each}
          {/if}
        </div>
        <div class="flex flex-col items-center gap-[3px]">
          <span class="text-[13px] font-semibold text-fg">{t("ptt_label")}</span>
          <span class="text-[11.5px] text-fg-faint">{t("ptt_hint")}</span>
        </div>
      </button>
    </section>

    <section class="mt-[34px] animate-[rise_0.45s_cubic-bezier(0.2,0.7,0.2,1)_both]" style="animation-delay: calc(var(--d) * 55ms); --d: 4">
      <h2 class="font-mono text-[10.5px] font-medium uppercase tracking-[0.14em] text-fg-faint m-0 mb-1 pb-2 border-b border-border">{t("sec_insertion")}</h2>
      <div class="flex items-center justify-between gap-6 py-[14px] border-b border-border">
        <div class="flex flex-col gap-0.5 text-fg">
          {t("write_mode")}
          <span class="text-[11.5px] text-fg-faint">
            {t("write_mode_hint")}
          </span>
        </div>
        <div class="shrink-0">
          <div class="inline-flex border border-border-strong rounded-[5px] overflow-hidden" role="radiogroup" aria-label={t("write_mode")}>
            <button
              class="font-mono text-[11px] px-3 py-[5px] cursor-pointer transition-[background] duration-120 ease-[ease] border-r border-border hover:bg-bg-hover focus-visible:outline focus-visible:outline-1 focus-visible:outline-accent focus-visible:outline-offset-1 {settings.insertion_mode === 'live' ? 'bg-accent-dim text-accent' : 'text-fg-muted'}"
              onclick={() => save({ insertion_mode: "live" })}
              role="radio"
              aria-checked={settings.insertion_mode === "live"}
            >
              {t("mode_live")}
            </button>
            <button
              class="font-mono text-[11px] px-3 py-[5px] cursor-pointer transition-[background] duration-120 ease-[ease] hover:bg-bg-hover focus-visible:outline focus-visible:outline-1 focus-visible:outline-accent focus-visible:outline-offset-1 {settings.insertion_mode === 'on_release' ? 'bg-accent-dim text-accent' : 'text-fg-muted'}"
              onclick={() => save({ insertion_mode: "on_release" })}
              role="radio"
              aria-checked={settings.insertion_mode === "on_release"}
            >
              {t("mode_on_release")}
            </button>
          </div>
        </div>
      </div>
      <div class="flex items-center justify-between gap-6 py-[14px] border-b border-border">
        <div class="flex flex-col gap-0.5 text-fg">
          {t("lang_label")}
          <span class="text-[11.5px] text-fg-faint">{t("lang_hint")}</span>
        </div>
        <div class="shrink-0">
          <select
            value={settings.language ?? ""}
            onchange={(e) => save({ language: e.currentTarget.value || null })}
            class="font-mono text-[11px] pr-[28px] pl-[10px] py-[5px] rounded-[5px] border border-border-strong bg-bg text-fg appearance-none cursor-pointer hover:bg-bg-hover focus-visible:outline focus-visible:outline-1 focus-visible:outline-accent focus-visible:outline-offset-1"
            style="background-image: url(&quot;data:image/svg+xml;charset=utf-8,%3Csvg xmlns='http://www.w3.org/2000/svg' width='8' height='5'%3E%3Cpath d='M1 1l3 3 3-3' stroke='%2397978f' fill='none'/%3E%3C/svg%3E&quot;); background-repeat: no-repeat; background-position: right 9px center"
          >
            {#each LANGUAGES as lang}
              <option value={lang.code ?? ""} class="text-fg bg-bg-raised">{t(lang.key)}</option>
            {/each}
          </select>
        </div>
      </div>
    </section>

    <section class="mt-[34px] animate-[rise_0.45s_cubic-bezier(0.2,0.7,0.2,1)_both]" style="animation-delay: calc(var(--d) * 55ms); --d: 5">
      <h2 class="font-mono text-[10.5px] font-medium uppercase tracking-[0.14em] text-fg-faint m-0 mb-1 pb-2 border-b border-border">{t("sec_system")}</h2>
      <div class="flex items-center justify-between gap-6 py-[14px] border-b border-border">
        <div class="flex flex-col gap-0.5 text-fg">
          {t("ui_lang_label")}
          <span class="text-[11.5px] text-fg-faint">{t("ui_lang_hint")}</span>
        </div>
        <div class="shrink-0">
          <select
            value={settings.ui_language ?? ""}
            onchange={(e) => save({ ui_language: e.currentTarget.value || null })}
            class="font-mono text-[11px] pr-[28px] pl-[10px] py-[5px] rounded-[5px] border border-border-strong bg-bg text-fg appearance-none cursor-pointer hover:bg-bg-hover focus-visible:outline focus-visible:outline-1 focus-visible:outline-accent focus-visible:outline-offset-1"
            style="background-image: url(&quot;data:image/svg+xml;charset=utf-8,%3Csvg xmlns='http://www.w3.org/2000/svg' width='8' height='5'%3E%3Cpath d='M1 1l3 3 3-3' stroke='%2397978f' fill='none'/%3E%3C/svg%3E&quot;); background-repeat: no-repeat; background-position: right 9px center"
          >
            <option value="" class="text-fg bg-bg-raised">{t("ui_lang_auto")}</option>
            {#each SUPPORTED as code}
              <option value={code} class="text-fg bg-bg-raised">{t(("lang_" + code) as TKey)}</option>
            {/each}
          </select>
        </div>
      </div>
      <div class="flex items-center justify-between gap-6 py-[14px] border-b border-border">
        <div class="flex flex-col gap-0.5 text-fg">
          {t("autostart_label")}
          <span class="text-[11.5px] text-fg-faint">{t("autostart_hint")}</span>
        </div>
        <div class="shrink-0">
          <div class="inline-flex border border-border-strong rounded-[5px] overflow-hidden" role="radiogroup" aria-label={t("autostart_label")}>
            <button
              class="font-mono text-[11px] px-3 py-[5px] cursor-pointer transition-[background] duration-120 ease-[ease] border-r border-border hover:bg-bg-hover focus-visible:outline focus-visible:outline-1 focus-visible:outline-accent focus-visible:outline-offset-1 {settings.autostart ? 'bg-accent-dim text-accent' : 'text-fg-muted'}"
              onclick={() => save({ autostart: true })}
              role="radio"
              aria-checked={settings.autostart}
            >
              {t("on")}
            </button>
            <button
              class="font-mono text-[11px] px-3 py-[5px] cursor-pointer transition-[background] duration-120 ease-[ease] hover:bg-bg-hover focus-visible:outline focus-visible:outline-1 focus-visible:outline-accent focus-visible:outline-offset-1 {!settings.autostart ? 'bg-accent-dim text-accent' : 'text-fg-muted'}"
              onclick={() => save({ autostart: false })}
              role="radio"
              aria-checked={!settings.autostart}
            >
              {t("off")}
            </button>
          </div>
        </div>
      </div>
    </section>

    <section class="mt-[34px] animate-[rise_0.45s_cubic-bezier(0.2,0.7,0.2,1)_both]" style="animation-delay: calc(var(--d) * 55ms); --d: 6">
      <h2 class="font-mono text-[10.5px] font-medium uppercase tracking-[0.14em] text-fg-faint m-0 mb-1 pb-2 border-b border-border">{t("sec_models")}</h2>
      <ul class="list-none m-0 p-0">
        {#each models as model (model.id)}
          {@const p = progress[model.id]}
          <li class="flex items-start justify-between gap-[18px] px-[10px] py-[13px] -mx-[10px] border-b border-border rounded-md transition-[background] duration-120 ease-[ease] hover:bg-bg-raised">
            <div class="min-w-0">
              <div class="flex items-center gap-2">
                {#if model.active}<span class="w-1.5 h-1.5 rounded-full bg-accent shadow-[0_0_6px_rgba(122,162,247,0.55)] shrink-0" title={t("model_active_title")}></span>{/if}
                <strong class="font-semibold text-[13px]">{model.name}</strong>
                <span class="font-mono text-[9.5px] uppercase tracking-[0.08em] text-fg-faint border border-border rounded-[3px] px-[5px] py-px">{model.engine}</span>
              </div>
              <p class="mt-[3px] text-xs text-fg-muted leading-[1.45]">{modelDesc(model)}</p>
              {#if p}
                <div class="mt-[10px] h-0.5 rounded-[1px] bg-border overflow-hidden" aria-hidden="true">
                  <div
                    class="h-full bg-accent transition-[width] duration-250 ease-[ease] {p.pct === null || p.status === 'extracting' ? 'animate-[pulse_1.1s_ease-in-out_infinite]' : ''}"
                    style="width: {p.pct ?? 100}%"
                  ></div>
                </div>
                <span class="font-mono text-[11px] tracking-[0.02em] inline-block mt-[6px] text-fg-faint tabular-nums">
                  {p.status === "extracting"
                    ? t("extracting")
                    : p.pct !== null
                      ? `${p.pct} %`
                      : t("downloading")}
                </span>
              {/if}
            </div>
            <div class="flex items-center gap-2 shrink-0 pt-px">
              <span class="font-mono text-[11px] tracking-[0.02em] text-fg-faint tabular-nums">{fmtSize(model.size_mb)}</span>
              {#if !p}
                {#if model.downloaded}
                  {#if !model.active}
                    <button class="font-mono text-[11px] tracking-[0.02em] px-3 py-[5px] rounded-[5px] border border-accent/50 bg-transparent text-accent cursor-pointer transition-[background,border-color] duration-120 ease-[ease] hover:bg-accent-dim focus-visible:outline focus-visible:outline-1 focus-visible:outline-accent focus-visible:outline-offset-1" onclick={() => save({ model_id: model.id })}>
                      {t("use_model")}
                    </button>
                    <button class="font-mono text-[11px] tracking-[0.02em] px-3 py-[5px] rounded-[5px] border border-border-strong bg-transparent text-fg-faint cursor-pointer transition-[background,border-color] duration-120 ease-[ease] hover:text-danger hover:border-danger/50 hover:bg-danger/8 focus-visible:outline focus-visible:outline-1 focus-visible:outline-accent focus-visible:outline-offset-1" onclick={() => removeModel(model.id)}>
                      {t("delete_short")}
                    </button>
                  {:else}
                    <span class="font-mono text-[11px] tracking-[0.02em] text-accent">{t("active")}</span>
                  {/if}
                {:else}
                  <button class="font-mono text-[11px] tracking-[0.02em] px-3 py-[5px] rounded-[5px] border border-border-strong bg-transparent text-fg cursor-pointer transition-[background,border-color] duration-120 ease-[ease] hover:bg-bg-hover hover:border-white/24 focus-visible:outline focus-visible:outline-1 focus-visible:outline-accent focus-visible:outline-offset-1" onclick={() => startDownload(model.id)}>
                    {t("download")}
                  </button>
                {/if}
              {/if}
            </div>
          </li>
        {/each}
      </ul>
    </section>
  {:else if settings}
    <section class="animate-[rise_0.45s_cubic-bezier(0.2,0.7,0.2,1)_both]" style="animation-delay: calc(var(--d) * 55ms); --d: 3">
      <div class="flex items-center justify-between gap-6 py-[14px] border-b border-border">
        <div class="flex flex-col gap-0.5 text-fg">
          {t("keep_label")}
          <span class="text-[11.5px] text-fg-faint">{t("keep_hint")}</span>
        </div>
        <div class="shrink-0">
          <div class="inline-flex border border-border-strong rounded-[5px] overflow-hidden" role="radiogroup" aria-label={t("keep_label")}>
            <button
              class="font-mono text-[11px] px-3 py-[5px] cursor-pointer transition-[background] duration-120 ease-[ease] border-r border-border hover:bg-bg-hover focus-visible:outline focus-visible:outline-1 focus-visible:outline-accent focus-visible:outline-offset-1 {settings.history_enabled ? 'bg-accent-dim text-accent' : 'text-fg-muted'}"
              onclick={() => save({ history_enabled: true })}
              role="radio"
              aria-checked={settings.history_enabled}
            >
              {t("on")}
            </button>
            <button
              class="font-mono text-[11px] px-3 py-[5px] cursor-pointer transition-[background] duration-120 ease-[ease] hover:bg-bg-hover focus-visible:outline focus-visible:outline-1 focus-visible:outline-accent focus-visible:outline-offset-1 {!settings.history_enabled ? 'bg-accent-dim text-accent' : 'text-fg-muted'}"
              onclick={() => save({ history_enabled: false })}
              role="radio"
              aria-checked={!settings.history_enabled}
            >
              {t("off")}
            </button>
          </div>
        </div>
      </div>

      {#if history.length > 0}
        <div class="flex gap-2 items-center py-[14px] pb-1">
          <input
            class="flex-1 font-mono text-[11px] tracking-[0.02em] px-[10px] py-[5px] rounded-[5px] border border-border-strong bg-transparent text-fg placeholder:text-fg-faint focus-visible:outline focus-visible:outline-1 focus-visible:outline-accent focus-visible:outline-offset-1"
            type="text"
            placeholder={t("filter_placeholder")}
            bind:value={query}
          />
          <button class="font-mono text-[11px] tracking-[0.02em] px-[9px] py-[2.5px] rounded-[5px] border border-border-strong bg-transparent text-fg-faint cursor-pointer transition-[background,border-color] duration-120 ease-[ease] hover:text-danger hover:border-danger/50 hover:bg-danger/8 focus-visible:outline focus-visible:outline-1 focus-visible:outline-accent focus-visible:outline-offset-1" onclick={wipeHistory}>{t("clear_all")}</button>
        </div>
      {/if}

      {#if filteredHistory.length === 0}
        <p class="font-mono text-[11px] tracking-[0.02em] text-fg-faint py-3 m-0">
          {history.length === 0 ? t("empty_none") : t("empty_no_results")}
        </p>
      {:else}
        {#each groupedHistory as group (group.label)}
          <div class="font-mono text-[11px] tracking-[0.02em] text-fg-faint text-[10px] uppercase tracking-[0.12em] mt-[18px] mb-0.5">{group.label}</div>
          <ul class="list-none m-0 p-0">
            {#each group.entries as entry (entry.ts_ms)}
              <li class="group flex items-center gap-3 px-[10px] py-[9px] -mx-[10px] border-b border-border rounded-md hover:bg-bg-raised">
                <span class="font-mono text-[11px] tracking-[0.02em] text-fg-faint shrink-0 tabular-nums">{fmtTime(entry.ts_ms)}</span>
                <span class="flex-1 min-w-0 text-[12.5px] text-fg-muted truncate" title={entry.text}>{entry.text}</span>
                <div class="shrink-0 flex gap-[6px] invisible group-hover:visible">
                  <button class="font-mono text-[11px] tracking-[0.02em] px-[9px] py-[2.5px] rounded-[5px] border border-border-strong bg-transparent text-fg cursor-pointer transition-[background,border-color] duration-120 ease-[ease] hover:bg-bg-hover hover:border-white/24 focus-visible:outline focus-visible:outline-1 focus-visible:outline-accent focus-visible:outline-offset-1" onclick={() => copyEntry(entry)}>
                    {copiedTs === entry.ts_ms ? t("copied") : t("copy")}
                  </button>
                  <button
                    class="font-mono text-[11px] tracking-[0.02em] px-2 py-[2.5px] leading-none rounded-[5px] border border-border-strong bg-transparent text-fg-faint cursor-pointer transition-[background,border-color] duration-120 ease-[ease] hover:text-danger hover:border-danger/50 hover:bg-danger/8 focus-visible:outline focus-visible:outline-1 focus-visible:outline-accent focus-visible:outline-offset-1"
                    title={t("delete_entry")}
                    aria-label={t("delete_entry")}
                    onclick={() => deleteEntry(entry)}>✕</button
                  >
                </div>
              </li>
            {/each}
          </ul>
        {/each}
      {/if}
    </section>
  {/if}
</main>
