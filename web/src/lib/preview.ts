// 預覽繪製用的純函式（不依賴 DOM，方便測試）。座標單位 0.1mm，y 軸向下，原點在框中心。
import type { Hoop, StitchPlan } from './types'

export interface View {
  /** 每 0.1mm 對應的畫布像素數 */
  scale: number
  ox: number
  oy: number
}

export interface Segment {
  x0: number
  y0: number
  x1: number
  y1: number
  /** 使用 plan.colors 的第幾個顏色 */
  color: number
}

/** 讓整個繡框完整放進畫布並置中。 */
export function computeView(hoop: Hoop, canvasW: number, canvasH: number, padding: number): View {
  const scale = Math.min((canvasW - 2 * padding) / hoop.width, (canvasH - 2 * padding) / hoop.height)
  return { scale, ox: canvasW / 2, oy: canvasH / 2 }
}

export function toCanvas(view: View, x: number, y: number): [number, number] {
  return [view.ox + x * view.scale, view.oy + y * view.scale]
}

/** 只有一般針會畫線；跳針、剪線、換色只是移動位置。`upto` 為只算前幾筆（模擬播放用）。 */
export function planSegments(plan: StitchPlan, upto = plan.stitches.length): Segment[] {
  const segs: Segment[] = []
  let px = 0
  let py = 0
  let color = 0
  for (let i = 0; i < Math.min(upto, plan.stitches.length); i++) {
    const s = plan.stitches[i]
    if (s.kind === 'Stitch') segs.push({ x0: px, y0: py, x1: s.x, y1: s.y, color })
    if (s.kind === 'ColorChange') color++
    px = s.x
    py = s.y
  }
  return segs
}

/** 播放到第 `upto` 筆時使用的顏色編號。 */
export function colorIndexAt(plan: StitchPlan, upto: number): number {
  let n = 0
  for (let i = 0; i < Math.min(upto, plan.stitches.length); i++) {
    if (plan.stitches[i].kind === 'ColorChange') n++
  }
  return n
}
