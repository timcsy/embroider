<script lang="ts">
  // 在縮圖上用手指（或滑鼠）拖出一個框，框外的部分不繡。
  import { rectFromDrag, type CropRect, type Pt } from '../lib/crop'
  import { strings } from '../lib/strings.zh-TW'

  let {
    src,
    alt,
    rect,
    onchange,
  }: { src: string; alt: string; rect: CropRect | null; onchange: (r: CropRect | null) => void } = $props()

  let box = $state<HTMLDivElement>()
  let start = $state<Pt | null>(null)
  let live = $state<CropRect | null>(null)

  const at = (e: PointerEvent): Pt => {
    const b = box!.getBoundingClientRect()
    return { x: (e.clientX - b.left) / b.width, y: (e.clientY - b.top) / b.height }
  }
  function down(e: PointerEvent) {
    box!.setPointerCapture(e.pointerId)
    start = at(e)
    live = null
  }
  function move(e: PointerEvent) {
    if (!start) return
    const p = at(e)
    live = {
      x0: Math.max(0, Math.min(start.x, p.x)),
      y0: Math.max(0, Math.min(start.y, p.y)),
      x1: Math.min(1, Math.max(start.x, p.x)),
      y1: Math.min(1, Math.max(start.y, p.y)),
    }
  }
  function up(e: PointerEvent) {
    if (!start) return
    const r = rectFromDrag(start, at(e))
    start = null
    live = null
    // 只是點一下（框太小）時不動原本的範圍
    if (r) onchange(r)
  }

  const shown = $derived(live ?? rect)
  const pct = (v: number) => `${(v * 100).toFixed(2)}%`
</script>

<div class="wrap">
  <div
    class="box"
    bind:this={box}
    role="application"
    aria-label={strings.image.cropHint}
    data-testid="crop-area"
    onpointerdown={down}
    onpointermove={move}
    onpointerup={up}
    onpointercancel={() => ((start = null), (live = null))}
  >
    <img {src} {alt} draggable="false" />
    {#if shown}
      <div
        class="frame"
        data-testid="crop-frame"
        style:left={pct(shown.x0)}
        style:top={pct(shown.y0)}
        style:width={pct(shown.x1 - shown.x0)}
        style:height={pct(shown.y1 - shown.y0)}
      ></div>
    {/if}
  </div>
  <p class="muted">{rect ? strings.image.cropOn : strings.image.cropHint}</p>
  {#if rect}
    <button class="secondary" onclick={() => onchange(null)} data-testid="crop-reset">{strings.image.cropAll}</button>
  {/if}
</div>

<style>
  .wrap { display: grid; justify-items: center; gap: 6px; }
  .box { position: relative; display: inline-block; touch-action: none; overflow: hidden; border-radius: 10px; cursor: crosshair; line-height: 0; }
  img { display: block; max-width: 100%; max-height: 260px; user-select: none; -webkit-user-drag: none; }
  .frame { position: absolute; border: 3px solid var(--accent, #b3282d); box-shadow: 0 0 0 9999px rgba(0, 0, 0, 0.45); box-sizing: border-box; pointer-events: none; }
  .muted { margin: 0; color: var(--muted); font-size: 0.92em; text-align: center; }
  .secondary { min-height: 48px; padding: 0 18px; border-radius: 12px; border: 2px solid var(--line); background: #fff; font-size: 1em; font-weight: 700; }
</style>
