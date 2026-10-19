import { describe, expect, it } from 'vitest'
import {
  addImage,
  addShape,
  addText,
  centerItem,
  duplicateItem,
  emptyDoc,
  moveLayer,
  removeItem,
  restoreRemoved,
  toRequest,
  updateItem,
  type EditorDoc,
  type EditorItem,
  type ImageSource,
} from '../src/lib/editor'
import type { Hoop } from '../src/lib/types'

const hoop: Hoop = { id: 'h', name: '中框', width: 1200, height: 1800, shape: 'rect', margin: 50, builtin: true }

const img = (id: string): ImageSource => ({ id, name: `${id}.png`, width: 40, height: 20, rgba: new Uint8Array(40 * 20 * 4) })

describe('加物件', () => {
  it('加文字：有預設字型、顏色，放在中間，並被選取', () => {
    const d = addText(emptyDoc(hoop.id))
    expect(d.items).toHaveLength(1)
    const t = d.items[0]
    expect(t.kind).toBe('text')
    expect(d.selectedId).toBe(t.id)
    expect([t.x, t.y]).toEqual([0, 0])
    expect(t.scale).toBe(1)
  })

  it('連續加物件：位置錯開，才不會疊在一起看不到', () => {
    let d = emptyDoc(hoop.id)
    d = addText(d)
    d = addShape(d, 'circle')
    d = addShape(d, 'heart')
    const pos = new Set(d.items.map((i) => `${i.x},${i.y}`))
    expect(pos.size).toBe(3)
  })

  it('每個物件的編號都不同，刪掉再加也不重複', () => {
    let d = addText(emptyDoc(hoop.id))
    d = addText(d)
    d = removeItem(d, d.items[0].id).doc
    d = addText(d)
    expect(new Set(d.items.map((i) => i.id)).size).toBe(d.items.length)
  })

  it('加圖片：大小放滿框（依框和圖的比例）', () => {
    const d = addImage(emptyDoc(hoop.id), img('a'), hoop)
    const it = d.items[0]
    if (it.kind !== 'image') throw new Error('應是圖片')
    expect(it.imageId).toBe('a')
    expect(it.sizeCm).toBeGreaterThan(2)
  })
})

describe('改物件', () => {
  it('updateItem 只改指定的那個', () => {
    let d = addText(emptyDoc(hoop.id))
    d = addText(d)
    const [a, b] = d.items
    d = updateItem(d, a.id, { x: 123 })
    expect(d.items[0].x).toBe(123)
    expect(d.items[1]).toEqual(b)
  })

  it('不會被改壞：縮放最小 0.1、最大 20', () => {
    let d = addShape(emptyDoc(hoop.id), 'circle')
    const id = d.items[0].id
    d = updateItem(d, id, { scale: 0 })
    expect(d.items[0].scale).toBe(0.1)
    d = updateItem(d, id, { scale: 999 })
    expect(d.items[0].scale).toBe(20)
  })

  it('旋轉整理到 -180 ~ 180 度', () => {
    let d = addShape(emptyDoc(hoop.id), 'rect')
    d = updateItem(d, d.items[0].id, { rotation: 370 })
    expect(d.items[0].rotation).toBe(10)
    d = updateItem(d, d.items[0].id, { rotation: -190 })
    expect(d.items[0].rotation).toBe(170)
  })

  it('置中：水平、垂直、兩者', () => {
    let d = addShape(emptyDoc(hoop.id), 'circle')
    const id = d.items[0].id
    d = updateItem(d, id, { x: 300, y: -200 })
    expect(centerItem(d, id, 'x').items[0]).toMatchObject({ x: 0, y: -200 })
    expect(centerItem(d, id, 'y').items[0]).toMatchObject({ x: 300, y: 0 })
    expect(centerItem(d, id, 'both').items[0]).toMatchObject({ x: 0, y: 0 })
  })
})

