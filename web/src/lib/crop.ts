// 圈出圖片裡「想繡的部分」：框外的不繡。
// 這張圖裡哪裡重要，只有人知道；與其讓程式猜，不如讓使用者直接圈出來，
// 圈小之後，框內的細節也能用更多像素保留下來。
export interface CropRect {
  /** 以整張圖為 1 的比例座標，x0 < x1、y0 < y1 */
  x0: number
  y0: number
  x1: number
  y1: number
}

export interface Pt {
  x: number
  y: number
}

/** 小於這個比例的框，視為手指點了一下，不算圈範圍。 */
const MIN_SIDE = 0.05

const clamp01 = (v: number) => Math.min(1, Math.max(0, v))

/** 由拖曳的起點與終點得到框；太小回傳 null。 */
export function rectFromDrag(a: Pt, b: Pt): CropRect | null {
  const x0 = clamp01(Math.min(a.x, b.x))
  const x1 = clamp01(Math.max(a.x, b.x))
  const y0 = clamp01(Math.min(a.y, b.y))
  const y1 = clamp01(Math.max(a.y, b.y))
  if (x1 - x0 < MIN_SIDE || y1 - y0 < MIN_SIDE) return null
  return { x0, y0, x1, y1 }
}

export interface Pixels {
  rgba: Uint8Array
  width: number
  height: number
}

/** 裁出框內的像素；沒有框就原樣回傳（同一個物件）。 */
export function cropRgba(rgba: Uint8Array, w: number, h: number, rect: CropRect | null): Pixels {
  if (!rect) return { rgba, width: w, height: h }
  const x0 = Math.min(w - 1, Math.max(0, Math.round(rect.x0 * w)))
  const y0 = Math.min(h - 1, Math.max(0, Math.round(rect.y0 * h)))
  const x1 = Math.min(w, Math.max(x0 + 1, Math.round(rect.x1 * w)))
  const y1 = Math.min(h, Math.max(y0 + 1, Math.round(rect.y1 * h)))
  const width = x1 - x0
  const height = y1 - y0
  const out = new Uint8Array(width * height * 4)
  for (let y = 0; y < height; y++) {
    const from = ((y0 + y) * w + x0) * 4
    out.set(rgba.subarray(from, from + width * 4), y * width * 4)
  }
  return { rgba: out, width, height }
}
