import { describe, expect, it } from 'vitest'
import {
  fromCanvas,
  hitBox,
  itemBox,
  pinch,
  resizeBy,
  rotateTo,
  rotationHandle,
  snapAngle,
  snapToCenter,
  topHit,
  type Box,
} from '../src/lib/geometry'

const box = (o: Partial<Box> = {}): Box => ({ cx: 0, cy: 0, w: 200, h: 100, rot: 0, ...o })

describe('物件外框', () => {
  it('位置與大小都乘上「整體縮小」的倍率', () => {
    const b = itemBox({ x: 100, y: -50, rotation: 20 }, { width: 400, height: 200 }, 0.5)
    expect(b).toEqual({ cx: 50, cy: -25, w: 200, h: 100, rot: 20 })
  })
})

describe('點到哪個物件', () => {
  it('點在框裡算點到，框外不算', () => {
    expect(hitBox(box(), { x: 90, y: 40 }, 0)).toBe(true)
    expect(hitBox(box(), { x: 110, y: 0 }, 0)).toBe(false)
  })

  it('旋轉九十度後，寬高對調', () => {
    const b = box({ rot: 90 })
    expect(hitBox(b, { x: 0, y: 90 }, 0)).toBe(true)
    expect(hitBox(b, { x: 90, y: 0 }, 0)).toBe(false)
  })

  it('手指很粗：留一圈容許範圍', () => {
    expect(hitBox(box(), { x: 108, y: 0 }, 10)).toBe(true)
  })

  it('重疊時選最上面的（陣列後面的）', () => {
    const boxes = [box(), box({ cx: 20 })]
    expect(topHit(boxes, { x: 0, y: 0 }, 0)).toBe(1)
  })

  it('什麼都沒點到回傳 -1', () => {
    expect(topHit([box()], { x: 500, y: 500 }, 0)).toBe(-1)
  })
})

describe('拉角改大小', () => {
  it('離中心的距離變兩倍，縮放就變兩倍', () => {
    expect(resizeBy(1, { x: 100, y: 0 }, { x: 200, y: 0 }, { x: 0, y: 0 })).toBeCloseTo(2)
  })

  it('不會縮到零', () => {
    expect(resizeBy(1, { x: 100, y: 0 }, { x: 0, y: 0 }, { x: 0, y: 0 })).toBeGreaterThan(0)
  })
})

describe('旋轉', () => {
  it('旋轉把手在物件上方', () => {
    const h = rotationHandle(box({ h: 100 }), 40)
    expect(h.x).toBeCloseTo(0)
    expect(h.y).toBeCloseTo(-90)
  })

  it('旋轉後把手跟著轉', () => {
    const h = rotationHandle(box({ h: 100, rot: 90 }), 40)
    expect(h.x).toBeCloseTo(90)
    expect(h.y).toBeCloseTo(0)
  })

  it('手指移到正右邊，物件轉到 90 度（把手原本在上方）', () => {
    expect(rotateTo({ x: 0, y: 0 }, { x: 100, y: 0 })).toBeCloseTo(90)
  })

  it('接近 0、45、90 度時吸附，方便擺正', () => {
    expect(snapAngle(2)).toBe(0)
    expect(snapAngle(88)).toBe(90)
    expect(snapAngle(43)).toBe(45)
    expect(snapAngle(60)).toBe(60)
  })
})

describe('對齊中線', () => {
  it('靠近中心線就吸過去，並回報吸到哪一條', () => {
    expect(snapToCenter(12, 300, 30)).toEqual({ x: 0, y: 300, sx: true, sy: false })
    expect(snapToCenter(200, -20, 30)).toEqual({ x: 200, y: 0, sx: false, sy: true })
  })
})

describe('兩指縮放與旋轉', () => {
  it('兩指距離變兩倍、角度轉 90 度', () => {
    const r = pinch(
      { scale: 1, rotation: 10 },
      [{ x: -50, y: 0 }, { x: 50, y: 0 }],
      [{ x: 0, y: -100 }, { x: 0, y: 100 }],
    )
    expect(r.scale).toBeCloseTo(2)
    expect(r.rotation).toBeCloseTo(100)
  })
})

describe('畫面座標換成框座標', () => {
  it('原點在畫布中心、依比例換算', () => {
    const view = { scale: 0.5, ox: 100, oy: 200 }
    expect(fromCanvas(view, 150, 150)).toEqual({ x: 100, y: -100 })
  })
})
