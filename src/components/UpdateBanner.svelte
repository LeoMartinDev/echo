<script lang="ts">
  import { t } from "../lib/i18n.svelte";

  let { version, oninstall }: {
    version: string;
    oninstall?: () => void;
  } = $props();
</script>

<div class="flex items-center justify-between gap-[14px] px-[14px] py-2.5 mb-[18px] border border-border border-l-2 border-l-accent rounded-md bg-bg-raised text-fg-muted">
  <span>{t("update_available", { version })}</span>
  <button class="font-mono text-[11px] tracking-[0.02em] px-3 py-[5px] rounded-[5px] border border-accent/50 bg-transparent text-accent cursor-pointer transition-[background,border-color] duration-120 ease-[ease] hover:bg-accent-dim focus-visible:outline focus-visible:outline-1 focus-visible:outline-accent focus-visible:outline-offset-1" onclick={() => oninstall?.()}>{t("update_restart")}</button>
</div>
