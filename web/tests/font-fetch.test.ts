import { describe, expect, it, vi } from 'vitest'
import { FONT_CACHE, fetchFontBytes } from '../src/lib/fontFetch'

function fakeCaches(initial: Record<string, Uint8Array> = {}) {
  const store = new Map<string, Uint8Array>(Object.entries(initial))
  const cache = {
    match: async (url: string) => (store.has(url) ? new Response(store.get(url) as BodyInit) : undefined),
    put: async (url: string, res: Response) => void store.set(url, new Uint8Array(await res.arrayBuffer())),
  }
  const opened: string[] = []
  return {
    store,
    opened,
    caches: { open: async (name: string) => (opened.push(name), cache) } as unknown as CacheStorage,
  }
}

describe('字型下載與留存', () => {
  it('第一次從網路抓，並且存進快取（之後離線也能用）', async () => {
    const f = fakeCaches()
    const fetchFn = vi.fn(async () => new Response(new Uint8Array([7, 8, 9])))
    const bytes = await fetchFontBytes('https://x/fonts/k.ttf', { caches: f.caches, fetch: fetchFn })
    expect(Array.from(bytes)).toEqual([7, 8, 9])
    expect(fetchFn).toHaveBeenCalledTimes(1)
    expect(Array.from(f.store.get('https://x/fonts/k.ttf')!)).toEqual([7, 8, 9])
    expect(f.opened).toContain(FONT_CACHE)
  })

  it('快取裡已經有就不再下載', async () => {
    const f = fakeCaches({ 'https://x/fonts/k.ttf': new Uint8Array([1, 2]) })
    const fetchFn = vi.fn(async () => new Response(new Uint8Array([9])))
    const bytes = await fetchFontBytes('https://x/fonts/k.ttf', { caches: f.caches, fetch: fetchFn })
    expect(Array.from(bytes)).toEqual([1, 2])
    expect(fetchFn).not.toHaveBeenCalled()
  })

  it('連不上網而且快取沒有：丟出錯誤', async () => {
    const f = fakeCaches()
    const fetchFn = vi.fn(async () => {
      throw new Error('offline')
    })
    await expect(fetchFontBytes('https://x/a.ttf', { caches: f.caches, fetch: fetchFn })).rejects.toThrow()
  })

  it('伺服器回錯誤：丟出錯誤，而且不會把錯誤的回應存進快取', async () => {
    const f = fakeCaches()
    const fetchFn = vi.fn(async () => new Response('no', { status: 404 }))
    await expect(fetchFontBytes('https://x/a.ttf', { caches: f.caches, fetch: fetchFn })).rejects.toThrow()
    expect(f.store.size).toBe(0)
  })

  it('瀏覽器不能用快取（例如私密瀏覽）：照樣能下載', async () => {
    const fetchFn = vi.fn(async () => new Response(new Uint8Array([5])))
    const bytes = await fetchFontBytes('https://x/a.ttf', { caches: undefined, fetch: fetchFn })
    expect(Array.from(bytes)).toEqual([5])
  })

  it('快取壞掉（丟例外）：照樣能下載', async () => {
    const broken = { open: async () => Promise.reject(new Error('blocked')) } as unknown as CacheStorage
    const fetchFn = vi.fn(async () => new Response(new Uint8Array([6])))
    const bytes = await fetchFontBytes('https://x/a.ttf', { caches: broken, fetch: fetchFn })
    expect(Array.from(bytes)).toEqual([6])
  })
})