describe('複製、刪除、層次', () => {
  it('複製：內容相同、位置錯開、新編號，並選到新的', () => {
    let d = addShape(emptyDoc(hoop.id), 'star')
    const src = d.items[0]
    d = duplicateItem(d, src.id)
    expect(d.items).toHaveLength(2)
    const copy = d.items[1]
    expect(copy.id).not.toBe(src.id)
    expect(copy).toMatchObject({ kind: 'shape', shape: 'star' })
    expect(`${copy.x},${copy.y}`).not.toBe(`${src.x},${src.y}`)
    expect(d.selectedId).toBe(copy.id)
  })

  it('刪除：不問，而且可以拿回原來的位置', () => {
    let d = addText(emptyDoc(hoop.id))
    d = addShape(d, 'circle')
    d = addShape(d, 'rect')
    const middle = d.items[1]
    const r = removeItem(d, middle.id)
    expect(r.doc.items).toHaveLength(2)
    expect(r.removed).toEqual({ item: middle, index: 1 })
    const back = restoreRemoved(r.doc, r.removed!)
    expect(back.items.map((i) => i.id)).toEqual(d.items.map((i) => i.id))
    expect(back.selectedId).toBe(middle.id)
  })

  it('刪掉被選到的，就沒有選取', () => {
    let d = addText(emptyDoc(hoop.id))
    d = removeItem(d, d.items[0].id).doc
    expect(d.selectedId).toBeNull()
  })

  it('上下層：往前、往後、最前、最後；到頭不動', () => {
    let d = addText(emptyDoc(hoop.id))
    d = addShape(d, 'circle')
    d = addShape(d, 'rect')
    const ids = d.items.map((i) => i.id)
    expect(moveLayer(d, ids[0], 'forward').items.map((i) => i.id)).toEqual([ids[1], ids[0], ids[2]])
    expect(moveLayer(d, ids[2], 'backward').items.map((i) => i.id)).toEqual([ids[0], ids[2], ids[1]])
    expect(moveLayer(d, ids[0], 'front').items.map((i) => i.id)).toEqual([ids[1], ids[2], ids[0]])
    expect(moveLayer(d, ids[2], 'back').items.map((i) => i.id)).toEqual([ids[2], ids[0], ids[1]])
    expect(moveLayer(d, ids[2], 'forward').items.map((i) => i.id)).toEqual(ids)
  })
})

describe('組成送給核心的請求', () => {
  const build = (d: EditorDoc, images: ImageSource[] = []) => toRequest(d, hoop, images)

  it('文字：公分換成 0.1mm、顏色用色票', () => {
    let d = addText(emptyDoc(hoop.id))
    d = updateItem(d, d.items[0].id, { text: '王小明', heightCm: 2.5, colorIdx: 8 })
    const { request } = build(d)
    expect(request.items[0]).toMatchObject({
      kind: 'text',
      text: '王小明',
      height: 250,
      color: { name: '寶藍' },
      font_id: 'sans',
    })
  })

  it('形狀：公分換成 0.1mm、粗細公釐換成 0.1mm', () => {
    let d = addShape(emptyDoc(hoop.id), 'rect')
    d = updateItem(d, d.items[0].id, { widthCm: 4, heightCm: 2, lineWidthMm: 3, filled: false })
    const it = build(d).request.items[0]
    expect(it).toMatchObject({ kind: 'shape', width: 400, height: 200, line_width: 30, filled: false })
  })

  it('位置、旋轉、縮放原樣帶過去', () => {
    let d = addShape(emptyDoc(hoop.id), 'circle')
    d = updateItem(d, d.items[0].id, { x: 150, y: -70, rotation: 30, scale: 1.5 })
    expect(build(d).request.items[0]).toMatchObject({ x: 150, y: -70, rotation: 30, scale: 1.5 })
  })

  it('圖片：同一張圖用到兩次，只傳一份像素', () => {
    const a = img('a')
    let d = addImage(emptyDoc(hoop.id), a, hoop)
    d = duplicateItem(d, d.items[0].id)
    const { request, images } = build(d, [a])
    expect(images).toHaveLength(1)
    expect(request.items.map((i) => (i.kind === 'image' ? i.image : -1))).toEqual([0, 0])
  })

  it('圖片：有圈範圍就只傳框內的像素，寬高也跟著變', () => {
    const a = img('a')
    let d = addImage(emptyDoc(hoop.id), a, hoop)
    d = updateItem(d, d.items[0].id, { crop: { x0: 0, y0: 0, x1: 0.5, y1: 1 } })
    const { request, images } = build(d, [a])
    const it = request.items[0]
    if (it.kind !== 'image') throw new Error('應是圖片')
    expect(it.width).toBe(20)
    expect(it.height).toBe(20)
    expect(images[0].length).toBe(20 * 20 * 4)
  })

  it('找不到像素的圖片物件會被略過，不會讓整個設計壞掉', () => {
    const a = img('a')
    const d = addImage(emptyDoc(hoop.id), a, hoop)
    expect(build(d, []).request.items).toHaveLength(0)
  })

  it('文字空白的物件仍送過去（核心會回報這個物件的問題），但標出是空的', () => {
    const d = addText(emptyDoc(hoop.id))
    expect(build(d).request.items).toHaveLength(1)
  })
})

