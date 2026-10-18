// 圖片的預設大小：剛好放滿繡框的可繡範圍。
// 圖越大，能保留的細節越多（細線與小字在小尺寸下一定繡不出來），所以預設不要縮得太小。
import type { Hoop } from './types'

export const MIN_IMAGE_CM = 2
export const MAX_IMAGE_CM = 20
const SLIDER_STEP = 0.5
/** 再多留的邊（公分，兩側合計）：拉伸補償與緞面會比圖多出一點，剛好放滿會被擋下來。 */
const EXTRA_CM = 0.6

/** 回傳圖片長邊的公分數，捨去到 0.5，並限制在滑桿範圍內。 */
export function fitImageSizeCm(hoop: Hoop, imgW: number, imgH: number): number {
  // 尺寸單位是 0.1mm；圓形與橢圓只能放下內接的方形
  const inner = hoop.shape === 'rect' ? 1 : Math.SQRT1_2
  const w = ((hoop.width - 2 * hoop.margin) / 100 - EXTRA_CM) * inner
  const h = ((hoop.height - 2 * hoop.margin) / 100 - EXTRA_CM) * inner
  const aspect = imgW / imgH
  // 橫的圖：長邊是寬；直的圖：長邊是高
  const longSide = aspect >= 1 ? Math.min(w, h * aspect) : Math.min(h, w / aspect)
  const stepped = Math.floor(longSide / SLIDER_STEP + 1e-9) * SLIDER_STEP
  return Math.min(MAX_IMAGE_CM, Math.max(MIN_IMAGE_CM, stepped))
}
