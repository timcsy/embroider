// 把針跡畫成像真的繡線：每種顏色先畫較深的底線，再疊一層較亮的細線表現光澤。
import { computeView, planSegments, toCanvas, type Segment, type View } from './preview'
import type { Hoop, StitchPlan, ThreadColor } from './types'

const CLOTH = '#f1eadb'
const HOOP_WOOD = '#b08d57'

function shade(r: number, g: number, b: number, k: number): string {
  const f = (v: number) => Math.max(0, Math.min(255, Math.round(k >= 0 ? v + (255 - v) * k : v * (1 + k))))
  return `rgb(${f(r)},${f(g)},${f(b)})`
}

function hoopPath(ctx: CanvasRenderingContext2D, hoop: Hoop, view: ReturnType<typeof computeView>, inset = 0) {
  const w = (hoop.width - 2 * inset) * view.scale
  const h = (hoop.height - 2 * inset) * view.scale
  ctx.beginPath()
  if (hoop.shape === 'rect') {
    ctx.roundRect(view.ox - w / 2, view.oy - h / 2, w, h, 6)
  } else {
    ctx.ellipse(view.ox, view.oy, w / 2, h / 2, 0, 0, Math.PI * 2)
  }
}

function paintBase(
  ctx: CanvasRenderingContext2D,
  hoop: Hoop,
  cssW: number,
  cssH: number,
  dpr: number,
  viewOverride?: View,
): View {
  ctx.setTransform(dpr, 0, 0, dpr, 0, 0)
  ctx.clearRect(0, 0, cssW, cssH)
  ctx.fillStyle = '#faf6ee'
  ctx.fillRect(0, 0, cssW, cssH)
  const view = viewOverride ?? computeView(hoop, cssW, cssH, 14)

  // 布料與框
  hoopPath(ctx, hoop, view)
  ctx.fillStyle = CLOTH
  ctx.fill()
  ctx.lineWidth = 5
  ctx.strokeStyle = HOOP_WOOD
  ctx.stroke()
  // 安全範圍（虛線）
  ctx.save()
  hoopPath(ctx, hoop, view, hoop.margin)
  ctx.setLineDash([4, 4])
  ctx.lineWidth = 1
  ctx.strokeStyle = 'rgba(120,100,70,0.35)'
  ctx.stroke()
  ctx.restore()
  return view
}

const THREAD_MM = 4.2
const GLOSS_MAX_SEGMENTS = 6000

/** 把同一種線色的一堆線段畫成像繡線：深色底線、本色、細的亮線表現光澤。 */
function paintColor(ctx: CanvasRenderingContext2D, c: { r: number; g: number; b: number }, segs: [number, number, number, number][], view: View) {
  const thread = Math.max(1, THREAD_MM * view.scale)
  ctx.lineCap = 'round'
  const pass = (color: string, width: number, dx: number, dy: number) => {
    ctx.beginPath()
    for (const [x0, y0, x1, y1] of segs) {
      const [ax, ay] = toCanvas(view, x0, y0)
      const [bx, by] = toCanvas(view, x1, y1)
      ctx.moveTo(ax + dx, ay + dy)
      ctx.lineTo(bx + dx, by + dy)
    }
    ctx.lineWidth = width
    ctx.strokeStyle = color
    ctx.stroke()
  }
  pass(shade(c.r, c.g, c.b, -0.35), thread * 1.25, 0, 0)
  pass(shade(c.r, c.g, c.b, 0), thread * 0.85, 0, 0)
  // 線段非常多的時候，省掉最後那層光澤（肉眼差別很小，但畫一次要多花三分之一的時間）
  if (segs.length <= GLOSS_MAX_SEGMENTS) pass(shade(c.r, c.g, c.b, 0.35), Math.max(0.6, thread * 0.3), -0.25, -0.25)
}

export function drawPreview(
  ctx: CanvasRenderingContext2D,
  plan: StitchPlan | null,
  hoop: Hoop,
  cssW: number,
  cssH: number,
  dpr: number,
  upto?: number,
  /** 放大或平移後的畫面；沒給就是整個繡框置中 */
  viewOverride?: View,
): void {
  const view = paintBase(ctx, hoop, cssW, cssH, dpr, viewOverride)
  if (!plan) return
  const byColor = new Map<number, [number, number, number, number][]>()
  for (const s of planSegments(plan, upto)) {
    const list = byColor.get(s.color)
    const seg: [number, number, number, number] = [s.x0, s.y0, s.x1, s.y1]
    if (list) list.push(seg)
    else byColor.set(s.color, [seg])
  }
  for (const [ci, list] of byColor) {
    paintColor(ctx, plan.colors[ci] ?? { r: 128, g: 128, b: 128 }, list, view)
  }
}

/** 一個物件的針跡先畫在一張獨立的圖上（只畫一次），拖動時直接把這張圖搬、轉、放大縮小，畫面才跟得上手指。 */
export interface LayerBitmap {
  canvas: HTMLCanvasElement
  /** 這張圖左上角在「框座標」的位置（再往外多留 `pad` 像素），畫的時候的比例與解析度（用來判斷要不要重畫） */
  hx: number
  hy: number
  pad: number
  scale: number
  dpr: number
}

