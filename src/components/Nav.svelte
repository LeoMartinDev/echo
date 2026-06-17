<script lang="ts">
  import { t } from "../lib/i18n.svelte";

  let { view, historyCount, onchange }: {
    view: "settings" | "history";
    historyCount: number;
    onchange?: (view: "settings" | "history") => void;
  } = $props();
</script>

<nav class="flex gap-5 mt-[22px] border-b border-border animate-[rise_0.45s_cubic-bezier(0.2,0.7,0.2,1)_both]" style="animation-delay: calc(var(--d) * 55ms); --d: 2">
  <button
    class="font-mono text-[11px] tracking-[0.04em] px-px pt-[7px] pb-[9px] bg-transparent border-none border-b -mb-px cursor-pointer transition-[color] duration-120 ease-[ease] {view === 'settings' ? 'text-fg border-b-fg' : 'border-b-transparent text-fg-faint hover:text-fg-muted'}"
    onclick={() => onchange?.("settings")}
  >
    {t("tab_settings")}
  </button>
  <button
    class="font-mono text-[11px] tracking-[0.04em] px-px pt-[7px] pb-[9px] bg-transparent border-none border-b -mb-px cursor-pointer transition-[color] duration-120 ease-[ease] {view === 'history' ? 'text-fg border-b-fg' : 'border-b-transparent text-fg-faint hover:text-fg-muted'}"
    onclick={() => onchange?.("history")}
  >
    {t("tab_history")}{#if historyCount > 0}<span class="ml-[6px] text-fg-faint tabular-nums">{historyCount}</span>{/if}
  </button>
</nav>