import { fromLegacy } from '../src/lib/editor'

describe('舊版存下來的設計', () => {
  it('文字設計變成一個文字物件', () => {
    const it = fromLegacy({ mode: 'text', text: '王小明', fontId: 'serif', sizeCm: 3, colorIdx: 5 })
    expect(it).toMatchObject({ kind: 'text', text: '王小明', fontId: 'serif', heightCm: 3, colorIdx: 5, x: 0, y: 0 })
  })

  it('圖片設計變成一個圖片物件，需要指定圖片編號', () => {
    const it = fromLegacy({ mode: 'image', colors: 6, simplify: 40, imageSizeCm: 9 }, 'img-1')
    expect(it).toMatchObject({ kind: 'image', imageId: 'img-1', colors: 6, simplify: 40, sizeCm: 9 })
  })

  it('沒有圖片編號的圖片設計、看不懂的內容回傳 null', () => {
    expect(fromLegacy({ mode: 'image' })).toBeNull()
    expect(fromLegacy({ mode: 'other' })).toBeNull()
  })
})

import { appendStroke, removeLastStroke, startDraw, type DrawItem } from '../src/lib/editor'

const brush = { widthMm: 2.5, colorIdx: 3, filled: false }
const world = (it: DrawItem, p: [number, number]): [number, number] => {
  const r = (it.rotation * Math.PI) / 180
  const [x, y] = [p[0] * it.scale, p[1] * it.scale]
  return [it.x + x * Math.cos(r) - y * Math.sin(r), it.y + x * Math.sin(r) + y * Math.cos(r)]
}

describe('手繪', () => {
  it('第一筆：物件的中心放在這一筆的正中間，點是相對於中心的', () => {
    const d = startDraw(emptyDoc(hoop.id), [{ x: 100, y: 200 }, { x: 300, y: 200 }], brush, 1)
    const it = d.items[0] as DrawItem
    expect(it.kind).toBe('draw')
    expect([it.x, it.y]).toEqual([200, 200])
    expect(it.strokes[0]).toEqual([[-100, 0], [100, 0]])
    expect(it).toMatchObject({ widthMm: 2.5, colorIdx: 3, filled: false })
    expect(d.selectedId).toBe(it.id)
  })

  it('畫面上整體縮小過（fit<1）：位置要換回設計座標', () => {
    const d = startDraw(emptyDoc(hoop.id), [{ x: 50, y: 50 }, { x: 150, y: 50 }], brush, 0.5)
    const it = d.items[0] as DrawItem
    expect([it.x, it.y]).toEqual([200, 100])
    expect(it.strokes[0]).toEqual([[-100, 0], [100, 0]])
  })

  it('再畫一筆：所有點在畫面上的位置都不變（即使物件轉過、縮放過）', () => {
    let d = startDraw(emptyDoc(hoop.id), [{ x: 0, y: 0 }, { x: 100, y: 0 }], brush, 1)
    const id = d.items[0].id
    d = updateItem(d, id, { rotation: 30, scale: 1.5 })
    const before = d.items[0] as DrawItem
    const firstWorld = before.strokes[0].map((p) => world(before, p))
    // 畫面上的新一筆（已套用物件的位置與旋轉、縮放）
    const input = [{ x: 400, y: -100 }, { x: 500, y: 50 }]
    d = appendStroke(d, id, input, 1)
    const it = d.items[0] as DrawItem
    expect(it.strokes).toHaveLength(2)
    it.strokes[1].forEach((p, i) => {
      const w = world(it, p)
      expect(w[0]).toBeCloseTo(input[i].x, 0)
      expect(w[1]).toBeCloseTo(input[i].y, 0)
    })
    it.strokes[0].forEach((p, i) => {
      const w = world(it, p)
      expect(w[0]).toBeCloseTo(firstWorld[i][0], 0)
      expect(w[1]).toBeCloseTo(firstWorld[i][1], 0)
    })
  })

  it('刪掉最後一筆；沒有筆畫了就整個物件拿掉', () => {
    let d = startDraw(emptyDoc(hoop.id), [{ x: 0, y: 0 }, { x: 10, y: 0 }], brush, 1)
    const id = d.items[0].id
    d = appendStroke(d, id, [{ x: 50, y: 50 }, { x: 80, y: 50 }], 1)
    d = removeLastStroke(d, id)
    expect((d.items[0] as DrawItem).strokes).toHaveLength(1)
    d = removeLastStroke(d, id)
    expect(d.items).toHaveLength(0)
    expect(d.selectedId).toBeNull()
  })

  it('組成請求：粗細公釐換成 0.1mm，筆畫原樣帶過去', () => {
    const d = startDraw(emptyDoc(hoop.id), [{ x: 0, y: 0 }, { x: 100, y: 0 }], { ...brush, filled: true }, 1)
    const it = toRequest(d, hoop, []).request.items[0]
    expect(it).toMatchObject({ kind: 'draw', width: 25, filled: true, color: { name: THREADS_NAME_3 } })
    if (it.kind !== 'draw') throw new Error('應是手繪')
    expect(it.strokes).toHaveLength(1)
  })
})

