import { describe, expect, it } from 'vitest'
import { parseHoopForm } from '../src/lib/hoopForm'
import { strings } from '../src/lib/strings.zh-TW'

const base = { name: '10×10 方框', widthMm: '100', heightMm: '100', shape: 'rect' as const }

describe('新增框的表單', () => {
  it('公釐換成核心的 0.1mm 單位', () => {
    const r = parseHoopForm({ ...base, widthMm: '100', heightMm: '62.5' })
    expect(r).toEqual({
      ok: true,
      input: { name: '10×10 方框', width: 1000, height: 625, shape: 'rect' },
    })
  })

  it('容許前後空白與全形數字', () => {
    const r = parseHoopForm({ ...base, widthMm: ' １００ ', heightMm: '100 ' })
    expect(r.ok).toBe(true)
    if (r.ok) expect(r.input.width).toBe(1000)
  })

  it('圓形只看寬度，高度自動跟著一樣', () => {
    const r = parseHoopForm({ ...base, shape: 'circle', widthMm: '120', heightMm: '' })
    expect(r.ok).toBe(true)
    if (r.ok) expect(r.input).toMatchObject({ width: 1200, height: 1200, shape: 'circle' })
  })

  it('沒填或亂填的數字給白話提示', () => {
    for (const bad of ['', '   ', 'abc', '10x', '-', '1e', '10,5,2']) {
      const r = parseHoopForm({ ...base, widthMm: bad })
      expect(r.ok, `寬度 ${JSON.stringify(bad)}`).toBe(false)
      if (!r.ok) {
        expect(r.message.text).toMatch(/寬/)
        expect(r.message.text).toMatch(/[一-鿿]/)
      }
    }
    const h = parseHoopForm({ ...base, heightMm: 'abc' })
    expect(h.ok).toBe(false)
    if (!h.ok) expect(h.message.text).toMatch(/高/)
  })

  it('名字沒填時提示', () => {
    const r = parseHoopForm({ ...base, name: '  ' })
    expect(r.ok).toBe(false)
    if (!r.ok) expect(r.message.text).toMatch(/名字/)
  })

  it('接受逗號當小數點', () => {
    const r = parseHoopForm({ ...base, widthMm: '100,5' })
    expect(r.ok).toBe(true)
    if (r.ok) expect(r.input.width).toBe(1005)
  })

  it('有「量哪裡」的提示文字，強調量內框', () => {
    expect(strings.hoop.measureHint).toMatch(/裡面|內/)
  })
})
