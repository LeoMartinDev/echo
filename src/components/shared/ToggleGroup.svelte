<script lang="ts" generics="V extends string | boolean">
  type Option = { value: V; label: string };

  let { options, value, ariaLabel, onchange }: {
    options: Option[];
    value: V;
    ariaLabel: string;
    onchange?: (value: V) => void;
  } = $props();
</script>

<div class="inline-flex border border-border-strong rounded-[5px] overflow-hidden" role="radiogroup" aria-label={ariaLabel}>
  {#each options as opt, i}
    <button
      class="font-mono text-[11px] px-3 py-[5px] cursor-pointer transition-[background] duration-120 ease-[ease] hover:bg-bg-hover focus-visible:outline focus-visible:outline-1 focus-visible:outline-accent focus-visible:outline-offset-1 {value === opt.value ? 'bg-accent-dim text-accent' : 'text-fg-muted'} {i < options.length - 1 ? 'border-r border-border' : ''}"
      onclick={() => onchange?.(opt.value)}
      role="radio"
      aria-checked={value === opt.value}
    >
      {opt.label}
    </button>
  {/each}
</div>
