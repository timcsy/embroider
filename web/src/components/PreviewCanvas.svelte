<script lang="ts">
  import { drawPreview } from '../lib/draw'
  import type { Hoop, StitchPlan } from '../lib/types'

  let { plan, hoop, label }: { plan: StitchPlan | null; hoop: Hoop; label: string } = $props()

  let canvas = $state<HTMLCanvasElement>()
  let width = $state(340)

  const height = $derived(Math.round(Math.min(width * (hoop.height / hoop.width), width * 1.1)))

  $effect(() => {
    if (!canvas) return
    const dpr = window.devicePixelRatio || 1
    canvas.width = Math.round(width * dpr)
    canvas.height = Math.round(height * dpr)
    const ctx = canvas.getContext('2d')
    if (ctx) drawPreview(ctx, plan, hoop, width, height, dpr)
  })
</script>

<div class="wrap" bind:clientWidth={width} role="img" aria-label={label}>
  <canvas bind:this={canvas} style="width:{width}px;height:{height}px" data-testid="preview"></canvas>
</div>

<style>
  .wrap { width: 100%; }
  canvas { display: block; border-radius: 12px; }
</style>
