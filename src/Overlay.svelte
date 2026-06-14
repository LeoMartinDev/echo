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

<div class="pill" class:error={phase === "error"}>
  {#if phase === "error"}
    <span class="error-dot" aria-hidden="true"></span>
  {:else if phase === "recording"}
    <div class="bars" aria-hidden="true">
      {#each bars as b}
        <span class="bar" style="height: {(b * 24).toFixed(1)}px"></span>
      {/each}
    </div>
  {:else}
    <span class="spinner" aria-hidden="true"></span>
  {/if}
</div>

<style>
  :global(html),
  :global(body) {
    background: transparent;
  }

  .pill {
    box-sizing: border-box;
    margin: 4px;
    height: calc(100vh - 8px);
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 100px;
    background: #101114;
    border: 1px solid rgba(255, 255, 255, 0.1);
    overflow: hidden;
    animation: pop 0.16s cubic-bezier(0.2, 0.9, 0.3, 1.2) both;
  }

  @keyframes pop {
    from {
      opacity: 0;
      transform: translateY(6px) scale(0.96);
    }
    to {
      opacity: 1;
      transform: none;
    }
  }

  .pill.error {
    border-color: rgba(207, 122, 109, 0.55);
  }

  .bars {
    display: flex;
    align-items: center;
    gap: 4px;
    height: 26px;
  }

  .bar {
    width: 4.5px;
    min-height: 4.5px;
    border-radius: 999px;
    background: #ffffff;
    transition: height 60ms linear;
  }

  .spinner {
    width: 12px;
    height: 12px;
    border-radius: 50%;
    border: 1.5px solid rgba(255, 255, 255, 0.14);
    border-top-color: #7aa2f7;
    animation: spin 0.8s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  .error-dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: #cf7a6d;
  }

</style>
