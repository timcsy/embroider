import { describe, expect, it } from 'vitest'
import { TEMPLATES, applyTemplate } from '../src/lib/templates'
import { emptyDoc, toRequest, type EditorDoc } from '../src/lib/editor'
import type { Hoop } from '../src/lib/types'

const rect: Hoop = { id: 'm', name: '中框', width: 1200, height: 1800, shape: 'rect', margin: 50, builtin: true }
const round: Hoop = { id: 'r', name: '圓框', width: 1000, height: 1000, shape: 'circle', margin: 50, builtin: false }
const small: Hoop = { id: 's', name: '小框', width: 500, height: 750, shape: 'rect', margin: 50, builtin: true }

describe('範本', () => {
  it('有三個：名字標籤、圓形標籤、花邊，每個都有給人看的名字與說明', () => {
    expect(TEMPLATES.map((t) => t.id)).toEqual(['name-tag', 'round-tag', 'border'])
    for (const t of TEMPLATES) {
      expect(t.name).toMatch(/[一-鿿]/)
      expect(t.description).toMatch(/[一-鿿]/)
    }
  })

  for (const t of TEMPLATES) {
    it(`${t.name}：套用後有物件、被選到第一個文字（讓她直接打字）、都在框的安全範圍內`, () => {
      for (const h of [rect, round, small]) {
        const d = applyTemplate(emptyDoc(h.id), t.id, h)
        expect(d.items.length).toBeGreaterThan(0)
        const firstText = d.items.find((i) => i.kind === 'text')
        if (firstText) expect(d.selectedId).toBe(firstText.id)
        const hw = (h.width - 2 * h.margin) / 2
        const hh = (h.height - 2 * h.margin) / 2
        for (const it of d.items) {
          expect(Math.abs(it.x), `${t.id} x`).toBeLessThanOrEqual(hw)
          expect(Math.abs(it.y), `${t.id} y`).toBeLessThanOrEqual(hh)
        }
      }
    })
  }

  it('套用後能組成合法的請求', () => {
    for (const t of TEMPLATES) {
      const d = applyTemplate(emptyDoc(rect.id), t.id, rect)
      const { request } = toRequest(d, rect, [])
      expect(request.items.length).toBe(d.items.length)
    }
  })

  it('名字標籤：名字大、下面有一行小字（日期等），都置中', () => {
    const d = applyTemplate(emptyDoc(rect.id), 'name-tag', rect)
    const texts = d.items.filter((i) => i.kind === 'text')
    expect(texts.length).toBe(2)
    expect(texts[0].x).toBe(0)
    expect(texts[1].x).toBe(0)
    expect(texts[1].y).toBeGreaterThan(texts[0].y)
    if (texts[0].kind === 'text' && texts[1].kind === 'text') expect(texts[0].heightCm).toBeGreaterThan(texts[1].heightCm)
  })

  it('圓形標籤：一個圓框加一行繞成弧形的字', () => {
    const d = applyTemplate(emptyDoc(rect.id), 'round-tag', rect)
    expect(d.items.some((i) => i.kind === 'shape' && i.shape === 'circle' && !i.filled)).toBe(true)
    expect(d.items.some((i) => i.kind === 'text' && i.curve !== 0)).toBe(true)
  })

  it('花邊：一排小圖案沿著框的上緣，是同一組（可以一起移動）', () => {
    const d = applyTemplate(emptyDoc(rect.id), 'border', rect)
    const shapes = d.items.filter((i) => i.kind === 'shape')
    expect(shapes.length).toBeGreaterThanOrEqual(5)
    expect(new Set(shapes.map((s) => s.groupId)).size).toBe(1)
    expect(shapes[0].groupId).toBeTruthy()
    expect(new Set(shapes.map((s) => s.y)).size).toBe(1)
  })

  it('套用範本不會蓋掉已經做的東西：新的接在後面，編號不重複', () => {
    const base = applyTemplate(emptyDoc(rect.id), 'name-tag', rect)
    const d: EditorDoc = applyTemplate(base, 'border', rect)
    expect(d.items.length).toBeGreaterThan(base.items.length)
    expect(new Set(d.items.map((i) => i.id)).size).toBe(d.items.length)
    expect(d.items.slice(0, base.items.length)).toEqual(base.items)
  })

  it('不認得的範本：原樣回傳', () => {
    const d = emptyDoc(rect.id)
    expect(applyTemplate(d, 'nope', rect)).toBe(d)
  })
})
