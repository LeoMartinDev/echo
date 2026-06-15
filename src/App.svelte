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

  async function refresh() {
    [settings, models] = await Promise.all([getSettings(), listModels()]);
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

<main>
  <header class="reveal" style="--d: 0">
    <div class="brand">
      <svg class="mark" viewBox="0 0 16 16" aria-hidden="true">
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
      <h1>echo</h1>
      <span class="version">0.1.0</span>
    </div>
    <p class="status">
      {#if activeModel}
        <span class="dot ok"></span>
        <span class="mono">{activeModel.name.toLowerCase()}</span>
      {:else}
        <span class="dot warn"></span>
        <span class="mono">{t("no_model")}</span>
      {/if}
      {#if memBytes !== null}
        <span class="mono ram">
          <svg class="ram-icon" viewBox="0 0 16 16" aria-hidden="true">
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
  </header>

  <p class="lede reveal" style="--d: 1">
    {t("lede_pre")} <kbd>{settings?.hotkey ?? "…"}</kbd> {t("lede_post")}
  </p>

  {#if errorMsg}
    <div class="notice error" role="alert">
      <span>{errorMsg}</span>
      <button class="dismiss" onclick={() => (errorMsg = null)} aria-label={t("close")}>✕</button>
    </div>
  {/if}

  {#if isMac && !accessibilityOk}
    <div class="notice">
      <span>
        {t("a11y_before")} <strong>{t("a11y_perm")}</strong> {t("a11y_after")}
      </span>
      <button
        class="btn"
        onclick={async () => {
          await accessibilityStatus(true);
          accessibilityOk = await accessibilityStatus(false);
        }}>{t("authorize")}</button
      >
    </div>
  {/if}

  {#if updateVersion}
    <div class="notice update">
      <span>{t("update_available", { version: updateVersion })}</span>
      <button class="btn primary" onclick={() => installUpdate()}>{t("update_restart")}</button>
    </div>
  {/if}

  <nav class="tabs reveal" style="--d: 2">
    <button class="tab" class:on={view === "settings"} onclick={() => (view = "settings")}>
      {t("tab_settings")}
    </button>
    <button class="tab" class:on={view === "history"} onclick={() => (view = "history")}>
      {t("tab_history")}{#if history.length > 0}<span class="count">{history.length}</span>{/if}
    </button>
  </nav>

  {#if !settings}
    <p class="mono loading">{t("loading")}</p>
  {:else if view === "settings"}
    <section class="reveal hero-section" style="--d: 3">
      <button
        type="button"
        class="hotkey-hero"
        class:capturing={capturingHotkey}
        onclick={() => (capturingHotkey ? endCapture() : startCapture())}
        aria-label={t("ptt_label")}
      >
        {#if !capturingHotkey}
          <span class="hero-edit">{t("ptt_change")}</span>
        {/if}
        <div class="hero-keys">
          {#if capturingHotkey}
            <span class="recording-dot big"></span>
            <span class="hero-prompt">{t("press_key")}</span>
          {:else}
            {#each settings.hotkey.split("+") as part, i}
              {#if i > 0}<span class="hero-plus">+</span>{/if}
              <kbd class="hero-kbd">{part}</kbd>
            {/each}
          {/if}
        </div>
        <div class="hero-meta">
          <span class="hero-title">{t("ptt_label")}</span>
          <span class="hero-hint">{t("ptt_hint")}</span>
        </div>
      </button>
    </section>

    <section class="reveal" style="--d: 4">
      <h2>{t("sec_insertion")}</h2>
      <div class="row">
        <div class="row-label">
          {t("write_mode")}
          <span class="row-hint">
            {t("write_mode_hint")}
          </span>
        </div>
        <div class="row-control">
          <div class="segmented" role="radiogroup" aria-label={t("write_mode")}>
            <button
              class="seg"
              class:on={settings.insertion_mode === "live"}
              onclick={() => save({ insertion_mode: "live" })}
              role="radio"
              aria-checked={settings.insertion_mode === "live"}
            >
              {t("mode_live")}
            </button>
            <button
              class="seg"
              class:on={settings.insertion_mode === "on_release"}
              onclick={() => save({ insertion_mode: "on_release" })}
              role="radio"
              aria-checked={settings.insertion_mode === "on_release"}
            >
              {t("mode_on_release")}
            </button>
          </div>
        </div>
      </div>
      <div class="row">
        <div class="row-label">
          {t("lang_label")}
          <span class="row-hint">{t("lang_hint")}</span>
        </div>
        <div class="row-control">
          <select
            value={settings.language ?? ""}
            onchange={(e) => save({ language: e.currentTarget.value || null })}
          >
            {#each LANGUAGES as lang}
              <option value={lang.code ?? ""}>{t(lang.key)}</option>
            {/each}
          </select>
        </div>
      </div>
    </section>

    <section class="reveal" style="--d: 5">
      <h2>{t("sec_system")}</h2>
      <div class="row">
        <div class="row-label">
          {t("ui_lang_label")}
          <span class="row-hint">{t("ui_lang_hint")}</span>
        </div>
        <div class="row-control">
          <select
            value={settings.ui_language ?? ""}
            onchange={(e) => save({ ui_language: e.currentTarget.value || null })}
          >
            <option value="">{t("ui_lang_auto")}</option>
            {#each SUPPORTED as code}
              <option value={code}>{t(("lang_" + code) as TKey)}</option>
            {/each}
          </select>
        </div>
      </div>
      <div class="row">
        <div class="row-label">
          {t("autostart_label")}
          <span class="row-hint">{t("autostart_hint")}</span>
        </div>
        <div class="row-control">
          <div class="segmented" role="radiogroup" aria-label={t("autostart_label")}>
            <button
              class="seg"
              class:on={settings.autostart}
              onclick={() => save({ autostart: true })}
              role="radio"
              aria-checked={settings.autostart}
            >
              {t("on")}
            </button>
            <button
              class="seg"
              class:on={!settings.autostart}
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

    <section class="reveal" style="--d: 6">
      <h2>{t("sec_models")}</h2>
      <ul class="models">
        {#each models as model (model.id)}
          {@const p = progress[model.id]}
          <li class="model" class:active={model.active}>
            <div class="model-main">
              <div class="model-name">
                {#if model.active}<span class="active-dot" title={t("model_active_title")}></span>{/if}
                <strong>{model.name}</strong>
                <span class="tag">{model.engine}</span>
              </div>
              <p class="model-desc">{modelDesc(model)}</p>
              {#if p}
                <div class="progress" aria-hidden="true">
                  <div
                    class="progress-bar"
                    class:indeterminate={p.pct === null || p.status === "extracting"}
                    style="width: {p.pct ?? 100}%"
                  ></div>
                </div>
                <span class="mono progress-label">
                  {p.status === "extracting"
                    ? t("extracting")
                    : p.pct !== null
                      ? `${p.pct} %`
                      : t("downloading")}
                </span>
              {/if}
            </div>
            <div class="model-side">
              <span class="mono size">{fmtSize(model.size_mb)}</span>
              {#if !p}
                {#if model.downloaded}
                  {#if !model.active}
                    <button class="btn primary" onclick={() => save({ model_id: model.id })}>
                      {t("use_model")}
                    </button>
                    <button class="btn danger" onclick={() => removeModel(model.id)}>
                      {t("delete_short")}
                    </button>
                  {:else}
                    <span class="mono active-label">{t("active")}</span>
                  {/if}
                {:else}
                  <button class="btn" onclick={() => startDownload(model.id)}>
                    {t("download")}
                  </button>
                {/if}
              {/if}
            </div>
          </li>
        {/each}
      </ul>
    </section>
  {:else}
    <section class="reveal" style="--d: 3">
      <div class="row">
        <div class="row-label">
          {t("keep_label")}
          <span class="row-hint">{t("keep_hint")}</span>
        </div>
        <div class="row-control">
          <div class="segmented" role="radiogroup" aria-label={t("keep_label")}>
            <button
              class="seg"
              class:on={settings.history_enabled}
              onclick={() => save({ history_enabled: true })}
              role="radio"
              aria-checked={settings.history_enabled}
            >
              {t("on")}
            </button>
            <button
              class="seg"
              class:on={!settings.history_enabled}
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
        <div class="history-tools">
          <input
            class="search"
            type="text"
            placeholder={t("filter_placeholder")}
            bind:value={query}
          />
          <button class="btn danger small" onclick={wipeHistory}>{t("clear_all")}</button>
        </div>
      {/if}

      {#if filteredHistory.length === 0}
        <p class="mono empty">
          {history.length === 0 ? t("empty_none") : t("empty_no_results")}
        </p>
      {:else}
        {#each groupedHistory as group (group.label)}
          <div class="mono day">{group.label}</div>
          <ul class="history">
            {#each group.entries as entry (entry.ts_ms)}
              <li class="entry">
                <span class="mono when">{fmtTime(entry.ts_ms)}</span>
                <span class="entry-text" title={entry.text}>{entry.text}</span>
                <div class="entry-actions">
                  <button class="btn small copy" onclick={() => copyEntry(entry)}>
                    {copiedTs === entry.ts_ms ? t("copied") : t("copy")}
                  </button>
                  <button
                    class="btn small danger del"
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

<style>
  :global(:root) {
    --bg: #0d0e10;
    --bg-raised: #141518;
    --bg-hover: #1a1b1f;
    --border: rgba(255, 255, 255, 0.07);
    --border-strong: rgba(255, 255, 255, 0.16);
    --fg: #e6e6e3;
    --fg-muted: #97978f;
    --fg-faint: #5b5b55;
    --accent: #7aa2f7;
    --accent-dim: rgba(122, 162, 247, 0.14);
    --danger: #cf7a6d;
    --mono:
      ui-monospace, "SF Mono", "JetBrains Mono", "Cascadia Code", Menlo,
      monospace;
    --sans:
      -apple-system, BlinkMacSystemFont, "Segoe UI", "Helvetica Neue", sans-serif;
  }

  :global(html) {
    background: var(--bg);
    color-scheme: dark;
  }

  :global(body) {
    margin: 0;
    background: var(--bg);
    color: var(--fg);
    font-family: var(--sans);
    font-size: 13px;
    line-height: 1.5;
    -webkit-font-smoothing: antialiased;
  }

  :global(::selection) {
    background: var(--accent-dim);
  }

  main {
    max-width: 620px;
    margin: 0 auto;
    padding: 36px 32px 56px;
  }

  /* — entrance animation on load, subtle and one-shot — */
  .reveal {
    animation: rise 0.45s cubic-bezier(0.2, 0.7, 0.2, 1) both;
    animation-delay: calc(var(--d) * 55ms);
  }

  @keyframes rise {
    from {
      opacity: 0;
      transform: translateY(6px);
    }
    to {
      opacity: 1;
      transform: none;
    }
  }

  .mono {
    font-family: var(--mono);
    font-size: 11px;
    letter-spacing: 0.02em;
  }

  /* ————— header ————— */

  header {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 16px;
  }

  .brand {
    display: flex;
    align-items: baseline;
    gap: 8px;
  }

  .mark {
    width: 15px;
    height: 15px;
    color: var(--accent);
    transform: translateY(2px);
  }

  h1 {
    margin: 0;
    font-family: var(--mono);
    font-size: 16px;
    font-weight: 600;
    letter-spacing: -0.01em;
  }

  .version {
    font-family: var(--mono);
    font-size: 10.5px;
    color: var(--fg-faint);
  }

  .status {
    margin: 0;
    display: flex;
    align-items: center;
    gap: 7px;
    color: var(--fg-muted);
  }

  .dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    flex: none;
  }

  .dot.ok {
    background: var(--accent);
    box-shadow: 0 0 6px rgba(122, 162, 247, 0.55);
  }

  .dot.warn {
    background: #c9a35a;
  }

  .ram {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    color: var(--fg-faint);
    font-variant-numeric: tabular-nums;
  }

  .ram-icon {
    width: 11px;
    height: 11px;
  }

  .lede {
    margin: 14px 0 30px;
    color: var(--fg-muted);
    max-width: 46ch;
  }

  kbd {
    font-family: var(--mono);
    font-size: 11px;
    padding: 1.5px 7px 2.5px;
    border-radius: 4px;
    border: 1px solid var(--border-strong);
    border-bottom-width: 2px;
    background: var(--bg-raised);
    color: var(--fg);
    white-space: nowrap;
  }

  /* ————— notices ————— */

  .notice {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 14px;
    padding: 10px 14px;
    margin-bottom: 18px;
    border: 1px solid var(--border);
    border-left: 2px solid var(--accent);
    border-radius: 6px;
    background: var(--bg-raised);
    color: var(--fg-muted);
  }

  .notice strong {
    color: var(--fg);
    font-weight: 600;
  }

  .notice.error {
    border-left-color: var(--danger);
    color: var(--fg);
  }

  .dismiss {
    background: none;
    border: none;
    color: var(--fg-faint);
    cursor: pointer;
    font-size: 11px;
    padding: 4px;
  }

  .dismiss:hover {
    color: var(--fg);
  }

  /* ————— sections / rows ————— */

  section {
    margin-top: 34px;
  }

  h2 {
    font-family: var(--mono);
    font-size: 10.5px;
    font-weight: 500;
    text-transform: uppercase;
    letter-spacing: 0.14em;
    color: var(--fg-faint);
    margin: 0 0 4px;
    padding-bottom: 8px;
    border-bottom: 1px solid var(--border);
  }

  .row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 24px;
    padding: 14px 0;
    border-bottom: 1px solid var(--border);
  }

  .row-label {
    display: flex;
    flex-direction: column;
    gap: 2px;
    color: var(--fg);
  }

  .row-hint {
    font-size: 11.5px;
    color: var(--fg-faint);
  }

  .row-control {
    flex: none;
  }

  /* ————— controls ————— */

  .btn {
    font-family: var(--mono);
    font-size: 11px;
    letter-spacing: 0.02em;
    padding: 5px 12px;
    border-radius: 5px;
    border: 1px solid var(--border-strong);
    background: transparent;
    color: var(--fg);
    cursor: pointer;
    transition:
      background 0.12s ease,
      border-color 0.12s ease;
  }

  .btn:hover {
    background: var(--bg-hover);
    border-color: rgba(255, 255, 255, 0.24);
  }

  .btn:focus-visible,
  .seg:focus-visible,
  select:focus-visible {
    outline: 1px solid var(--accent);
    outline-offset: 1px;
  }

  .btn.primary {
    border-color: rgba(122, 162, 247, 0.5);
    color: var(--accent);
  }

  .btn.primary:hover {
    background: var(--accent-dim);
  }

  .btn.danger {
    color: var(--fg-faint);
  }

  .btn.danger:hover {
    color: var(--danger);
    border-color: rgba(207, 122, 109, 0.5);
    background: rgba(207, 122, 109, 0.08);
  }

  /* ————— hotkey hero ————— */

  .hero-section {
    margin-top: 30px;
  }

  .hotkey-hero {
    position: relative;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 18px;
    width: 100%;
    padding: 34px 24px 26px;
    border: 1px solid var(--border);
    border-radius: 12px;
    background: var(--bg-raised);
    cursor: pointer;
    transition:
      border-color 0.15s ease,
      background 0.15s ease;
  }

  .hotkey-hero:hover {
    border-color: var(--border-strong);
    background: var(--bg-hover);
  }

  .hotkey-hero:focus-visible {
    outline: 1px solid var(--accent);
    outline-offset: 2px;
  }

  .hotkey-hero.capturing {
    border-color: var(--accent);
    background: var(--accent-dim);
  }

  .hero-edit {
    position: absolute;
    top: 12px;
    right: 14px;
    font-family: var(--mono);
    font-size: 10px;
    letter-spacing: 0.04em;
    color: var(--fg-faint);
    opacity: 0;
    transition: opacity 0.15s ease;
  }

  .hotkey-hero:hover .hero-edit {
    opacity: 1;
  }

  .hero-keys {
    display: flex;
    align-items: center;
    gap: 10px;
    min-height: 56px;
  }

  .hero-kbd {
    font-family: var(--mono);
    font-size: 26px;
    font-weight: 500;
    padding: 9px 18px 12px;
    border-radius: 10px;
    border: 1px solid var(--border-strong);
    border-bottom-width: 3px;
    background: var(--bg);
    color: var(--fg);
    line-height: 1;
    white-space: nowrap;
  }

  .hero-plus {
    font-family: var(--mono);
    font-size: 17px;
    color: var(--fg-faint);
  }

  .hero-prompt {
    font-family: var(--mono);
    font-size: 19px;
    letter-spacing: 0.01em;
    color: var(--accent);
  }

  .hero-meta {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 3px;
  }

  .hero-title {
    font-size: 13px;
    font-weight: 600;
    color: var(--fg);
  }

  .hero-hint {
    font-size: 11.5px;
    color: var(--fg-faint);
  }

  .recording-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--accent);
    animation: blink 1s ease-in-out infinite;
  }

  .recording-dot.big {
    width: 9px;
    height: 9px;
  }

  @keyframes blink {
    50% {
      opacity: 0.25;
    }
  }

  .segmented {
    display: inline-flex;
    border: 1px solid var(--border-strong);
    border-radius: 5px;
    overflow: hidden;
  }

  .seg {
    font-family: var(--mono);
    font-size: 11px;
    padding: 5px 12px;
    border: none;
    background: transparent;
    color: var(--fg-muted);
    cursor: pointer;
    transition: background 0.12s ease;
  }

  .seg + .seg {
    border-left: 1px solid var(--border);
  }

  .seg:hover {
    background: var(--bg-hover);
  }

  .seg.on {
    background: var(--accent-dim);
    color: var(--accent);
  }

  select {
    font-family: var(--mono);
    font-size: 11px;
    padding: 5px 28px 5px 10px;
    border-radius: 5px;
    border: 1px solid var(--border-strong);
    background: var(--bg) no-repeat right 9px center;
    background-image: url("data:image/svg+xml;charset=utf-8,%3Csvg xmlns='http://www.w3.org/2000/svg' width='8' height='5'%3E%3Cpath d='M1 1l3 3 3-3' stroke='%2397978f' fill='none'/%3E%3C/svg%3E");
    color: var(--fg);
    appearance: none;
    cursor: pointer;
  }

  select:hover {
    background-color: var(--bg-hover);
  }

  /* ————— models ————— */

  .models {
    list-style: none;
    margin: 0;
    padding: 0;
  }

  .model {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 18px;
    padding: 13px 10px;
    margin: 0 -10px;
    border-bottom: 1px solid var(--border);
    border-radius: 6px;
    transition: background 0.12s ease;
  }

  .model:hover {
    background: var(--bg-raised);
  }

  .model-main {
    min-width: 0;
  }

  .model-name {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .model-name strong {
    font-weight: 600;
    font-size: 13px;
  }

  .active-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--accent);
    box-shadow: 0 0 6px rgba(122, 162, 247, 0.55);
    flex: none;
  }

  .tag {
    font-family: var(--mono);
    font-size: 9.5px;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--fg-faint);
    border: 1px solid var(--border);
    border-radius: 3px;
    padding: 1px 5px;
  }

  .model-desc {
    margin: 3px 0 0;
    font-size: 12px;
    color: var(--fg-muted);
    line-height: 1.45;
  }

  .model-side {
    display: flex;
    align-items: center;
    gap: 8px;
    flex: none;
    padding-top: 1px;
  }

  .size {
    color: var(--fg-faint);
    font-variant-numeric: tabular-nums;
  }

  .active-label {
    color: var(--accent);
  }

  .progress {
    margin-top: 10px;
    height: 2px;
    border-radius: 1px;
    background: var(--border);
    overflow: hidden;
  }

  .progress-bar {
    height: 100%;
    background: var(--accent);
    transition: width 0.25s ease;
  }

  .progress-bar.indeterminate {
    animation: pulse 1.1s ease-in-out infinite;
  }

  @keyframes pulse {
    50% {
      opacity: 0.35;
    }
  }

  .progress-label {
    display: inline-block;
    margin-top: 6px;
    color: var(--fg-faint);
    font-variant-numeric: tabular-nums;
  }

  .loading {
    color: var(--fg-faint);
  }

  /* ————— tabs ————— */

  .tabs {
    display: flex;
    gap: 20px;
    margin-top: 22px;
    border-bottom: 1px solid var(--border);
  }

  .tab {
    font-family: var(--mono);
    font-size: 11px;
    letter-spacing: 0.04em;
    padding: 7px 1px 9px;
    background: none;
    border: none;
    border-bottom: 1px solid transparent;
    margin-bottom: -1px;
    color: var(--fg-faint);
    cursor: pointer;
    transition: color 0.12s ease;
  }

  .tab:hover {
    color: var(--fg-muted);
  }

  .tab.on {
    color: var(--fg);
    border-bottom-color: var(--fg);
  }

  .count {
    margin-left: 6px;
    color: var(--fg-faint);
    font-variant-numeric: tabular-nums;
  }

  /* ————— history ————— */

  .history-tools {
    display: flex;
    gap: 8px;
    align-items: center;
    padding: 14px 0 4px;
  }

  .search {
    flex: 1;
    font-family: var(--mono);
    font-size: 11px;
    letter-spacing: 0.02em;
    padding: 5px 10px;
    border-radius: 5px;
    border: 1px solid var(--border-strong);
    background: transparent;
    color: var(--fg);
  }

  .search::placeholder {
    color: var(--fg-faint);
  }

  .search:focus-visible {
    outline: 1px solid var(--accent);
    outline-offset: 1px;
  }

  .day {
    color: var(--fg-faint);
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 0.12em;
    margin: 18px 0 2px;
  }

  .btn.small {
    padding: 2.5px 9px;
    font-size: 10.5px;
  }

  .empty {
    color: var(--fg-faint);
    padding: 12px 0;
    margin: 0;
  }

  .history {
    list-style: none;
    margin: 0;
    padding: 0;
  }

  .entry {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 9px 10px;
    margin: 0 -10px;
    border-bottom: 1px solid var(--border);
    border-radius: 6px;
  }

  .entry:hover {
    background: var(--bg-raised);
  }

  .when {
    color: var(--fg-faint);
    flex: none;
    font-variant-numeric: tabular-nums;
  }

  .entry-text {
    flex: 1;
    min-width: 0;
    font-size: 12.5px;
    color: var(--fg-muted);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .entry-actions {
    flex: none;
    display: flex;
    gap: 6px;
    visibility: hidden;
  }

  .entry:hover .entry-actions {
    visibility: visible;
  }

  .btn.del {
    padding: 2.5px 8px;
    line-height: 1;
  }
</style>
