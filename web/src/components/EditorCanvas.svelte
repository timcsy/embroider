<script lang="ts">
  // 編輯畫面：下層是繡出來的樣子，上層是可以用手指（或滑鼠）操作的外框。
  // 一根手指拖移物件、拉角改大小、拉上方的圓點轉方向；兩根手指同時縮放與旋轉；
  // 滑鼠滾輪縮放、方向鍵微調。
  import { drawPreview } from '../lib/draw'
  import type { EditorItem } from '../lib/editor'
  import {
    fromCanvas,
    describeGesture,
    hitBox,
    itemBox,
    applyZoom,
    clampZoom,
    MAX_ZOOM,
    pinch,
    pinchZoom,
    resizeBy,
    rotateTo,
    rotationHandle,
    snapAngle,
    snapMove,
    topHit,
    zoomAround,
    type Box,
    type Pt,
    type Zoom,
  } from '../lib/geometry'
  import { computeView } from '../lib/preview'
  import { strings } from '../lib/strings.zh-TW'
  import type { Hoop, ItemInfo, StitchPlan } from '../lib/types'

  let {
    plan,
    hoop,
    items,
    infos,
    fit,
    selectedId,
    label,
    fill = false,
    drawing = false,
    brush = { css: '#b3282d', units: 25 },
    onSelect,
    onChange,
    onRemove,
    onStroke = () => {},
  }: {
    plan: StitchPlan | null
    hoop: Hoop
    items: EditorItem[]
    infos: ItemInfo[]
    /** 為了放進框而整體縮小的倍率 */
    fit: number
    selectedId: string | null
    label: string
    onSelect: (id: string | null) => void
    onChange: (id: string, patch: Partial<EditorItem>, key: string) => void
    onRemove: (id: string) => void
    /** 畫畫模式：手指畫完一筆時呼叫，點是框座標（已套用整體縮小） */
    /** 寬螢幕（畫布在左、工具在右）：畫布盡量放大；窄螢幕要留位置給下面的工具 */
    fill?: boolean
    drawing?: boolean
    brush?: { css: string; units: number }
    onStroke?: (points: Pt[]) => void
  } = $props()

  let area = $state<HTMLDivElement>()
  let avail = $state(340)
  let canvas = $state<HTMLCanvasElement>()
  let viewportH = $state(typeof window === 'undefined' ? 800 : window.innerHeight)

  // 畫布大小：寬度盡量用滿，但高度不超過畫面的三成（矮螢幕）、四成（手機直立，下面要留位置給工具）到八成（電腦）
  const maxH = $derived(
    fill ? Math.max(320, viewportH - 190) : viewportH * (viewportH < 700 ? 0.3 : 0.4),
  )
  const ratio = $derived(hoop.width / hoop.height)
  const width = $derived(Math.max(200, Math.min(avail, maxH * ratio)))
  const height = $derived(Math.round(width / ratio))
  const baseView = $derived(computeView(hoop, width, height, 14))

  // 放大與平移：繡框很小、細節很小，要能放大才好精準擺放
  let zm = $state<Zoom>({ z: 1, px: 0, py: 0 })
  const view = $derived(applyZoom(baseView, zm))
  const setZoom = (next: Zoom) => (zm = clampZoom(baseView, next, width, height, hoop.width, hoop.height))
  const zoomBy = (factor: number, at?: Pt) =>
    setZoom(zoomAround(baseView, zm, at ?? { x: width / 2, y: height / 2 }, zm.z * factor))
  const zoomFit = () => (zm = { z: 1, px: 0, py: 0 })
  // 換框或畫布大小變了：回到看全部
  $effect(() => {
    void hoop.id
    void width
    zm = { z: 1, px: 0, py: 0 }
  })

  $effect(() => {
    if (!canvas) return
    const dpr = window.devicePixelRatio || 1
    canvas.width = Math.round(width * dpr)
    canvas.height = Math.round(height * dpr)
    const ctx = canvas.getContext('2d')
    if (ctx) drawPreview(ctx, plan, hoop, width, height, dpr, undefined, view)
  })

  const HANDLE_R = 11 // 看得到的把手半徑（像素）
  const HANDLE_HIT = 26 // 手指實際可以按到的範圍
  const ROTATE_GAP = 30 // 旋轉把手離物件上緣的距離（像素）
  const PAD_PX = 10 // 點選物件時多留的容許範圍

  /** 沒有核心回報的大小時（還沒算好、物件有問題）先用輸入的大小估一個。 */
  function fallbackSize(it: EditorItem): { width: number; height: number } {
    if (it.kind === 'text') return { width: Math.max(300, it.heightCm * 100 * 3), height: it.heightCm * 100 }
    if (it.kind === 'draw') {
      const all = it.strokes.flat()
      const xs = all.map((p) => p[0])
      const ys = all.map((p) => p[1])
      const pad = it.widthMm * 10
      return {
        width: Math.max(...xs) - Math.min(...xs) + pad,
        height: Math.max(...ys) - Math.min(...ys) + pad,
      }
    }
    if (it.kind === 'shape') {
      const lineH = it.shape === 'line' ? it.lineWidthMm * 10 : it.heightCm * 100
      return { width: it.widthCm * 100, height: Math.max(lineH, 40) }
    }
    return { width: it.sizeCm * 100, height: it.sizeCm * 100 }
  }

  function boxOf(it: EditorItem): Box {
    const info = infos.find((i) => i.id === it.id)
    // 核心回報的大小是「當時的縮放」下的；使用者正在拖大小時，先用比例估，算好後再對準
    const size =
      info && info.width > 0 && info.scale > 0
        ? { width: (info.width / info.scale) * it.scale, height: (info.height / info.scale) * it.scale }
        : (() => {
            const f = fallbackSize(it)
            return { width: f.width * it.scale, height: f.height * it.scale }
          })()
    return itemBox(it, size, fit)
  }

  const boxes = $derived(items.map(boxOf))
  const selected = $derived(items.find((i) => i.id === selectedId) ?? null)
  const selectedBox = $derived(selected ? boxOf(selected) : null)

  const px = (b: Box) => ({ x: view.ox + b.cx * view.scale, y: view.oy + b.cy * view.scale })
  const corners = (b: Box) =>
    [
      [-1, -1],
      [1, -1],
      [1, 1],
      [-1, 1],
    ].map(([sx, sy]) => ({ sx, sy }))

  function cornerPx(b: Box, sx: number, sy: number): Pt {
    const r = (b.rot * Math.PI) / 180
    const lx = (sx * b.w * view.scale) / 2
    const ly = (sy * b.h * view.scale) / 2
    const c = px(b)
    return { x: c.x + lx * Math.cos(r) - ly * Math.sin(r), y: c.y + lx * Math.sin(r) + ly * Math.cos(r) }
  }

  function rotatePx(b: Box): Pt {
    const h = rotationHandle(b, ROTATE_GAP / view.scale)
    return { x: view.ox + h.x * view.scale, y: view.oy + h.y * view.scale }
  }

  type Drag =
    | { mode: 'move'; id: string; start: Pt; x: number; y: number }
    | { mode: 'resize'; id: string; scale: number; from: Pt }
    | { mode: 'rotate'; id: string }
    | { mode: 'pan'; start: Pt; zm: Zoom; moved: boolean }
    | null

  let drag = null as Drag
  const pointers = new Map<number, Pt>()
  // 兩根手指落在空白處：放大縮小與平移整個畫布（落在選到的物件上才是縮放物件）
  let canvasPinch: { zm: Zoom; a: [Pt, Pt] } | null = null
  let pinchStart: { id: string; scale: number; rotation: number; a: [Pt, Pt] } | null = null
  // 對齊參考線（框座標）：吸到什麼，就在那裡畫一條線
  let guides = $state<{ xs: number[]; ys: number[] }>({ xs: [], ys: [] })
  // 拖動時顯示的數字（位置、大小、角度）
  let gesture = $state<'move' | 'resize' | 'rotate' | null>(null)
  // 碰到鎖住的東西時的提示（幾秒後消失）
  let hint = $state('')
  let hintTimer: ReturnType<typeof setTimeout> | undefined
  function showHint(text: string) {
    hint = text
    clearTimeout(hintTimer)
    hintTimer = setTimeout(() => (hint = ''), 2200)
  }
  const readout = $derived(hint || (gesture && selectedBox ? describeGesture(gesture, selectedBox) : ''))

  const local = (e: PointerEvent): Pt => {
    const r = (e.currentTarget as Element).getBoundingClientRect()
    return { x: e.clientX - r.left, y: e.clientY - r.top }
  }
  const dist = (a: Pt, b: Pt) => Math.hypot(a.x - b.x, a.y - b.y)

  // 畫畫模式：正在畫的這一筆（畫布像素）
  let live = $state<Pt[]>([])
  const MIN_STEP_PX = 2

  function strokeDown(e: PointerEvent) {
    ;(e.currentTarget as Element).setPointerCapture?.(e.pointerId)
    live = [local(e)]
  }
  function strokeMove(e: PointerEvent) {
    if (live.length === 0) return
    const p = local(e)
    const last = live[live.length - 1]
    if (Math.hypot(p.x - last.x, p.y - last.y) >= MIN_STEP_PX) live = [...live, p]
  }
  function strokeUp() {
    if (live.length === 0) return
    const pts = live.map((p) => fromCanvas(view, p.x, p.y))
    live = []
    onStroke(pts)
  }

  function onDown(e: PointerEvent) {
    if (drawing) return strokeDown(e)
    const cp = local(e)
    ;(e.currentTarget as Element).setPointerCapture?.(e.pointerId)
    pointers.set(e.pointerId, cp)
    // 取得鍵盤焦點，但不要讓頁面跳動（手指一碰畫布頁面就捲走會很糟）
    ;(e.currentTarget as HTMLElement).focus?.({ preventScroll: true })

    if (pointers.size === 2) {
      const [a, b] = [...pointers.values()]
      const mid = fromCanvas(view, (a.x + b.x) / 2, (a.y + b.y) / 2)
      if (selected && selectedBox && hitBox(selectedBox, mid, PAD_PX / view.scale)) {
        pinchStart = { id: selected.id, scale: selected.scale, rotation: selected.rotation, a: [a, b] }
      } else {
        canvasPinch = { zm: { ...zm }, a: [a, b] }
      }
      drag = null
      return
    }
    // 滑鼠中鍵：在哪裡都可以平移
    if ((e as PointerEvent).button === 1) {
      drag = { mode: 'pan', start: cp, zm: { ...zm }, moved: true }
      return
    }
    if (selected && selectedBox && !selected.locked) {
      const rp = rotatePx(selectedBox)
      if (dist(cp, rp) <= HANDLE_HIT) {
        drag = { mode: 'rotate', id: selected.id }
        gesture = 'rotate'
        return
      }
      for (const { sx, sy } of corners(selectedBox)) {
        if (dist(cp, cornerPx(selectedBox, sx, sy)) <= HANDLE_HIT) {
          drag = { mode: 'resize', id: selected.id, scale: selected.scale, from: fromCanvas(view, cp.x, cp.y) }
          gesture = 'resize'
          return
        }
      }
    }
    const p = fromCanvas(view, cp.x, cp.y)
    const hit = topHit(boxes, p, PAD_PX / view.scale)
    if (hit >= 0) {
      const it = items[hit]
      if (it.id !== selectedId) onSelect(it.id)
      if (it.locked) {
        drag = null
        showHint(strings.editor.lockedHint)
        return
      }
      drag = { mode: 'move', id: it.id, start: cp, x: it.x, y: it.y }
      gesture = null // 真的動了才顯示
    } else if (zm.z > 1) {
      // 放大後，拖空白處是平移；只是點一下才取消選取
      drag = { mode: 'pan', start: cp, zm: { ...zm }, moved: false }
    } else {
      drag = null
      onSelect(null)
    }
  }

  // 滑鼠移到不同地方，游標跟著變：把手可以拉、物件可以拖
  let cursor = $state('default')
  function hover(cp: Pt) {
    if (drawing) return
    if (selected && selectedBox) {
      if (dist(cp, rotatePx(selectedBox)) <= HANDLE_HIT) return (cursor = 'grab')
      for (const { sx, sy } of corners(selectedBox)) {
        if (dist(cp, cornerPx(selectedBox, sx, sy)) <= HANDLE_HIT) return (cursor = 'nwse-resize')
      }
    }
    const p = fromCanvas(view, cp.x, cp.y)
    cursor = topHit(boxes, p, PAD_PX / view.scale) >= 0 ? 'move' : 'default'
  }

  function onMove(e: PointerEvent) {
    if (drawing) return strokeMove(e)
    if (!pointers.has(e.pointerId)) return hover(local(e))
    const cp = local(e)
    pointers.set(e.pointerId, cp)
    if (canvasPinch && pointers.size >= 2) {
      const [a, b] = [...pointers.values()]
      setZoom(pinchZoom(baseView, canvasPinch.zm, canvasPinch.a, [a, b]))
      return
    }
    if (pinchStart && pointers.size >= 2) {
      const [a, b] = [...pointers.values()]
      const r = pinch(pinchStart, pinchStart.a, [a, b])
      gesture = 'resize'
      onChange(pinchStart.id, { scale: r.scale, rotation: snapAngle(r.rotation) }, `pinch:${pinchStart.id}`)
      return
    }
    if (!drag) return
    if (drag.mode === 'pan') {
      const dx = cp.x - drag.start.x
      const dy = cp.y - drag.start.y
      if (Math.hypot(dx, dy) > 4) drag.moved = true
      if (drag.moved) setZoom({ z: drag.zm.z, px: drag.zm.px + dx, py: drag.zm.py + dy })
      return
    }
    const dragId = drag.id
    const it = items.find((i) => i.id === dragId)
    if (!it) return
    if (drag.mode === 'move') {
      const dx = (cp.x - drag.start.x) / view.scale / fit
      const dy = (cp.y - drag.start.y) / view.scale / fit
      // 想要放的位置（設計座標）→ 畫面座標的外框 → 吸附到別的物件與繡框的線
      const cand = { ...it, x: drag.x + dx, y: drag.y + dy }
      const mine = boxOf(cand)
      const group = it.groupId
      const others = items.filter((o) => o.id !== it.id && !(group && o.groupId === group)).map(boxOf)
      const safe = { w: hoop.width - 2 * hoop.margin, h: hoop.height - 2 * hoop.margin }
      const s = snapMove(mine, others, safe, 8 / view.scale)
      guides = { xs: s.xs, ys: s.ys }
      const nx = drag.x + dx + s.dx / fit
      const ny = drag.y + dy + s.dy / fit
      gesture = 'move'
      onChange(it.id, { x: Math.round(nx), y: Math.round(ny) }, `move:${it.id}`)
    } else if (drag.mode === 'resize' && selectedBox) {
      const p = fromCanvas(view, cp.x, cp.y)
      onChange(it.id, { scale: resizeBy(drag.scale, drag.from, p, { x: selectedBox.cx, y: selectedBox.cy }) }, `scale:${it.id}`)
    } else if (drag.mode === 'rotate' && selectedBox) {
      const p = fromCanvas(view, cp.x, cp.y)
      onChange(it.id, { rotation: snapAngle(rotateTo({ x: selectedBox.cx, y: selectedBox.cy }, p)) }, `rotate:${it.id}`)
    }
  }

  function onUp(e: PointerEvent) {
    if (drawing) return strokeUp()
    pointers.delete(e.pointerId)
    if (pointers.size < 2) {
      pinchStart = null
      canvasPinch = null
    }
    if (pointers.size === 0) {
      if (drag?.mode === 'pan' && !drag.moved) onSelect(null)
      drag = null
      gesture = null
      guides = { xs: [], ys: [] }
    }
  }

  // 滾輪（或觸控板的兩指撐開）：以游標為中心放大縮小
  function wheel(node: Element) {
    const h = (e: Event) => {
      const we = e as WheelEvent
      we.preventDefault()
      const r = node.getBoundingClientRect()
      zoomBy(Math.exp(-we.deltaY / (we.ctrlKey ? 100 : 400)), { x: we.clientX - r.left, y: we.clientY - r.top })
    }
    node.addEventListener('wheel', h, { passive: false })
    return { destroy: () => node.removeEventListener('wheel', h) }
  }

  function onKey(e: KeyboardEvent) {
    if (e.key === '0') {
      e.preventDefault()
      return zoomFit()
    }
    if (!selected) return
    const step = e.shiftKey ? 50 : 10
    const move = (dx: number, dy: number) => {
      e.preventDefault()
      onChange(selected.id, { x: selected.x + dx, y: selected.y + dy }, `move:${selected.id}`)
    }
    if (e.key === 'ArrowLeft') move(-step, 0)
    else if (e.key === 'ArrowRight') move(step, 0)
    else if (e.key === 'ArrowUp') move(0, -step)
    else if (e.key === 'ArrowDown') move(0, step)
    else if (e.key === '+' || e.key === '=') {
      e.preventDefault()
      onChange(selected.id, { scale: selected.scale * 1.05 }, `scale:${selected.id}`)
    } else if (e.key === '-') {
      e.preventDefault()
      onChange(selected.id, { scale: selected.scale / 1.05 }, `scale:${selected.id}`)
    } else if (e.key === 'Delete' || e.key === 'Backspace') {
      e.preventDefault()
      onRemove(selected.id)
    } else if (e.key === 'Escape') onSelect(null)
  }

  const emptyText = (it: EditorItem) => it.kind === 'text' && it.text.trim() === ''
