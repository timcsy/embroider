import { describe, expect, it } from 'vitest'
import { fitWithin, stemOf, validateImageFile } from '../src/lib/imageInput'

const file = (name: string, type: string, size = 1000) =>
  ({ name, type, size }) as unknown as File

describe('圖片尺寸換算', () => {
  it('長邊超過上限時等比縮小', () => {
    expect(fitWithin(2000, 1000, 500)).toEqual({ width: 500, height: 250 })
    expect(fitWithin(1000, 4000, 500)).toEqual({ width: 125, height: 500 })
  })
  it('不到上限就維持原樣', () => {
    expect(fitWithin(300, 200, 500)).toEqual({ width: 300, height: 200 })
  })
  it('至少保留一個單位', () => {
    expect(fitWithin(10000, 1, 100)).toEqual({ width: 100, height: 1 })
  })
})

describe('檢查選的檔案', () => {
  it('常見圖片格式可以', () => {
    for (const [n, t] of [
      ['a.png', 'image/png'],
      ['a.jpg', 'image/jpeg'],
      ['a.webp', 'image/webp'],
      ['a.gif', 'image/gif'],
    ]) {
      expect(validateImageFile(file(n, t))).toBeNull()
    }
  })

  it('不是圖片時給白話提示', () => {
    const m = validateImageFile(file('a.pdf', 'application/pdf'))
    expect(m?.code).toBe('not_image')
    expect(m?.text).toMatch(/[一-鿿]/)
    expect(m?.hint).toBeTruthy()
  })

  it('瀏覽器打不開的格式（例如 HEIC）提示改選 JPG 或 PNG', () => {
    const m = validateImageFile(file('a.heic', 'image/heic'))
    expect(m?.code).toBe('unsupported_image')
    expect(m?.hint).toMatch(/JPG|PNG/)
  })

  it('檔案太大時提示', () => {
    expect(validateImageFile(file('a.png', 'image/png', 30 * 1024 * 1024))?.code).toBe('image_too_big')
    expect(validateImageFile(file('a.png', 'image/png', 5 * 1024 * 1024))).toBeNull()
  })

  it('沒有副檔名也沒有類型時當作不是圖片', () => {
    expect(validateImageFile(file('noname', ''))?.code).toBe('not_image')
  })
})

describe('檔名主體', () => {
  it('去掉副檔名', () => {
    expect(stemOf('flower.png')).toBe('flower')
    expect(stemOf('my.photo.jpg')).toBe('my.photo')
    expect(stemOf('noext')).toBe('noext')
    expect(stemOf('')).toBe('')
  })
})

import { dataUrlToBytes, fileFromBytes } from '../src/lib/imageInput'

describe('圖片位元組轉換', () => {
  it('data URL 還原成位元組', () => {
    // "Hello" 的 base64 是 SGVsbG8=
    const bytes = dataUrlToBytes('data:image/png;base64,SGVsbG8=')
    expect(Array.from(bytes)).toEqual([72, 101, 108, 108, 111])
  })

  it('不是 data URL 時回傳空', () => {
    expect(dataUrlToBytes('https://example.com/a.png').length).toBe(0)
    expect(dataUrlToBytes('').length).toBe(0)
  })

  it('位元組包回檔案，帶檔名與 PNG 類型', () => {
    const f = fileFromBytes(new Uint8Array([1, 2, 3]), 'flower.png')
    expect(f.name).toBe('flower.png')
    expect(f.type).toBe('image/png')
    expect(f.size).toBe(3)
  })
})