import { THREADS } from '../src/lib/threads'
const THREADS_NAME_3 = THREADS[3].name

import { addFrame, flipItem, mirrorCopy, repeatCircle, repeatRow } from '../src/lib/editor'

const docWith = (kind: 'circle' | 'rect' = 'rect', at = { x: 0, y: 0 }) => {
  let d = addShape(emptyDoc(hoop.id), kind)
  d = updateItem(d, d.items[0].id, at)
  return d
}

describe('整排重複', () => {
  it('橫排：份數正確、間距相等、整排以原本的位置為中心', () => {
    const d = docWith('rect', { x: 100, y: 50 })
    const r = repeatRow(d, d.items[0].id, { count: 4, gap: 20, axis: 'x', size: { width: 80, height: 40 } })
    expect(r.items).toHaveLength(4)
    const xs = r.items.map((i) => i.x)
    expect(xs.map((x, i) => (i ? x - xs[i - 1] : 100))).toEqual([100, 100, 100, 100])
    expect((xs[0] + xs[3]) / 2).toBeCloseTo(100)
    expect(new Set(r.items.map((i) => i.y))).toEqual(new Set([50]))
  })

  it('直排用高度算間距', () => {
    const d = docWith()
    const r = repeatRow(d, d.items[0].id, { count: 3, gap: 10, axis: 'y', size: { width: 80, height: 40 } })
    const ys = r.items.map((i) => i.y)
    expect(ys[1] - ys[0]).toBe(50)
    expect(new Set(r.items.map((i) => i.x)).size).toBe(1)
  })

  it('物件轉了 90 度：橫排改用轉過的寬度（原本的高）算間距', () => {
    let d = docWith()
    d = updateItem(d, d.items[0].id, { rotation: 90 })
    const r = repeatRow(d, d.items[0].id, { count: 2, gap: 0, axis: 'x', size: { width: 80, height: 40 } })
    expect(Math.abs(r.items[1].x - r.items[0].x)).toBeCloseTo(40)
  })

  it('份數限制在 2 到 24；新的物件編號都不同', () => {
    const d = docWith()
    expect(repeatRow(d, d.items[0].id, { count: 99, gap: 0, axis: 'x', size: { width: 10, height: 10 } }).items).toHaveLength(24)
    expect(repeatRow(d, d.items[0].id, { count: 1, gap: 0, axis: 'x', size: { width: 10, height: 10 } }).items).toHaveLength(2)
    const r = repeatRow(d, d.items[0].id, { count: 5, gap: 0, axis: 'x', size: { width: 10, height: 10 } })
    expect(new Set(r.items.map((i) => i.id)).size).toBe(5)
  })
})

describe('圍成一圈', () => {
  it('每一個離圓心一樣遠，第一個在正上方，依序順時針、並朝外轉', () => {
    const d = docWith('rect', { x: 20, y: 30 })
    const r = repeatCircle(d, d.items[0].id, { count: 4, radius: 200 })
    expect(r.items).toHaveLength(4)
    const [top, right, bottom, left] = r.items
    for (const it of r.items) expect(Math.hypot(it.x - 20, it.y - 30)).toBeCloseTo(200)
    expect([top.x, top.y]).toEqual([20, -170])
    expect(right.x).toBeCloseTo(220)
    expect(bottom.y).toBeCloseTo(230)
    expect(left.x).toBeCloseTo(-180)
    expect(r.items.map((i) => i.rotation)).toEqual([0, 90, 180, -90])
  })
})

