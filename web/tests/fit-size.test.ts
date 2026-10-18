import { describe, expect, it } from 'vitest'
import { fitImageSizeCm } from '../src/lib/fitSize'
import type { Hoop } from '../src/lib/types'

const hoop = (w: number, h: number, shape: Hoop['shape'] = 'rect', margin = 50): Hoop => ({
  id: 'x', name: 'x', width: w, height: h, shape, margin, builtin: false,
})

describe('圖片預設大小：剛好放滿繡框', () => {
  it('橫的圖放進大框，受限於高度', () => {
    // 大框 26×16 cm、邊距 5mm → 可用 25×15；圖 2:1 → 長邊（寬）最多 25，但高 12.5 沒問題 → 20（滑桿上限）
    expect(fitImageSizeCm(hoop(2600, 1600), 300, 150)).toBe(20)
  })
  it('直的圖受限於可用寬度', () => {
    // 小框 5×7.5 cm、邊距 5mm、再留 0.6 → 可用 3.4×5.9；圖 1:2（寬:高）→ 長邊是高 = min(5.9, 3.4/0.5=6.8)，捨去到 0.5 → 5.5
    expect(fitImageSizeCm(hoop(500, 750), 100, 200)).toBe(5.5)
  })
  it('橫的圖受限於可用高度', () => {
    // 中框 12×18 cm（1200×1800），可用 10.4×16.4；
    // 橫放的圖 2:1 → 長邊=寬，高=寬/2 ≤ 17 → 寬 ≤ 10.4，捨去到 0.5 → 10
    expect(fitImageSizeCm(hoop(1200, 1800), 200, 100)).toBe(10)
  })
  it('圓框只能用內接的方形，更小', () => {
    const rect = fitImageSizeCm(hoop(1000, 1000), 100, 100)
    const round = fitImageSizeCm(hoop(1000, 1000, 'circle'), 100, 100)
    expect(round).toBeLessThan(rect)
    expect(round).toBeGreaterThanOrEqual(2)
  })
  it('最小 2 公分、不超過滑桿上限 20 公分，並且無條件捨去到 0.5', () => {
    expect(fitImageSizeCm(hoop(100, 100), 100, 100)).toBe(2)
    expect(fitImageSizeCm(hoop(9000, 9000), 100, 100)).toBe(20)
    expect(fitImageSizeCm(hoop(1000, 1000), 100, 100)).toBe(8)
  })
})
