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