describe('鏡像', () => {
  it('左右對稱複製：位置在另一邊、旋轉反向、翻轉', () => {
    let d = docWith('heart' as 'rect', { x: 150, y: 40 })
    d = updateItem(d, d.items[0].id, { rotation: 20 })
    const r = mirrorCopy(d, d.items[0].id, 'x')
    expect(r.items).toHaveLength(2)
    expect(r.items[1]).toMatchObject({ x: -150, y: 40, rotation: -20, flipX: true, flipY: false })
    expect(r.selectedId).toBe(r.items[1].id)
  })

  it('上下對稱複製', () => {
    const d = docWith('rect', { x: 10, y: 120 })
    const r = mirrorCopy(d, d.items[0].id, 'y')
    expect(r.items[1]).toMatchObject({ x: 10, y: -120, flipY: true, flipX: false })
  })

  it('翻轉自己：再翻一次就回來', () => {
    let d = docWith()
    const id = d.items[0].id
    d = flipItem(d, id, 'x')
    expect(d.items[0].flipX).toBe(true)
    d = flipItem(d, id, 'x')
    expect(d.items[0].flipX).toBe(false)
  })

  it('翻轉帶進請求', () => {
    let d = docWith()
    d = flipItem(d, d.items[0].id, 'y')
    expect(toRequest(d, hoop, []).request.items[0]).toMatchObject({ flip_x: false, flip_y: true })
  })
})

describe('邊框', () => {
  it('方框：空心、放滿框的可繡範圍（留一點邊）', () => {
    const d = addFrame(emptyDoc(hoop.id), 'rect', hoop)
    const it = d.items[0]
    if (it.kind !== 'shape') throw new Error('應是形狀')
    expect(it).toMatchObject({ shape: 'rect', filled: false, x: 0, y: 0 })
    expect(it.widthCm).toBeCloseTo((hoop.width - 2 * hoop.margin) / 100 - 1)
    expect(it.heightCm).toBeCloseTo((hoop.height - 2 * hoop.margin) / 100 - 1)
  })

  it('圓框：橢圓或圓都用同樣的寬高', () => {
    const d = addFrame(emptyDoc(hoop.id), 'circle', hoop)
    expect(d.items[0]).toMatchObject({ kind: 'shape', shape: 'circle', filled: false })
  })
})

describe('弧形文字', () => {
  it('文字預設不彎，彎度帶進請求', () => {
    let d = addText(emptyDoc(hoop.id))
    d = updateItem(d, d.items[0].id, { text: '字' })
    expect(toRequest(d, hoop, []).request.items[0]).toMatchObject({ kind: 'text', curve: 0 })
    d = updateItem(d, d.items[0].id, { curve: 90 } as Partial<EditorItem>)
    expect(toRequest(d, hoop, []).request.items[0]).toMatchObject({ curve: 90 })
  })
})

import { moveBy, ungroup } from '../src/lib/editor'

