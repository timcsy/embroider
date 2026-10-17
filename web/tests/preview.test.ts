import { describe, expect, it } from 'vitest'
import { colorIndexAt, computeView, planSegments, toCanvas } from '../src/lib/preview'
import type { Hoop, StitchPlan } from '../src/lib/types'

const hoop = (w: number, h: number): Hoop => ({
  id: 'h', name: 'h', width: w, height: h, shape: 'rect', margin: 50, builtin: false,
})

const red = { r: 200, g: 30, b: 30, name: '正紅' }
const blue = { r: 30, g: 30, b: 200, name: '深藍' }

describe('預覽座標換算', () => {
  it('框完整放進畫布並置中', () => {
    const v = computeView(hoop(1000, 500), 400, 400, 20)
    // 可用寬度 360 → 每 0.1mm 為 0.36 像素
    expect(v.scale).toBeCloseTo(0.36)
    const [cx, cy] = toCanvas(v, 0, 0)
    expect(cx).toBeCloseTo(200)
    expect(cy).toBeCloseTo(200)
    const [left] = toCanvas(v, -500, 0)
    expect(left).toBeCloseTo(20)
  })

  it('高度為限制時以高度縮放', () => {
    const v = computeView(hoop(500, 1000), 400, 400, 20)
    expect(v.scale).toBeCloseTo(0.36)
  })

  it('y 軸向下：正的 y 在畫面較下方', () => {
    const v = computeView(hoop(1000, 1000), 400, 400, 0)
    expect(toCanvas(v, 0, 100)[1]).toBeGreaterThan(toCanvas(v, 0, 0)[1])
  })
})

describe('針跡線段', () => {
  const plan: StitchPlan = {
    colors: [red, blue],
    stitches: [
      { x: 0, y: 0, kind: 'Jump' },
      { x: 10, y: 0, kind: 'Stitch' },
      { x: 20, y: 0, kind: 'Stitch' },
      { x: 20, y: 0, kind: 'Trim' },
      { x: 100, y: 0, kind: 'Jump' },
      { x: 110, y: 0, kind: 'Stitch' },
      { x: 110, y: 0, kind: 'ColorChange' },
      { x: 110, y: 10, kind: 'Stitch' },
      { x: 110, y: 10, kind: 'End' },
    ],
  }

  it('只有一般針會畫線，跳針與剪線不畫', () => {
    const segs = planSegments(plan)
    expect(segs).toHaveLength(4)
    expect(segs[0]).toMatchObject({ x0: 0, y0: 0, x1: 10, y1: 0 })
    expect(segs[2]).toMatchObject({ x0: 100, y0: 0, x1: 110, y1: 0 })
  })

  it('換色之後線段使用下一個顏色', () => {
    const segs = planSegments(plan)
    expect(segs.slice(0, 3).every((s) => s.color === 0)).toBe(true)
    expect(segs[3].color).toBe(1)
  })

  it('空計畫沒有線段', () => {
    expect(planSegments({ colors: [], stitches: [] })).toEqual([])
  })

  it('指定針數時只算到那一針（模擬播放用）', () => {
    expect(planSegments(plan, 3)).toHaveLength(2)
    expect(colorIndexAt(plan, 3)).toBe(0)
    expect(colorIndexAt(plan, 8)).toBe(1)
  })
})
