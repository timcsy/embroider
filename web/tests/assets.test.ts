import { describe, expect, it } from 'vitest'
import { fontsBaseUrl } from '../src/lib/assets'

describe('字型網址', () => {
  it('相對基準路徑以頁面網址為準，放在子路徑也對', () => {
    expect(fontsBaseUrl('./', 'https://x.github.io/embroider/index.html')).toBe(
      'https://x.github.io/embroider/fonts/',
    )
  })
  it('頁面在根目錄', () => {
    expect(fontsBaseUrl('./', 'http://localhost:4173/')).toBe('http://localhost:4173/fonts/')
  })
  it('開發模式的絕對基準路徑', () => {
    expect(fontsBaseUrl('/', 'http://localhost:5173/')).toBe('http://localhost:5173/fonts/')
  })
})
