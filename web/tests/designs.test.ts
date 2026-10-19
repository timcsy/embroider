import 'fake-indexeddb/auto'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { clearRecent, listRecent, removeRecent, saveRecent } from '../src/lib/designs'

const text = (name: string, extra = {}) => ({
  id: `text:${name}`,
  name,
  params: { mode: 'text', text: name, fontId: 'sans', ...extra },
})

describe('最近的設計', () => {
  beforeEach(async () => {
    await clearRecent()
  })

  it('存起來之後可以列出來', async () => {
    expect(await saveRecent(text('王小明'))).toBe(true)
    const list = await listRecent()
    expect(list).toHaveLength(1)
    expect(list[0]).toMatchObject({ id: 'text:王小明', name: '王小明' })
    expect(list[0].params).toMatchObject({ text: '王小明', fontId: 'sans' })
    expect(list[0].savedAt).toBeGreaterThan(0)
  })

  it('最新的排最前面', async () => {
    await saveRecent(text('甲'), 1000)
    await saveRecent(text('乙'), 2000)
    await saveRecent(text('丙'), 3000)
    expect((await listRecent()).map((d) => d.name)).toEqual(['丙', '乙', '甲'])
  })

  it('同一個設計再存一次：更新內容並移到最前面，不會重複', async () => {
    await saveRecent(text('甲', { sizeCm: 2 }), 1000)
    await saveRecent(text('乙'), 2000)
    await saveRecent(text('甲', { sizeCm: 5 }), 3000)
    const list = await listRecent()
    expect(list.map((d) => d.name)).toEqual(['甲', '乙'])
    expect(list[0].params).toMatchObject({ sizeCm: 5 })
  })

  it('最多保留十個，最舊的被丟掉', async () => {
    for (let i = 1; i <= 13; i++) await saveRecent(text(`設計${i}`), i * 1000)
    const list = await listRecent()
    expect(list).toHaveLength(10)
    expect(list[0].name).toBe('設計13')
    expect(list.at(-1)?.name).toBe('設計4')
  })

  it('可以刪掉其中一個', async () => {
    await saveRecent(text('甲'))
    await saveRecent(text('乙'))
    await removeRecent('text:甲')
    expect((await listRecent()).map((d) => d.name)).toEqual(['乙'])
  })

  it('圖片設計連同縮小後的圖一起存', async () => {
    const png = new Uint8Array([1, 2, 3, 4])
    await saveRecent({
      id: 'image:flower',
      name: 'flower.png',
      params: { mode: 'image', colors: 5 },
      image: { name: 'flower.png', width: 64, height: 48, png },
    })
    const [d] = await listRecent()
    expect(d.image).toMatchObject({ name: 'flower.png', width: 64, height: 48 })
    expect(Array.from(d.image!.png)).toEqual([1, 2, 3, 4])
  })

  it('儲存空間壞掉時不會當機：存失敗、列出為空', async () => {
    const open = vi.spyOn(indexedDB, 'open').mockImplementation(() => {
      throw new Error('blocked')
    })
    expect(await saveRecent(text('甲'))).toBe(false)
    expect(await listRecent()).toEqual([])
    await expect(removeRecent('x')).resolves.toBeUndefined()
    open.mockRestore()
  })
})

import { clearCurrent, loadCurrent, saveCurrent } from '../src/lib/designs'

describe('做到一半的設計（自動儲存）', () => {
  beforeEach(async () => {
    await clearCurrent()
    await clearRecent()
  })

  it('存了就能讀回來，內容一樣', async () => {
    const draft = {
      params: { version: 2, items: [{ id: 'item-1', kind: 'text', text: '王小明' }], hoopId: 'b70-medium' },
      images: [{ id: 'img-1', name: 'a.png', width: 2, height: 2, png: new Uint8Array([1, 2, 3]) }],
    }
    expect(await saveCurrent(draft)).toBe(true)
    const back = await loadCurrent()
    expect(back?.params).toEqual(draft.params)
    expect(Array.from(back!.images[0].png)).toEqual([1, 2, 3])
  })

  it('再存一次是覆蓋，不是新增', async () => {
    await saveCurrent({ params: { n: 1 }, images: [] })
    await saveCurrent({ params: { n: 2 }, images: [] })
    expect((await loadCurrent())?.params).toEqual({ n: 2 })
  })

  it('清掉之後讀不到；沒存過也不會出錯', async () => {
    expect(await loadCurrent()).toBeNull()
    await saveCurrent({ params: {}, images: [] })
    await clearCurrent()
    expect(await loadCurrent()).toBeNull()
  })

  it('不會跑進「最近做過的」清單', async () => {
    await saveCurrent({ params: { a: 1 }, images: [] })
    expect(await listRecent()).toEqual([])
  })
})
