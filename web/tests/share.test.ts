import { afterEach, describe, expect, it, vi } from 'vitest'
import { canShareFiles, shareFiles } from '../src/lib/share'

const files = [
  { fileName: 'AMY0001.DST', bytes: new Uint8Array([1, 2, 3]) },
  { fileName: 'AMY0001.EXP', bytes: new Uint8Array([4, 5]) },
]

describe('分享備援', () => {
  afterEach(() => {
    vi.unstubAllGlobals()
  })

  it('瀏覽器沒有分享功能時回報不能用', () => {
    vi.stubGlobal('navigator', {})
    expect(canShareFiles(files)).toBe(false)
  })

  it('能分享檔案時回報可以用', () => {
    vi.stubGlobal('navigator', { canShare: () => true, share: vi.fn() })
    expect(canShareFiles(files)).toBe(true)
  })

  it('瀏覽器說不能分享這些檔案時回報不能用', () => {
    vi.stubGlobal('navigator', { canShare: () => false, share: vi.fn() })
    expect(canShareFiles(files)).toBe(false)
  })

  it('分享時把每個檔案以原檔名交出去', async () => {
    const share = vi.fn().mockResolvedValue(undefined)
    vi.stubGlobal('navigator', { canShare: () => true, share })
    expect(await shareFiles(files)).toEqual({ ok: true })
    const arg = share.mock.calls[0][0] as { files: File[] }
    expect(arg.files.map((f) => f.name)).toEqual(['AMY0001.DST', 'AMY0001.EXP'])
    expect(arg.files[0].size).toBe(3)
  })

  it('使用者自己取消分享不算錯誤', async () => {
    const abort = Object.assign(new Error('x'), { name: 'AbortError' })
    vi.stubGlobal('navigator', { canShare: () => true, share: vi.fn().mockRejectedValue(abort) })
    expect(await shareFiles(files)).toEqual({ ok: false, cancelled: true })
  })

  it('其他失敗回報白話訊息', async () => {
    vi.stubGlobal('navigator', {
      canShare: () => true,
      share: vi.fn().mockRejectedValue(new Error('boom')),
    })
    const r = await shareFiles(files)
    expect(r.ok).toBe(false)
    if (!r.ok && !r.cancelled) expect(r.message.text).toMatch(/[一-鿿]/)
  })

  it('不能分享時回報白話訊息而不是丟例外', async () => {
    vi.stubGlobal('navigator', {})
    const r = await shareFiles(files)
    expect(r.ok).toBe(false)
  })
})