</script>

<svelte:window bind:innerHeight={viewportH} />

<div class="area" bind:clientWidth={avail} bind:this={area}>
  <div class="stage" style="width:{width}px;height:{height}px" role="img" aria-label={label}>
    <canvas bind:this={canvas} style="width:{width}px;height:{height}px" data-testid="preview"></canvas>
    {#if readout}
      <div class="readout" role="status" data-testid="readout">{readout}</div>
    {/if}
    <div class="zoom" role="group" aria-label={strings.editor.zoomIn}>
      <button onclick={() => zoomBy(1.4)} disabled={zm.z >= MAX_ZOOM} aria-label={strings.editor.zoomIn} data-testid="zoom-in">＋</button>
      <button onclick={() => zoomBy(1 / 1.4)} disabled={zm.z <= 1} aria-label={strings.editor.zoomOut} data-testid="zoom-out">－</button>
      {#if zm.z > 1}
        <button class="fit" onclick={zoomFit} data-testid="zoom-fit">{strings.editor.zoomFit}</button>
      {/if}
    </div>
    <!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
    <svg
      class="overlay"
      {width}
      {height}
      role="application"
      aria-label={strings.editor.canvasLabel}
      tabindex="0"
      data-testid="stage"
      onpointerdown={onDown}
      onpointermove={onMove}
      onpointerup={onUp}
      onpointercancel={onUp}
      class:drawing
      style:cursor={drawing ? 'crosshair' : cursor}
      onkeydown={onKey}
      use:wheel
    >
      {#each guides.xs as gx (gx)}
        <line x1={view.ox + gx * view.scale} y1="0" x2={view.ox + gx * view.scale} y2={height} class="guide" data-testid="guide" />
      {/each}
      {#each guides.ys as gy (gy)}
        <line x1="0" y1={view.oy + gy * view.scale} x2={width} y2={view.oy + gy * view.scale} class="guide" data-testid="guide" />
      {/each}
      {#each items as it, i (it.id)}
        {@const b = boxes[i]}
        {@const c = px(b)}
        <g transform="translate({c.x} {c.y}) rotate({b.rot})" data-testid="item-{it.id}" data-kind={it.kind}>
          <rect
            x={(-b.w * view.scale) / 2}
            y={(-b.h * view.scale) / 2}
            width={b.w * view.scale}
            height={b.h * view.scale}
            class="box"
            class:on={it.id === selectedId}
            class:empty={emptyText(it)}
            class:locked={it.locked}
          />
          {#if it.locked}
            <text class="lockmark" x={(b.w * view.scale) / 2 - 4} y={(-b.h * view.scale) / 2 + 16} text-anchor="end">🔒</text>
          {/if}
          {#if emptyText(it)}
            <text class="hint" text-anchor="middle" dominant-baseline="middle">{strings.editor.typeHere}</text>
          {/if}
        </g>
      {/each}
      {#if drawing && live.length > 0}
        <polyline
          class="live"
          points={live.map((p) => `${p.x},${p.y}`).join(' ')}
          style="stroke:{brush.css};stroke-width:{Math.max(1.5, brush.units * fit * view.scale)}px"
        />
      {/if}
      {#if selected && selectedBox && !drawing && !selected.locked}
        {#each corners(selectedBox) as k (`${k.sx},${k.sy}`)}
          {@const p = cornerPx(selectedBox, k.sx, k.sy)}
          <circle cx={p.x} cy={p.y} r={HANDLE_R} class="handle" data-testid="handle-resize" />
        {/each}
        {@const top = px({ ...selectedBox, h: selectedBox.h })}
        {@const rp = rotatePx(selectedBox)}
        {@const edge = rotationHandle(selectedBox, 0)}
        <line x1={view.ox + edge.x * view.scale} y1={view.oy + edge.y * view.scale} x2={rp.x} y2={rp.y} class="stem" />
        <circle cx={rp.x} cy={rp.y} r={HANDLE_R} class="handle rot" data-testid="handle-rotate" />
        <!-- 把外框的中心也標出來，旋轉時比較好對 -->
        <circle cx={top.x} cy={top.y} r="2" class="center" />
      {/if}
    </svg>
  </div>
</div>

<style>
  .area { width: 100%; display: flex; justify-content: center; }
  .stage { position: relative; touch-action: none; }
  canvas { display: block; border-radius: 12px; }
  .overlay { position: absolute; inset: 0; touch-action: none; user-select: none; -webkit-user-select: none; outline-offset: -3px; }
  .box { fill: transparent; stroke: rgba(70, 60, 50, 0.28); stroke-width: 1; stroke-dasharray: 4 4; }
  .box.on { stroke: #1a73e8; stroke-width: 2; stroke-dasharray: none; fill: rgba(26, 115, 232, 0.06); }
  .box.empty { fill: rgba(255, 255, 255, 0.6); stroke: #1a73e8; stroke-dasharray: 6 4; }
  .box.locked { stroke: #8a8a8a; stroke-dasharray: 2 3; }
  .lockmark { font-size: 14px; pointer-events: none; }
  .hint { fill: #1a73e8; font-size: 15px; font-weight: 700; pointer-events: none; }
  .handle { fill: #fff; stroke: #1a73e8; stroke-width: 2.5; }
  .handle.rot { fill: #1a73e8; }
  .stem { stroke: #1a73e8; stroke-width: 1.5; }
  .center { fill: #1a73e8; }
  .overlay.drawing { cursor: crosshair; }
  .live { fill: none; stroke-linecap: round; stroke-linejoin: round; opacity: 0.85; }
  .readout { position: absolute; left: 50%; top: 8px; transform: translateX(-50%); z-index: 3; background: rgba(30, 30, 30, 0.82); color: #fff; padding: 6px 14px; border-radius: 999px; font-size: 0.95em; font-weight: 700; pointer-events: none; white-space: nowrap; }
  .zoom { position: absolute; top: 8px; right: 8px; z-index: 3; display: flex; flex-direction: column; gap: 6px; align-items: stretch; }
  .zoom button { min-width: 44px; min-height: 44px; border: 2px solid var(--line); border-radius: 12px; background: rgba(255, 255, 255, 0.92); font-size: 1.3em; font-weight: 700; line-height: 1; padding: 0 8px; }
  .zoom button:disabled { opacity: 0.35; }
  .zoom .fit { font-size: 0.85em; }
  .guide { stroke: #e0457b; stroke-width: 1; stroke-dasharray: 5 4; }
</style>
