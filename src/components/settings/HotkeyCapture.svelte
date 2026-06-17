<script lang="ts">
  import { t } from "../../lib/i18n.svelte";
  import type { AppSettings } from "../../lib/api";

  let { settings, capturing, onstartcapture, onendcapture }: {
    settings: AppSettings;
    capturing: boolean;
    onstartcapture?: () => void;
    onendcapture?: () => void;
  } = $props();
</script>

<section class="mt-[30px] animate-[rise_0.45s_cubic-bezier(0.2,0.7,0.2,1)_both]" style="animation-delay: calc(var(--d) * 55ms); --d: 3">
  <button
    type="button"
    class="group relative flex flex-col items-center gap-[18px] w-full px-6 pt-[34px] pb-[26px] border rounded-xl bg-bg-raised cursor-pointer transition-[border-color,background] duration-150 ease-[ease] {capturing ? 'border-accent bg-accent-dim' : 'border-border hover:border-border-strong hover:bg-bg-hover'} focus-visible:outline focus-visible:outline-1 focus-visible:outline-accent focus-visible:outline-offset-2"
    onclick={() => (capturing ? onendcapture?.() : onstartcapture?.())}
    aria-label={t("ptt_label")}
  >
    {#if !capturing}
      <span class="absolute top-3 right-[14px] font-mono text-[10px] tracking-[0.04em] text-fg-faint opacity-0 transition-opacity duration-150 ease-[ease] group-hover:opacity-100">{t("ptt_change")}</span>
    {/if}
    <div class="flex items-center gap-[10px] min-h-14">
      {#if capturing}
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
