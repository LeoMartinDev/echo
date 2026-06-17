<script lang="ts">
  import { t } from "../lib/i18n.svelte";

  let { message, onclose }: {
    message: string;
    onclose?: () => void;
  } = $props();
</script>

<div class="flex items-center justify-between gap-[14px] px-[14px] py-2.5 mb-[18px] border border-border border-l-2 border-l-danger rounded-md bg-bg-raised text-fg" role="alert">
  <span>{message}</span>
  <button class="bg-transparent border-none text-fg-faint cursor-pointer text-[11px] p-1 hover:text-fg" onclick={() => onclose?.()} aria-label={t("close")}>✕</button>
</div>
