import { describe, expect, it } from 'vitest'
import { FORBIDDEN_TERMS, strings } from '../src/lib/strings.zh-TW'
import { THREADS } from '../src/lib/threads'

function allStrings(obj: unknown): string[] {
  if (typeof obj === 'string') return [obj]
  if (Array.isArray(obj)) return obj.flatMap(allStrings)
  if (obj && typeof obj === 'object') return Object.values(obj).flatMap(allStrings)
  return []
}

describe('存檔說明', () => {
  it('告訴使用者檔案存在手機的哪裡', () => {
    expect(strings.export.downloadHint).toContain('下載')
  })
})

describe('介面文字', () => {
  it('使用者看得到的字串不含技術術語', () => {
    for (const s of allStrings(strings)) {
      for (const term of FORBIDDEN_TERMS) {
        expect(s.toLowerCase(), `「${s}」含有 ${term}`).not.toContain(term.toLowerCase())
      }
    }
  })

  it('字串都是繁體中文為主', () => {
    const han = /[一-鿿]/
    for (const s of allStrings(strings)) {
      expect(han.test(s), `「${s}」沒有中文`).toBe(true)
    }
  })

  it('線色都有中文色名且顏色不重複', () => {
    expect(THREADS.length).toBeGreaterThanOrEqual(10)
    const seen = new Set<string>()
    for (const t of THREADS) {
      expect(t.name).toMatch(/[一-鿿]/)
      const key = `${t.r},${t.g},${t.b}`
      expect(seen.has(key)).toBe(false)
      seen.add(key)
    }
  })
})
