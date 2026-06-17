<script lang="ts">
  import { t } from "../../lib/i18n.svelte";
  import type { AppSettings, HistoryEntry } from "../../lib/api";
  import { fmtTime, dayLabel } from "../../lib/utils";
  import ToggleGroup from "../shared/ToggleGroup.svelte";

  let { settings, history, query, copiedTs, onsave, onquerychange, onwipe, oncopy, ondelete }: {
    settings: AppSettings;
    history: HistoryEntry[];
    query: string;
    copiedTs: number | null;
    onsave?: (patch: Partial<AppSettings>) => void;
    onquerychange?: (q: string) => void;
    onwipe?: () => void;
    oncopy?: (entry: HistoryEntry) => void;
    ondelete?: (entry: HistoryEntry) => void;
  } = $props();

  const filtered = $derived(
    query.trim()
      ? history.filter((e) =>
          e.text.toLowerCase().includes(query.trim().toLowerCase()),
        )
      : history,
  );

  const grouped = $derived.by(() => {
    const groups: Array<{ label: string; entries: HistoryEntry[] }> = [];
    for (const e of filtered) {
      const label = dayLabel(e.ts_ms);
      const last = groups[groups.length - 1];
      if (last && last.label === label) last.entries.push(e);
      else groups.push({ label, entries: [e] });
    }
    return groups;
  });
</script>

<section class="animate-[rise_0.45s_cubic-bezier(0.2,0.7,0.2,1)_both]" style="animation-delay: calc(var(--d) * 55ms); --d: 3">
  <div class="flex items-center justify-between gap-6 py-[14px] border-b border-border">
    <div class="flex flex-col gap-0.5 text-fg">
      {t("keep_label")}
      <span class="text-[11.5px] text-fg-faint">{t("keep_hint")}</span>
    </div>
    <div class="shrink-0">
      <ToggleGroup
        options={[
          { value: true as const, label: t("on") },
          { value: false as const, label: t("off") },
        ]}
        value={settings.history_enabled}
        ariaLabel={t("keep_label")}
        onchange={(v) => onsave?.({ history_enabled: v })}
      />
    </div>
  </div>

  {#if history.length > 0}
    <div class="flex gap-2 items-center py-[14px] pb-1">
      <input
        class="flex-1 font-mono text-[11px] tracking-[0.02em] px-[10px] py-[5px] rounded-[5px] border border-border-strong bg-transparent text-fg placeholder:text-fg-faint focus-visible:outline focus-visible:outline-1 focus-visible:outline-accent focus-visible:outline-offset-1"
        type="text"
        placeholder={t("filter_placeholder")}
        value={query}
        oninput={(e) => onquerychange?.(e.currentTarget.value)}
      />
      <button class="font-mono text-[11px] tracking-[0.02em] px-[9px] py-[2.5px] rounded-[5px] border border-border-strong bg-transparent text-fg-faint cursor-pointer transition-[background,border-color] duration-120 ease-[ease] hover:text-danger hover:border-danger/50 hover:bg-danger/8 focus-visible:outline focus-visible:outline-1 focus-visible:outline-accent focus-visible:outline-offset-1" onclick={() => onwipe?.()}>{t("clear_all")}</button>
    </div>
  {/if}

  {#if filtered.length === 0}
    <p class="font-mono text-[11px] tracking-[0.02em] text-fg-faint py-3 m-0">
      {history.length === 0 ? t("empty_none") : t("empty_no_results")}
    </p>
  {:else}
    {#each grouped as group (group.label)}
      <div class="font-mono text-[11px] tracking-[0.02em] text-fg-faint text-[10px] uppercase tracking-[0.12em] mt-[18px] mb-0.5">{group.label}</div>
      <ul class="list-none m-0 p-0">
        {#each group.entries as entry (entry.ts_ms)}
          <li class="group flex items-center gap-3 px-[10px] py-[9px] -mx-[10px] border-b border-border rounded-md hover:bg-bg-raised">
            <span class="font-mono text-[11px] tracking-[0.02em] text-fg-faint shrink-0 tabular-nums">{fmtTime(entry.ts_ms)}</span>
            <span class="flex-1 min-w-0 text-[12.5px] text-fg-muted truncate" title={entry.text}>{entry.text}</span>
            <div class="shrink-0 flex gap-[6px] invisible group-hover:visible">
              <button class="font-mono text-[11px] tracking-[0.02em] px-[9px] py-[2.5px] rounded-[5px] border border-border-strong bg-transparent text-fg cursor-pointer transition-[background,border-color] duration-120 ease-[ease] hover:bg-bg-hover hover:border-white/24 focus-visible:outline focus-visible:outline-1 focus-visible:outline-accent focus-visible:outline-offset-1" onclick={() => oncopy?.(entry)}>
                {copiedTs === entry.ts_ms ? t("copied") : t("copy")}
              </button>
              <button
                class="font-mono text-[11px] tracking-[0.02em] px-2 py-[2.5px] leading-none rounded-[5px] border border-border-strong bg-transparent text-fg-faint cursor-pointer transition-[background,border-color] duration-120 ease-[ease] hover:text-danger hover:border-danger/50 hover:bg-danger/8 focus-visible:outline focus-visible:outline-1 focus-visible:outline-accent focus-visible:outline-offset-1"
                title={t("delete_entry")}
                aria-label={t("delete_entry")}
                onclick={() => ondelete?.(entry)}>✕</button
              >
            </div>
          </li>
        {/each}
      </ul>
    {/each}
  {/if}
</section>
