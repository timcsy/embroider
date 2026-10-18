import { describe, expect, it } from 'vitest'
import { cropRgba, rectFromDrag } from '../src/lib/crop'

// 4×3 的小圖，每個像素的紅色值 = 它的編號，方便核對裁出來的是哪幾格
function sample(w = 4, h = 3) {
  const rgba = new Uint8Array(w * h * 4)
  for (let i = 0; i < w * h; i++) rgba.set([i, 0, 0, 255], i * 4)
  return { rgba, w, h }
}

describe('圈出要繡的範圍', () => {
  it('依比例裁出對應的像素', () => {
    const { rgba, w, h } = sample()
    // 右半邊（x 從 50% 到 100%）、中間一列（y 從 33% 到 67%）
    const r = cropRgba(rgba, w, h, { x0: 0.5, y0: 1 / 3, x1: 1, y1: 2 / 3 })
    expect(r.width).toBe(2)
    expect(r.height).toBe(1)
    expect(Array.from(r.rgba).filter((_, i) => i % 4 === 0)).toEqual([6, 7])
  })

  it('整張圖（沒有範圍）原樣回傳', () => {
    const { rgba, w, h } = sample()
    const r = cropRgba(rgba, w, h, null)
    expect(r.width).toBe(4)
    expect(r.rgba).toBe(rgba)
  })

  it('範圍超出圖片時收進圖片裡，且至少留一格', () => {
    const { rgba, w, h } = sample()
    const r = cropRgba(rgba, w, h, { x0: -0.5, y0: -1, x1: 0.01, y1: 0.01 })
    expect(r.width).toBeGreaterThanOrEqual(1)
    expect(r.height).toBeGreaterThanOrEqual(1)
    expect(r.rgba.length).toBe(r.width * r.height * 4)
  })

  it('拖曳的起點終點不分先後，也不會超出圖片', () => {
    const r = rectFromDrag({ x: 0.8, y: 0.9 }, { x: 0.2, y: 0.1 })!
    expect(r).toEqual({ x0: 0.2, y0: 0.1, x1: 0.8, y1: 0.9 })
    const out = rectFromDrag({ x: -0.3, y: 0.2 }, { x: 1.4, y: 0.7 })!
    expect(out.x0).toBe(0)
    expect(out.x1).toBe(1)
  })

  it('拖得太小（手指點一下）不算圈了範圍', () => {
    expect(rectFromDrag({ x: 0.5, y: 0.5 }, { x: 0.52, y: 0.51 })).toBeNull()
  })
})
