// 編輯畫面的幾何：外框、點選、縮放、旋轉、吸附。不依賴 DOM，座標單位 0.1mm、原點在框中心、y 軸向下。
import type { View } from './preview'

export interface Pt {
  x: number
  y: number
}

/** 物件在畫面上的外框（已套用整體縮小）：中心、寬高、旋轉（度，順時針）。 */
export interface Box {
  cx: number
  cy: number
  w: number
  h: number
  rot: number
}

export function itemBox(
  item: { x: number; y: number; rotation: number },
  info: { width: number; height: number },
  fit: number,
): Box {
  return { cx: item.x * fit, cy: item.y * fit, w: info.width * fit, h: info.height * fit, rot: item.rotation }
}

const rad = (deg: number) => (deg * Math.PI) / 180

/** 把點轉到物件自己的座標（物件沒旋轉、中心在原點）。 */
function toLocal(b: Box, p: Pt): Pt {
  const dx = p.x - b.cx
  const dy = p.y - b.cy
  const c = Math.cos(rad(-b.rot))
  const s = Math.sin(rad(-b.rot))
  return { x: dx * c - dy * s, y: dx * s + dy * c }
}

/** 點在外框裡嗎？`pad` 是手指粗細的容許範圍。 */
export function hitBox(b: Box, p: Pt, pad: number): boolean {
  const l = toLocal(b, p)
  return Math.abs(l.x) <= b.w / 2 + pad && Math.abs(l.y) <= b.h / 2 + pad
}

/** 最上面（最後面）被點到的物件編號；沒有回傳 -1。 */
export function topHit(boxes: Box[], p: Pt, pad: number): number {
  for (let i = boxes.length - 1; i >= 0; i--) if (hitBox(boxes[i], p, pad)) return i
  return -1
}

const dist = (a: Pt, b: Pt) => Math.hypot(a.x - b.x, a.y - b.y)

/** 拉角改大小：新縮放 = 原縮放 × 手指離中心的距離比。 */
export function resizeBy(startScale: number, from: Pt, to: Pt, center: Pt): number {
  const d0 = Math.max(1e-6, dist(from, center))
  return startScale * Math.max(0.01, dist(to, center) / d0)
}

/** 旋轉把手的位置：在物件上緣外 `offset`。 */
export function rotationHandle(b: Box, offset: number): Pt {
  const r = b.h / 2 + offset
  return { x: b.cx + r * Math.sin(rad(b.rot)), y: b.cy - r * Math.cos(rad(b.rot)) }
}

/** 手指在 `to`、中心在 `center` 時，物件該轉到幾度（把手原本在正上方）。 */
export function rotateTo(center: Pt, to: Pt): number {
  return (Math.atan2(to.x - center.x, -(to.y - center.y)) * 180) / Math.PI
}

const SNAP_STEP = 45
const SNAP_RANGE = 4

/** 接近 0、45、90…度時吸過去，方便擺正。 */
export function snapAngle(deg: number): number {
  const near = Math.round(deg / SNAP_STEP) * SNAP_STEP
  return Math.abs(deg - near) <= SNAP_RANGE ? near : deg
}

/** 靠近框的水平、垂直中線時吸過去；回傳有沒有吸到，畫面可以畫出參考線。 */
export function snapToCenter(x: number, y: number, range: number) {
  const sx = Math.abs(x) <= range
  const sy = Math.abs(y) <= range
  return { x: sx ? 0 : x, y: sy ? 0 : y, sx, sy }
}

/** 兩指：縮放看距離比、旋轉看角度差。 */
export function pinch(
  start: { scale: number; rotation: number },
  a: [Pt, Pt],
  b: [Pt, Pt],
): { scale: number; rotation: number } {
  const d0 = Math.max(1e-6, dist(a[0], a[1]))
  const ang = (p: [Pt, Pt]) => (Math.atan2(p[1].y - p[0].y, p[1].x - p[0].x) * 180) / Math.PI
  return { scale: start.scale * (dist(b[0], b[1]) / d0), rotation: start.rotation + (ang(b) - ang(a)) }
}

/** 畫布上的點換成框座標。 */
export function fromCanvas(view: View, px: number, py: number): Pt {
  return { x: (px - view.ox) / view.scale, y: (py - view.oy) / view.scale }
}

// ── 畫布放大與平移 ──

export interface Zoom {
  /** 放大倍率（1 = 看完整個繡框） */
  z: number
  /** 平移（畫布像素） */
  px: number
  py: number
}

export const MIN_ZOOM = 1
export const MAX_ZOOM = 8

/** 基本畫面（整個繡框放得進畫布）加上放大與平移，得到實際使用的畫面。 */
export function applyZoom(base: View, zm: Zoom): View {
  return { scale: base.scale * zm.z, ox: base.ox + zm.px, oy: base.oy + zm.py }
}

const clampZ = (z: number) => Math.min(MAX_ZOOM, Math.max(MIN_ZOOM, z))

/** 以畫布上的某一點為中心放大或縮小：那一點底下的位置不動。 */
export function zoomAround(base: View, zm: Zoom, at: Pt, nextZ: number): Zoom {
  const z = clampZ(nextZ)
  const k = z / zm.z
  const ox = base.ox + zm.px
  const oy = base.oy + zm.py
  return { z, px: at.x - (at.x - ox) * k - base.ox, py: at.y - (at.y - oy) * k - base.oy }
}

/** 限制平移：沒放大就不能平移；放大後至少要留一小塊繡框在畫面裡。 */
export function clampZoom(base: View, zm: Zoom, canvasW: number, canvasH: number, hoopW: number, hoopH: number): Zoom {
  const z = clampZ(zm.z)
  if (z <= MIN_ZOOM + 1e-9) return { z: MIN_ZOOM, px: 0, py: 0 }
  const keep = 60
  const lx = (hoopW * base.scale * z) / 2 + canvasW / 2 - keep
  const ly = (hoopH * base.scale * z) / 2 + canvasH / 2 - keep
  return { z, px: Math.min(lx, Math.max(-lx, zm.px)), py: Math.min(ly, Math.max(-ly, zm.py)) }
}

/** 兩指縮放加平移：兩指起始的中點底下的位置，跟著兩指現在的中點走。 */
export function pinchZoom(base: View, start: Zoom, a: [Pt, Pt], b: [Pt, Pt]): Zoom {
  const d0 = Math.max(1e-6, Math.hypot(a[1].x - a[0].x, a[1].y - a[0].y))
  const d1 = Math.hypot(b[1].x - b[0].x, b[1].y - b[0].y)
  const z = clampZ(start.z * (d1 / d0))
  const ma = { x: (a[0].x + a[1].x) / 2, y: (a[0].y + a[1].y) / 2 }
  const mb = { x: (b[0].x + b[1].x) / 2, y: (b[0].y + b[1].y) / 2 }
  const v0 = applyZoom(base, start)
  const hx = (ma.x - v0.ox) / v0.scale
  const hy = (ma.y - v0.oy) / v0.scale
  const s1 = base.scale * z
  return { z, px: mb.x - hx * s1 - base.ox, py: mb.y - hy * s1 - base.oy }
}
