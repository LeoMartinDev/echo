<script lang="ts">
  import { t } from "../lib/i18n.svelte";

  let { onauthorize }: {
    onauthorize?: () => void;
  } = $props();
</script>

<div class="flex items-center justify-between gap-[14px] px-[14px] py-2.5 mb-[18px] border border-border border-l-2 border-l-accent rounded-md bg-bg-raised text-fg-muted">
  <span>
    {t("a11y_before")} <strong class="text-fg font-semibold">{t("a11y_perm")}</strong> {t("a11y_after")}
  </span>
  <button
    class="font-mono text-[11px] tracking-[0.02em] px-3 py-[5px] rounded-[5px] border border-border-strong bg-transparent text-fg cursor-pointer transition-[background,border-color] duration-120 ease-[ease] hover:bg-bg-hover hover:border-white/24 focus-visible:outline focus-visible:outline-1 focus-visible:outline-accent focus-visible:outline-offset-1"
    onclick={() => onauthorize?.()}
  >{t("authorize")}</button
  >
</div>
