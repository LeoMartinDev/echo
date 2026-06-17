<script lang="ts">
  import { t } from "../../lib/i18n.svelte";
  import type { AppSettings, ModelInfo } from "../../lib/api";
  import { fmtSize, modelDesc } from "../../lib/utils";

  let { models, progress, onsave, onstartdownload, onremovemodel }: {
    models: ModelInfo[];
    progress: Record<string, { pct: number | null; status: string }>;
    onsave?: (patch: Partial<AppSettings>) => void;
    onstartdownload?: (id: string) => void;
    onremovemodel?: (id: string) => void;
  } = $props();
</script>

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
                <button class="font-mono text-[11px] tracking-[0.02em] px-3 py-[5px] rounded-[5px] border border-accent/50 bg-transparent text-accent cursor-pointer transition-[background,border-color] duration-120 ease-[ease] hover:bg-accent-dim focus-visible:outline focus-visible:outline-1 focus-visible:outline-accent focus-visible:outline-offset-1" onclick={() => onsave?.({ model_id: model.id })}>
                  {t("use_model")}
                </button>
                <button class="font-mono text-[11px] tracking-[0.02em] px-3 py-[5px] rounded-[5px] border border-border-strong bg-transparent text-fg-faint cursor-pointer transition-[background,border-color] duration-120 ease-[ease] hover:text-danger hover:border-danger/50 hover:bg-danger/8 focus-visible:outline focus-visible:outline-1 focus-visible:outline-accent focus-visible:outline-offset-1" onclick={() => onremovemodel?.(model.id)}>
                  {t("delete_short")}
                </button>
              {:else}
                <span class="font-mono text-[11px] tracking-[0.02em] text-accent">{t("active")}</span>
              {/if}
            {:else}
              <button class="font-mono text-[11px] tracking-[0.02em] px-3 py-[5px] rounded-[5px] border border-border-strong bg-transparent text-fg cursor-pointer transition-[background,border-color] duration-120 ease-[ease] hover:bg-bg-hover hover:border-white/24 focus-visible:outline focus-visible:outline-1 focus-visible:outline-accent focus-visible:outline-offset-1" onclick={() => onstartdownload?.(model.id)}>
                {t("download")}
              </button>
            {/if}
          {/if}
        </div>
      </li>
    {/each}
  </ul>
</section>
