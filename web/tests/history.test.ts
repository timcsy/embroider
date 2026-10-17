import { describe, expect, it } from 'vitest'
import { History } from '../src/lib/history'

describe('可復原的編輯歷史', () => {
  it('復原與重做', () => {
    const h = new History({ text: '' })
    h.push({ text: 'A' })
    h.push({ text: 'AB' })
    expect(h.current).toEqual({ text: 'AB' })
    expect(h.canUndo).toBe(true)
    expect(h.undo()).toEqual({ text: 'A' })
    expect(h.undo()).toEqual({ text: '' })
    expect(h.canUndo).toBe(false)
    expect(h.undo()).toEqual({ text: '' })
    expect(h.canRedo).toBe(true)
    expect(h.redo()).toEqual({ text: 'A' })
  })

  it('復原之後再編輯會丟掉重做的部分', () => {
    const h = new History({ n: 0 })
    h.push({ n: 1 })
    h.push({ n: 2 })
    h.undo()
    h.push({ n: 9 })
    expect(h.canRedo).toBe(false)
    expect(h.current).toEqual({ n: 9 })
  })

  it('連續輸入同一欄位會合併成一步', () => {
    const h = new History({ text: '' })
    h.push({ text: 'a' }, { coalesce: 'text', now: 0 })
    h.push({ text: 'ab' }, { coalesce: 'text', now: 300 })
    h.push({ text: 'abc' }, { coalesce: 'text', now: 600 })
    expect(h.undo()).toEqual({ text: '' })
  })

  it('停頓太久或換欄位就分成新的一步', () => {
    const h = new History({ text: '', size: 1 })
    h.push({ text: 'a', size: 1 }, { coalesce: 'text', now: 0 })
    h.push({ text: 'ab', size: 1 }, { coalesce: 'text', now: 5000 })
    h.push({ text: 'ab', size: 2 }, { coalesce: 'size', now: 5100 })
    expect(h.undo()).toEqual({ text: 'ab', size: 1 })
    expect(h.undo()).toEqual({ text: 'a', size: 1 })
    expect(h.undo()).toEqual({ text: '', size: 1 })
  })

  it('內容沒變就不新增一步', () => {
    const h = new History({ text: 'x' })
    h.push({ text: 'x' })
    expect(h.canUndo).toBe(false)
  })

  it('歷史有數量上限', () => {
    const h = new History({ n: 0 }, 5)
    for (let i = 1; i <= 20; i++) h.push({ n: i })
    let steps = 0
    while (h.canUndo) {
      h.undo()
      steps++
    }
    expect(steps).toBe(5)
  })
})
