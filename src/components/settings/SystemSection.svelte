<script lang="ts">
  import { t, SUPPORTED, type TKey } from "../../lib/i18n.svelte";
  import type { AppSettings } from "../../lib/api";
  import ToggleGroup from "../shared/ToggleGroup.svelte";

  let { settings, onsave }: {
    settings: AppSettings;
    onsave?: (patch: Partial<AppSettings>) => void;
  } = $props();
</script>

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
        onchange={(e) => onsave?.({ ui_language: e.currentTarget.value || null })}
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
      <ToggleGroup
        options={[
          { value: true as const, label: t("on") },
          { value: false as const, label: t("off") },
        ]}
        value={settings.autostart}
        ariaLabel={t("autostart_label")}
        onchange={(v) => onsave?.({ autostart: v })}
      />
    </div>
  </div>
</section>
