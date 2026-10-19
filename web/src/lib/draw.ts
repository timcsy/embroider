// 把針跡畫成像真的繡線：每種顏色先畫較深的底線，再疊一層較亮的細線表現光澤。
import { computeView, planSegments, toCanvas, type Segment, type View } from './preview'
import type { Hoop, StitchPlan } from './types'

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

  if (!plan) return

  const segs = planSegments(plan, upto)
  const byColor = new Map<number, Segment[]>()
  for (const s of segs) {
    const list = byColor.get(s.color)
    if (list) list.push(s)
    else byColor.set(s.color, [s])
  }
  const thread = Math.max(1, 4.2 * view.scale)
  ctx.lineCap = 'round'
  for (const [ci, list] of byColor) {
    const c = plan.colors[ci] ?? { r: 128, g: 128, b: 128, name: '' }
    const pass = (color: string, width: number, dx: number, dy: number) => {
      ctx.beginPath()
      for (const s of list) {
        const [x0, y0] = toCanvas(view, s.x0, s.y0)
        const [x1, y1] = toCanvas(view, s.x1, s.y1)
        ctx.moveTo(x0 + dx, y0 + dy)
        ctx.lineTo(x1 + dx, y1 + dy)
      }
      ctx.lineWidth = width
      ctx.strokeStyle = color
      ctx.stroke()
    }
    pass(shade(c.r, c.g, c.b, -0.35), thread * 1.25, 0, 0)
    pass(shade(c.r, c.g, c.b, 0), thread * 0.85, 0, 0)
    pass(shade(c.r, c.g, c.b, 0.35), Math.max(0.6, thread * 0.3), -0.25, -0.25)
  }
}
