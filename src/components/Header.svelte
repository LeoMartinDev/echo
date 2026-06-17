<script lang="ts">
  import type { ModelInfo } from "../lib/api";
  import { t } from "../lib/i18n.svelte";
  import { fmtRam } from "../lib/utils";

  let { activeModel, memBytes }: {
    activeModel?: ModelInfo;
    memBytes: number | null;
  } = $props();
</script>

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
