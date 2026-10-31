import { describe, expect, it } from 'vitest'
import { backupFileName, makeBackup, parseBackup } from '../src/lib/backup'

const draft = {
  params: { version: 2, items: [{ id: 'item-1', kind: 'text', text: '王小明' }], hoopId: 'b70-medium' },
  images: [{ id: 'img-1', name: 'a.png', width: 2, height: 2, png: new Uint8Array([0, 1, 2, 255]) }],
}

describe('設計備份', () => {
  it('存出再讀回，內容完全一樣（含圖片位元組與中文）', () => {
    const r = parseBackup(makeBackup(draft))
    expect(r.ok).toBe(true)
    if (!r.ok) return
    expect(r.draft.params).toEqual(draft.params)
    expect(Array.from(r.draft.images[0].png)).toEqual([0, 1, 2, 255])
    expect(r.draft.images[0]).toMatchObject({ id: 'img-1', name: 'a.png', width: 2, height: 2 })
  })

  it('沒有圖片也可以', () => {
    const r = parseBackup(makeBackup({ ...draft, images: [] }))
    expect(r.ok && r.draft.images).toEqual([])
  })

  it('檔案是人看得懂的文字（JSON），之後格式改了也讀得出版本', () => {
    const text = new TextDecoder().decode(makeBackup(draft))
    expect(JSON.parse(text).app).toBe('embroider')
    expect(JSON.parse(text).format).toBe(1)
  })

  it('讀到不是備份的檔案：白話說明，不當機', () => {
    for (const junk of ['hello', '{"a":1}', '', '[]', '{"app":"embroider","format":99}']) {
      const r = parseBackup(new TextEncoder().encode(junk))
      expect(r.ok).toBe(false)
      if (!r.ok) expect(r.message.text).toMatch(/[一-鿿]/)
    }
  })

  it('壞掉的內容（缺欄位）也擋下', () => {
    const bad = JSON.stringify({ app: 'embroider', format: 1, params: 5, images: 'x' })
    expect(parseBackup(new TextEncoder().encode(bad)).ok).toBe(false)
  })

  it('檔名：用設計名稱，去掉不能當檔名的字，結尾 .json', () => {
    expect(backupFileName('王小明', new Date('2026-10-31T08:00:00Z'))).toBe('刺繡備份-王小明-2026-10-31.json')
    expect(backupFileName('a/b:c?', new Date('2026-10-31T08:00:00Z'))).toBe('刺繡備份-abc-2026-10-31.json')
    expect(backupFileName('', new Date('2026-10-31T08:00:00Z'))).toBe('刺繡備份-2026-10-31.json')
  })
})
