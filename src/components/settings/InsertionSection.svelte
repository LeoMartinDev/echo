<script lang="ts">
  import { t } from "../../lib/i18n.svelte";
  import { LANGUAGES, type AppSettings } from "../../lib/api";
  import ToggleGroup from "../shared/ToggleGroup.svelte";

  let { settings, onsave }: {
    settings: AppSettings;
    onsave?: (patch: Partial<AppSettings>) => void;
  } = $props();
</script>

<section class="mt-[34px] animate-[rise_0.45s_cubic-bezier(0.2,0.7,0.2,1)_both]" style="animation-delay: calc(var(--d) * 55ms); --d: 4">
  <h2 class="font-mono text-[10.5px] font-medium uppercase tracking-[0.14em] text-fg-faint m-0 mb-1 pb-2 border-b border-border">{t("sec_insertion")}</h2>
  <div class="flex items-center justify-between gap-6 py-[14px] border-b border-border">
    <div class="flex flex-col gap-0.5 text-fg">
      {t("write_mode")}
      <span class="text-[11.5px] text-fg-faint">
        {t("write_mode_hint")}
      </span>
    </div>
    <div class="shrink-0">
      <ToggleGroup
        options={[
          { value: "live" as const, label: t("mode_live") },
          { value: "on_release" as const, label: t("mode_on_release") },
        ]}
        value={settings.insertion_mode}
        ariaLabel={t("write_mode")}
        onchange={(v) => onsave?.({ insertion_mode: v })}
      />
    </div>
  </div>
  <div class="flex items-center justify-between gap-6 py-[14px] border-b border-border">
    <div class="flex flex-col gap-0.5 text-fg">
      {t("lang_label")}
      <span class="text-[11.5px] text-fg-faint">{t("lang_hint")}</span>
    </div>
    <div class="shrink-0">
      <select
        value={settings.language ?? ""}
        onchange={(e) => onsave?.({ language: e.currentTarget.value || null })}
        class="font-mono text-[11px] pr-[28px] pl-[10px] py-[5px] rounded-[5px] border border-border-strong bg-bg text-fg appearance-none cursor-pointer hover:bg-bg-hover focus-visible:outline focus-visible:outline-1 focus-visible:outline-accent focus-visible:outline-offset-1"
        style="background-image: url(&quot;data:image/svg+xml;charset=utf-8,%3Csvg xmlns='http://www.w3.org/2000/svg' width='8' height='5'%3E%3Cpath d='M1 1l3 3 3-3' stroke='%2397978f' fill='none'/%3E%3C/svg%3E&quot;); background-repeat: no-repeat; background-position: right 9px center"
      >
        {#each LANGUAGES as lang}
          <option value={lang.code ?? ""} class="text-fg bg-bg-raised">{t(lang.key)}</option>
        {/each}
      </select>
    </div>
  </div>
</section>