describe('一組一起動', () => {
  const row = () => {
    const d = docWith('rect', { x: 0, y: 0 })
    return repeatRow(d, d.items[0].id, { count: 3, gap: 0, axis: 'x', size: { width: 100, height: 50 } })
  }

  it('做一排或圍成一圈產生的，屬於同一組；單獨的物件沒有組', () => {
    const r = row()
    const g = new Set(r.items.map((i) => i.groupId))
    expect(g.size).toBe(1)
    expect([...g][0]).toBeTruthy()
    expect(docWith().items[0].groupId).toBeNull()
    const c = repeatCircle(docWith(), docWith().items[0].id, { count: 4, radius: 100 })
    expect(new Set(c.items.map((i) => i.groupId)).size).toBe(1)
  })

  it('移動其中一個，整組跟著移動同樣的距離', () => {
    const r = row()
    const before = r.items.map((i) => [i.x, i.y])
    const moved = moveBy(r, r.items[1].id, 30, -20, false)
    moved.items.forEach((it, i) => {
      expect([it.x, it.y]).toEqual([before[i][0] + 30, before[i][1] - 20])
    })
  })

  it('選擇「只動這一個」時，只有它動', () => {
    const r = row()
    const moved = moveBy(r, r.items[0].id, 10, 10, true)
    expect(moved.items[0].x).toBe(r.items[0].x + 10)
    expect(moved.items[1].x).toBe(r.items[1].x)
  })

  it('沒有組的物件照常移動', () => {
    const d = docWith('rect', { x: 5, y: 5 })
    expect(moveBy(d, d.items[0].id, 1, 2, false).items[0]).toMatchObject({ x: 6, y: 7 })
  })

  it('拆開之後各自獨立', () => {
    const r = ungroup(row(), row().items[0].id)
    expect(r.items.every((i) => i.groupId === null)).toBe(true)
    const moved = moveBy(r, r.items[0].id, 50, 0, false)
    expect(moved.items[1].x).toBe(r.items[1].x)
  })

  it('舊的存檔沒有組的欄位，也當成沒有組', () => {
    const d = docWith('rect', { x: 0, y: 0 })
    const legacy = { ...d, items: d.items.map((i) => ({ ...i, groupId: undefined })) } as unknown as EditorDoc
    expect(moveBy(legacy, legacy.items[0].id, 3, 0, false).items[0].x).toBe(3)
  })
})

describe('複製與鏡像不會把新的物件塞進原本那一組', () => {
  it('複製、鏡像複製出來的是單獨的', () => {
    const d = docWith('rect', { x: 50, y: 0 })
    const r = repeatRow(d, d.items[0].id, { count: 2, gap: 0, axis: 'x', size: { width: 10, height: 10 } })
    const id = r.items[0].id
    expect(duplicateItem(r, id).items.find((i) => i.id === duplicateItem(r, id).selectedId)?.groupId).toBeNull()
    expect(mirrorCopy(r, id, 'x').items.find((i) => i.id === mirrorCopy(r, id, 'x').selectedId)?.groupId).toBeNull()
  })
})

import { transformGroup } from '../src/lib/editor'

describe('整組一起縮放、旋轉', () => {
  const ring = () => {
    const d = docWith('rect', { x: 0, y: 0 })
    return repeatCircle(d, d.items[0].id, { count: 4, radius: 100 })
  }

  it('放大兩倍：每個物件變大兩倍，離組中心也遠兩倍', () => {
    const r = ring()
    const t = transformGroup(r, r.items[0].id, { factor: 2, turn: 0 })
    t.items.forEach((it, i) => {
      expect(it.scale).toBeCloseTo(r.items[i].scale * 2)
      expect(it.x).toBeCloseTo(r.items[i].x * 2)
      expect(it.y).toBeCloseTo(r.items[i].y * 2)
    })
  })

  it('轉 90 度：位置繞組中心轉，每個物件自己也多轉 90 度', () => {
    const r = ring()
    const t = transformGroup(r, r.items[0].id, { factor: 1, turn: 90 })
    // 原本在正上方（0,-100）的，順時針轉 90 度到正右方（100,0）
    expect(t.items[0].x).toBeCloseTo(100)
    expect(t.items[0].y).toBeCloseTo(0)
    expect(t.items[0].rotation).toBeCloseTo(90)
  })

  it('沒有組的物件：只改自己，位置不變', () => {
    const d = docWith('rect', { x: 40, y: 10 })
    const t = transformGroup(d, d.items[0].id, { factor: 2, turn: 30 })
    expect(t.items[0]).toMatchObject({ x: 40, y: 10, scale: 2, rotation: 30 })
  })

  it('組裡的物件放大受上下限保護', () => {
    const r = ring()
    const t = transformGroup(r, r.items[0].id, { factor: 1000, turn: 0 })
    expect(t.items.every((i) => i.scale <= 20)).toBe(true)
  })
})

import { toggleLock } from '../src/lib/editor'