const MAX_BITMAP = 4096

/** 把一個物件的針跡畫成一張圖。針跡是空的或太大畫不了就回傳 null。 */
export function renderLayerBitmap(
  layer: LayerDraw,
  view: View,
  dpr: number,
  document: Document = globalThis.document,
): LayerBitmap | null {
  let x0 = Infinity
  let y0 = Infinity
  let x1 = -Infinity
  let y1 = -Infinity
  for (const part of layer.parts) {
    for (const run of part.runs) {
      for (const [x, y] of run) {
        x0 = Math.min(x0, x)
        y0 = Math.min(y0, y)
        x1 = Math.max(x1, x)
        y1 = Math.max(y1, y)
      }
    }
  }
  if (!Number.isFinite(x0)) return null
  const pad = 6 * view.scale + 4
  const left = view.ox + x0 * view.scale - pad
  const top = view.oy + y0 * view.scale - pad
  const w = (x1 - x0) * view.scale + 2 * pad
  const h = (y1 - y0) * view.scale + 2 * pad
  const pw = Math.ceil(w * dpr)
  const ph = Math.ceil(h * dpr)
  if (pw <= 0 || ph <= 0 || pw > MAX_BITMAP || ph > MAX_BITMAP) return null
  const canvas = document.createElement('canvas')
  canvas.width = pw
  canvas.height = ph
  const ctx = canvas.getContext('2d')
  if (!ctx) return null
  ctx.setTransform(dpr, 0, 0, dpr, 0, 0)
  // 把畫面的原點平移，讓這張圖的左上角對到 (0,0)
  const local: View = { scale: view.scale, ox: view.ox - left, oy: view.oy - top }
  for (const part of layer.parts) {
    const segs: [number, number, number, number][] = []
    for (const run of part.runs) {
      for (let i = 1; i < run.length; i++) segs.push([run[i - 1][0], run[i - 1][1], run[i][0], run[i][1]])
    }
    paintColor(ctx, part.color, segs, local)
  }
  return { canvas, hx: x0, hy: y0, pad, scale: view.scale, dpr }
}

/**
 * 用各物件的針跡圖畫：先畫底，再依序把每個物件的圖（可能已經被搬到新的位置、轉了方向、放大縮小）貼上去。
 * `move` 是以畫布像素表示的變換：以舊中心為中心放大 k、轉 angle，再平移到新中心。
 */
export interface BitmapPlacement {
  bitmap: LayerBitmap
  from: { x: number; y: number }
  to: { x: number; y: number }
  k: number
  angle: number
}

export function drawBitmaps(
  ctx: CanvasRenderingContext2D,
  placements: BitmapPlacement[],
  hoop: Hoop,
  cssW: number,
  cssH: number,
  dpr: number,
  viewOverride?: View,
): void {
  const view = paintBase(ctx, hoop, cssW, cssH, dpr, viewOverride)
  for (const p of placements) {
    const left = view.ox + p.bitmap.hx * view.scale - p.bitmap.pad
    const top = view.oy + p.bitmap.hy * view.scale - p.bitmap.pad
    ctx.save()
    ctx.translate(p.to.x, p.to.y)
    ctx.rotate(p.angle)
    ctx.scale(p.k, p.k)
    ctx.translate(-p.from.x, -p.from.y)
    ctx.drawImage(p.bitmap.canvas, left, top, p.bitmap.canvas.width / p.bitmap.dpr, p.bitmap.canvas.height / p.bitmap.dpr)
    ctx.restore()
  }
}

/** 一個物件的針跡層，加上把它搬到現在位置的函式（沒有就是原位）。 */
export interface LayerDraw {
  parts: { color: ThreadColor; runs: [number, number][][] }[]
  map: ((p: [number, number]) => [number, number]) | null
}

/**
 * 用各物件的針跡層畫：拖動、縮放、旋轉的計算中途，把舊的針跡直接變形到手指現在的位置，
 * 畫面立刻跟著動，不必等核心重新計算。順序照物件的上下層。
 */
export function drawLayers(
  ctx: CanvasRenderingContext2D,
  layers: LayerDraw[],
  hoop: Hoop,
  cssW: number,
  cssH: number,
  dpr: number,
  viewOverride?: View,
): void {
  const view = paintBase(ctx, hoop, cssW, cssH, dpr, viewOverride)
  for (const layer of layers) {
    for (const part of layer.parts) {
      const segs: [number, number, number, number][] = []
      for (const run of part.runs) {
        let prev = layer.map ? layer.map(run[0]) : run[0]
        for (let i = 1; i < run.length; i++) {
          const cur = layer.map ? layer.map(run[i]) : run[i]
          segs.push([prev[0], prev[1], cur[0], cur[1]])
          prev = cur
        }
      }
      paintColor(ctx, part.color, segs, view)
    }
  }
}
