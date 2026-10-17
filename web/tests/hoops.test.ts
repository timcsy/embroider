import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { beforeAll, beforeEach, describe, expect, it, vi } from 'vitest'
import { initCoreSync, listBuiltinHoops, validateHoop } from '../src/lib/core'
import { createHoopStore, mmToUnits } from '../src/lib/hoops'

beforeAll(() => {
  initCoreSync(readFileSync(resolve(process.cwd(), 'src/lib/wasm/stitch_wasm_bg.wasm')))
})

const make = () => createHoopStore(listBuiltinHoops(), validateHoop)
const mine = { name: '10×10 方框', width: mmToUnits(100), height: mmToUnits(100), shape: 'rect' as const }

describe('繡框儲存', () => {
  beforeEach(() => localStorage.clear())

  it('一開始只有三個原廠框', () => {
    const s = make()
    expect(s.list().map((h) => h.id)).toEqual(['b70-large', 'b70-medium', 'b70-small'])
  })

  it('毫米換成 0.1mm 單位', () => {
    expect(mmToUnits(100)).toBe(1000)
    expect(mmToUnits(12.5)).toBe(125)
  })

  it('新增框：用預設安全邊距，重新載入後仍在', () => {
    const s = make()
    const r = s.add(mine)
    expect(r.ok).toBe(true)
    if (!r.ok) return
    expect(r.hoop).toMatchObject({ name: '10×10 方框', width: 1000, height: 1000, margin: 50, builtin: false })
    const again = make()
    expect(again.list().map((h) => h.name)).toContain('10×10 方框')
  })

  it('新增不合理的框會回傳白話訊息，清單不變', () => {
    const s = make()
    const r = s.add({ ...mine, name: '  ' })
    expect(r.ok).toBe(false)
    if (r.ok) return
    expect(r.message.text).toMatch(/[一-鿿]/)
    expect(s.list()).toHaveLength(3)
    expect(s.add({ ...mine, width: 0 }).ok).toBe(false)
    expect(s.add({ ...mine, width: 99999 }).ok).toBe(false)
    expect(s.add({ ...mine, name: '字'.repeat(21) }).ok).toBe(false)
  })

  it('圓形的寬高要一樣', () => {
    const s = make()
    expect(s.add({ ...mine, shape: 'circle', height: 800 }).ok).toBe(false)
    expect(s.add({ ...mine, shape: 'circle' }).ok).toBe(true)
  })

  it('編輯自己的框', () => {
    const s = make()
    const r = s.add(mine)
    if (!r.ok) throw new Error('應該成功')
    const u = s.update(r.hoop.id, { name: '改名了', width: 1200 })
    expect(u.ok).toBe(true)
    expect(make().list().find((h) => h.id === r.hoop.id)).toMatchObject({ name: '改名了', width: 1200 })
  })

  it('編輯成不合理的值會被拒絕且保留原本的', () => {
    const s = make()
    const r = s.add(mine)
    if (!r.ok) throw new Error('應該成功')
    expect(s.update(r.hoop.id, { width: -5 }).ok).toBe(false)
    expect(s.list().find((h) => h.id === r.hoop.id)?.width).toBe(1000)
  })

  it('刪除自己的框', () => {
    const s = make()
    const r = s.add(mine)
    if (!r.ok) throw new Error('應該成功')
    expect(s.remove(r.hoop.id).ok).toBe(true)
    expect(make().list()).toHaveLength(3)
  })

  it('原廠框不能刪除或修改', () => {
    const s = make()
    const del = s.remove('b70-small')
    expect(del.ok).toBe(false)
    const upd = s.update('b70-small', { width: 100 })
    expect(upd.ok).toBe(false)
    expect(s.list().find((h) => h.id === 'b70-small')?.width).toBe(500)
  })

  it('找不到的框回報白話錯誤', () => {
    expect(make().remove('nope').ok).toBe(false)
    expect(make().update('nope', { name: 'x' }).ok).toBe(false)
  })

  it('刪除後再新增，編號不會重複', () => {
    const s = make()
    const a = s.add(mine)
    const b = s.add({ ...mine, name: '第二個' })
    if (!a.ok || !b.ok) throw new Error()
    s.remove(a.hoop.id)
    const c = s.add({ ...mine, name: '第三個' })
    if (!c.ok) throw new Error()
    const ids = s.list().map((h) => h.id)
    expect(new Set(ids).size).toBe(ids.length)
  })

  it('儲存的內容壞掉時退回只有原廠框，不會當機', () => {
    localStorage.setItem('embroider.hoops', '{壞掉的')
    expect(make().list()).toHaveLength(3)
    localStorage.setItem('embroider.hoops', JSON.stringify([{ id: 'x', name: 5 }]))
    expect(make().list()).toHaveLength(3)
  })

  it('儲存空間不能用時，仍然可以新增（只在這次使用中有效）', () => {
    vi.spyOn(Storage.prototype, 'setItem').mockImplementation(() => {
      throw new Error('blocked')
    })
    vi.spyOn(Storage.prototype, 'getItem').mockImplementation(() => {
      throw new Error('blocked')
    })
    const s = make()
    expect(s.add(mine).ok).toBe(true)
    expect(s.list()).toHaveLength(4)
    vi.restoreAllMocks()
  })
})
