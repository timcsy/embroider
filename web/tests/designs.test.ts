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
