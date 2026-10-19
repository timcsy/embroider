import { describe, expect, it, vi } from 'vitest'
import { createFontManager } from '../src/lib/fontManager'

const bytes = new Uint8Array([1, 2, 3])

function setup(opts: { fetchOk?: boolean; registerOk?: boolean; preloaded?: string[] } = {}) {
  const fetchFont = vi.fn(async () => {
    if (opts.fetchOk === false) throw new Error('offline')
    return bytes
  })
  const registerFont = vi.fn(async () => ({ ok: opts.registerOk !== false, message: null }))
  const fm = createFontManager({
    fonts: [
      { id: 'sans', file: 'a.ttf', preload: true },
      { id: 'kai', file: 'k.ttf', preload: false },
      { id: 'serif', file: 's.ttf', preload: false },
    ],
    fetchFont,
    registerFont,
    preloaded: opts.preloaded ?? ['sans'],
  })
  return { fm, fetchFont, registerFont }
}

describe('字型按需下載', () => {
  it('基本字型一開始就可以用；其他的要下載', () => {
    const { fm } = setup()
    expect(fm.status('sans')).toBe('ready')
    expect(fm.status('kai')).toBe('idle')
    expect(fm.isReady('sans')).toBe(true)
    expect(fm.isReady('kai')).toBe(false)
  })

  it('用的時候才下載，下載好就是 ready，再要一次不會重抓', async () => {
    const { fm, fetchFont, registerFont } = setup()
    expect(await fm.ensure('kai')).toBe(true)
    expect(fm.status('kai')).toBe('ready')
    expect(fetchFont).toHaveBeenCalledTimes(1)
    expect(registerFont).toHaveBeenCalledWith('kai', 'kai', bytes)
    expect(await fm.ensure('kai')).toBe(true)
    expect(fetchFont).toHaveBeenCalledTimes(1)
  })

  it('同時要同一個字型很多次，只下載一次', async () => {
    const { fm, fetchFont } = setup()
    const r = await Promise.all([fm.ensure('kai'), fm.ensure('kai'), fm.ensure('kai')])
    expect(r).toEqual([true, true, true])
    expect(fetchFont).toHaveBeenCalledTimes(1)
  })

  it('下載中的狀態：loading', async () => {
    const { fm } = setup()
    const p = fm.ensure('serif')
    expect(fm.status('serif')).toBe('loading')
    await p
    expect(fm.status('serif')).toBe('ready')
  })

  it('連不上網：回傳 false、狀態 failed，之後有網路可以再試', async () => {
    let online = false
    const fetchFont = vi.fn(async () => {
      if (!online) throw new Error('offline')
      return bytes
    })
    const fm = createFontManager({
      fonts: [{ id: 'kai', file: 'k.ttf', preload: false }],
      fetchFont,
      registerFont: async () => ({ ok: true, message: null }),
      preloaded: [],
    })
    expect(await fm.ensure('kai')).toBe(false)
    expect(fm.status('kai')).toBe('failed')
    online = true
    expect(await fm.ensure('kai')).toBe(true)
    expect(fm.status('kai')).toBe('ready')
  })

  it('核心說字型檔壞掉：false', async () => {
    const { fm } = setup({ registerOk: false })
    expect(await fm.ensure('kai')).toBe(false)
    expect(fm.status('kai')).toBe('failed')
  })

  it('不認得的字型：false，不會出錯', async () => {
    const { fm } = setup()
    expect(await fm.ensure('nope')).toBe(false)
  })

  it('一次要好幾個：回傳哪些成功', async () => {
    const { fm } = setup({ fetchOk: false })
    expect(await fm.ensureMany(['sans', 'kai'])).toEqual({ sans: true, kai: false })
  })

  it('狀態變化會通知畫面', async () => {
    const { fm } = setup()
    const seen: string[] = []
    fm.onChange(() => seen.push(fm.status('kai')))
    await fm.ensure('kai')
    expect(seen).toEqual(['loading', 'ready'])
  })
})
