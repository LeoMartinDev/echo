<script lang="ts">
  import { onMount } from "svelte";
  import { listen } from "@tauri-apps/api/event";
  import { getSettings, type PhaseEvent } from "./lib/api";
  import { t, setLocale, resolveLocale } from "./lib/i18n.svelte";

  type Phase = PhaseEvent["phase"];

  let phase = $state<Phase>("idle");
  let focusOk = $state(true);

  // Bars animated by mic level, exponential smoothing.
  const BAR_COUNT = 7;
  let bars = $state<number[]>(Array(BAR_COUNT).fill(0));
  let level = 0;
  let smoothed = 0;

  onMount(() => {
    const unsubs: Array<() => void> = [];
    let raf = 0;

    // Align the tooltip language with the interface setting.
    getSettings()
      .then((s) => setLocale(resolveLocale(s.ui_language)))
      .catch(() => {});

    listen<PhaseEvent>("echo://phase", (e) => {
      phase = e.payload.phase;
    }).then((u) => unsubs.push(u));

    listen<number>("echo://level", (e) => {
      level = e.payload;
    }).then((u) => unsubs.push(u));

    listen<boolean>("echo://focus", (e) => {
      focusOk = e.payload;
    }).then((u) => unsubs.push(u));

    const animate = () => {
      smoothed += (level - smoothed) * 0.3;
      level *= 0.92; // decays gently between events
      // Near-linear response (bars track the actual voice dynamics),
      // simply scaled up: noise gate then strong gain,
      // with barely perceptible compression for headroom.
      const gated = Math.max(0, smoothed - 0.004);
      const amplified = Math.min(1, Math.pow(gated * 28, 0.85));
      const center = (BAR_COUNT - 1) / 2;
      bars = bars.map((_, i) => {
        const dist = Math.abs(i - center);
        const falloff = 1 - dist / (center + 4.5);
        // Symmetric standing wave: the phase depends on the distance to
        // the center, so neighbours move together and both halves mirror —
        // a gentle breathing pulse rather than independent flicker.
        const jitter = 0.82 + 0.18 * Math.sin(performance.now() / 260 - dist * 0.9);
        return Math.max(0.14, amplified * falloff * jitter);
      });
      raf = requestAnimationFrame(animate);
    };
    raf = requestAnimationFrame(animate);

    return () => {
      cancelAnimationFrame(raf);
      unsubs.forEach((u) => u());
    };
  });
</script>

<div class="box-border m-1 h-[calc(100vh-8px)] flex items-center justify-center rounded-[100px] bg-[#101114] border border-white/10 overflow-hidden animate-[pop_0.16s_cubic-bezier(0.2,0.9,0.3,1.2)_both] {phase === 'error' ? '!border-danger/55' : ''}">
  {#if phase === "error"}
    <span class="w-[7px] h-[7px] rounded-full bg-danger" aria-hidden="true"></span>
  {:else if phase === "recording"}
    <div class="flex items-center gap-1 h-[26px]" aria-hidden="true">
      {#each bars as b}
        <span class="w-[4.5px] min-h-[4.5px] rounded-full bg-white transition-[height] duration-[60ms] linear" style="height: {(b * 24).toFixed(1)}px"></span>
      {/each}
    </div>
  {:else}
    <span class="w-3 h-3 rounded-full border-[1.5px] border-white/14 border-t-accent animate-[spin_0.8s_linear_infinite]" aria-hidden="true"></span>
  {/if}
</div>