describe('鎖定', () => {
  it('預設沒鎖；鎖了再按就解開', () => {
    let d = docWith()
    expect(d.items[0].locked).toBe(false)
    d = toggleLock(d, d.items[0].id)
    expect(d.items[0].locked).toBe(true)
    d = toggleLock(d, d.items[0].id)
    expect(d.items[0].locked).toBe(false)
  })

  it('鎖住的東西，移動、縮放、旋轉都沒有效果', () => {
    let d = docWith('rect', { x: 10, y: 20 })
    const id = d.items[0].id
    d = toggleLock(d, id)
    const moved = moveBy(d, id, 50, 50, false)
    expect([moved.items[0].x, moved.items[0].y]).toEqual([10, 20])
    const t = transformGroup(d, id, { factor: 2, turn: 45 })
    expect(t.items[0]).toMatchObject({ scale: 1, rotation: 0, x: 10, y: 20 })
    const u = updateItem(d, id, { scale: 3, rotation: 90, x: 99 })
    expect(u.items[0]).toMatchObject({ scale: 1, rotation: 0, x: 10 })
  })

  it('鎖住的東西仍然可以改顏色、文字、解鎖、刪除、複製', () => {
    let d = addText(emptyDoc(hoop.id))
    const id = d.items[0].id
    d = toggleLock(d, id)
    d = updateItem(d, id, { text: '王小明', colorIdx: 4 } as Partial<EditorItem>)
    expect(d.items[0]).toMatchObject({ text: '王小明', colorIdx: 4 })
    expect(duplicateItem(d, id).items).toHaveLength(2)
    expect(removeItem(d, id).doc.items).toHaveLength(0)
  })

  it('一組裡有鎖住的：整組移動時，鎖住的那個不動', () => {
    const d0 = docWith('rect', { x: 0, y: 0 })
    let d = repeatRow(d0, d0.items[0].id, { count: 3, gap: 0, axis: 'x', size: { width: 100, height: 50 } })
    d = toggleLock(d, d.items[1].id)
    const before = d.items.map((i) => i.x)
    const moved = moveBy(d, d.items[0].id, 40, 0, false)
    expect(moved.items.map((i) => i.x)).toEqual([before[0] + 40, before[1], before[2] + 40])
  })

  it('複製出來的新物件不會是鎖住的', () => {
    let d = docWith()
    d = toggleLock(d, d.items[0].id)
    const copy = duplicateItem(d, d.items[0].id)
    expect(copy.items.at(-1)?.locked).toBe(false)
  })

  it('舊的存檔沒有 locked 欄位，也當成沒鎖', () => {
    const d = docWith()
    const legacy = { ...d, items: d.items.map((i) => ({ ...i, locked: undefined })) } as unknown as EditorDoc
    expect(moveBy(legacy, legacy.items[0].id, 3, 0, false).items[0].x).toBe(3)
  })
})

import {
  alignItems,
  distributeItems,
  matchSize,
  removeSelected,
  selectedItems,
  selectItem,
  toggleSelect,
} from '../src/lib/editor'

describe('多選', () => {
  const three = () => {
    let d = emptyDoc(hoop.id)
    for (const x of [0, 100, 400]) {
      d = addShape(d, 'rect')
      d = updateItem(d, d.selectedId as string, { x, y: x / 2 })
    }
    return d
  }
  const sizes = new Map<string, { width: number; height: number }>()
  const sizeOf = (id: string) => sizes.get(id) ?? { width: 100, height: 60 }

  it('加選、再按一下取消；選到的都在 selectedItems 裡', () => {
    let d = three()
    const [a, b, c] = d.items.map((i) => i.id)
    d = { ...d, selectedId: a, extraIds: [] }
    d = toggleSelect(d, b)
    d = toggleSelect(d, c)
    expect(selectedItems(d).map((i) => i.id).sort()).toEqual([a, b, c].sort())
    d = toggleSelect(d, b)
    expect(selectedItems(d).map((i) => i.id).sort()).toEqual([a, c].sort())
  })

  it('一般的單選（selectItem）會清掉加選', () => {
    let d = three()
    const [a, b] = d.items.map((i) => i.id)
    d = { ...d, selectedId: a, extraIds: [b] }
    d = selectItem(d, b)
    expect(selectedItems(d).map((i) => i.id)).toEqual([b])
  })

  it('一起刪掉：每一個都可以拿回來', () => {
    let d = three()
    const [a, b] = d.items.map((i) => i.id)
    d = { ...d, selectedId: a, extraIds: [b] }
    const r = removeSelected(d)
    expect(r.doc.items).toHaveLength(1)
    expect(r.removed).toHaveLength(2)
    expect(r.doc.selectedId).toBeNull()
  })

  it('靠左對齊：每個的左邊緣都和最左邊的一樣', () => {
    let d = three()
    d = { ...d, selectedId: d.items[0].id, extraIds: [d.items[1].id, d.items[2].id] }
    const r = alignItems(d, 'left', sizeOf)
    const lefts = r.items.map((i) => i.x - 50)
    expect(new Set(lefts).size).toBe(1)
    expect(lefts[0]).toBe(-50)
  })

  it('靠右、靠上、靠下、水平置中、垂直置中', () => {
    let d = three()
    d = { ...d, selectedId: d.items[0].id, extraIds: [d.items[1].id, d.items[2].id] }
    expect(new Set(alignItems(d, 'right', sizeOf).items.map((i) => i.x + 50)).size).toBe(1)
    expect(new Set(alignItems(d, 'top', sizeOf).items.map((i) => i.y - 30)).size).toBe(1)
    expect(new Set(alignItems(d, 'bottom', sizeOf).items.map((i) => i.y + 30)).size).toBe(1)
    expect(new Set(alignItems(d, 'centerX', sizeOf).items.map((i) => i.x)).size).toBe(1)
    expect(new Set(alignItems(d, 'middleY', sizeOf).items.map((i) => i.y)).size).toBe(1)
  })

  it('只選一個時對齊不做事（至少要兩個）', () => {
    const d = three()
    expect(alignItems(d, 'left', sizeOf)).toBe(d)
  })

  it('等距：左右兩端不動，中間的平均分開', () => {
    let d = three() // x = 0, 100, 400
    d = { ...d, selectedId: d.items[0].id, extraIds: [d.items[1].id, d.items[2].id] }
    const r = distributeItems(d, 'x')
    const xs = r.items.map((i) => i.x)
    expect(xs[0]).toBe(0)
    expect(xs[2]).toBe(400)
    expect(xs[1]).toBeCloseTo(200)
  })

  it('等距要三個以上；兩個時不做事', () => {
    let d = three()
    d = { ...d, selectedId: d.items[0].id, extraIds: [d.items[1].id] }
    expect(distributeItems(d, 'x')).toBe(d)
  })

  it('同樣大小：全部換成第一個選到的大小', () => {
    let d = three()
    d = updateItem(d, d.items[0].id, { scale: 2 })
    d = { ...d, selectedId: d.items[0].id, extraIds: [d.items[1].id, d.items[2].id] }
    const r = matchSize(d)
    expect(r.items.map((i) => i.scale)).toEqual([2, 2, 2])
  })

  it('鎖住的不會被對齊、等距、同樣大小動到', () => {
    let d = three()
    d = toggleLock(d, d.items[2].id)
    d = { ...d, selectedId: d.items[0].id, extraIds: [d.items[1].id, d.items[2].id] }
    const r = alignItems(d, 'left', sizeOf)
    expect(r.items[2].x).toBe(400)
  })
})

import { groupSelected, moveSelected } from '../src/lib/editor'

describe('多選：綁成一組、一起移動', () => {
  const two = () => {
    let d = emptyDoc(hoop.id)
    for (const x of [0, 100]) {
      d = addShape(d, 'rect')
      d = updateItem(d, d.selectedId as string, { x, y: 0 })
    }
    return { ...d, selectedId: d.items[0].id, extraIds: [d.items[1].id] }
  }

  it('把選到的綁成一組：之後拖其中一個，整組一起動', () => {
    const g = groupSelected(two())
    expect(g.items[0].groupId).toBeTruthy()
    expect(g.items[0].groupId).toBe(g.items[1].groupId)
    const moved = moveBy(g, g.items[0].id, 10, 0, false)
    expect(moved.items.map((i) => i.x)).toEqual([10, 110])
  })

  it('只選一個時綁成一組不做事', () => {
    const d = { ...two(), extraIds: [] }
    expect(groupSelected(d)).toBe(d)
  })

  it('選到的一起移動同樣的距離，沒選到的不動，鎖住的不動', () => {
    let d: EditorDoc = two()
    d = addShape(d, 'circle')
    const third = d.items[2].id
    d = { ...d, selectedId: d.items[0].id, extraIds: [d.items[1].id] }
    const moved = moveSelected(d, 30, -20)
    expect(moved.items.map((i) => [i.x, i.y])).toEqual([[30, -20], [130, -20], [d.items[2].x, d.items[2].y]])
    expect(moved.items[2].id).toBe(third)
    const locked = moveSelected(toggleLock(d, d.items[1].id), 30, 0)
    expect(locked.items[1].x).toBe(100)
  })
})
