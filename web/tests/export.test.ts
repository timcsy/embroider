import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { downloadFiles, nextSequence } from '../src/lib/export'

describe('下載檔案', () => {
  const clicked: { download: string; href: string }[] = []

  beforeEach(() => {
    clicked.length = 0
    vi.useFakeTimers()
    let n = 0
    vi.stubGlobal('URL', {
      createObjectURL: () => `blob:fake-${n++}`,
      revokeObjectURL: vi.fn(),
    })
    vi.spyOn(HTMLAnchorElement.prototype, 'click').mockImplementation(function (this: HTMLAnchorElement) {
      clicked.push({ download: this.download, href: this.href })
    })
  })
  afterEach(() => {
    vi.useRealTimers()
    vi.unstubAllGlobals()
    vi.restoreAllMocks()
  })

  it('每個檔案各觸發一次下載，檔名正確', async () => {
    const p = downloadFiles([
      { fileName: 'AMY0001.DST', bytes: new Uint8Array([1, 2, 3]) },
      { fileName: 'AMY0001.EXP', bytes: new Uint8Array([4, 5]) },
    ])
    await vi.runAllTimersAsync()
    await p
    expect(clicked.map((c) => c.download)).toEqual(['AMY0001.DST', 'AMY0001.EXP'])
  })

  it('下載之間有間隔，避免瀏覽器擋掉多檔下載', async () => {
    const times: number[] = []
    vi.spyOn(HTMLAnchorElement.prototype, 'click').mockImplementation(() => {
      times.push(Date.now())
    })
    const p = downloadFiles([
      { fileName: 'A.DST', bytes: new Uint8Array([1]) },
      { fileName: 'A.EXP', bytes: new Uint8Array([1]) },
    ])
    await vi.runAllTimersAsync()
    await p
    expect(times[1] - times[0]).toBeGreaterThanOrEqual(300)
  })
})

describe('序號', () => {
  beforeEach(() => localStorage.clear())

  it('每次輸出遞增，從 1 開始', () => {
    expect(nextSequence()).toBe(1)
    expect(nextSequence()).toBe(2)
    expect(nextSequence()).toBe(3)
  })

  it('儲存空間壞掉時仍然給得出序號', () => {
    vi.spyOn(Storage.prototype, 'getItem').mockImplementation(() => {
      throw new Error('blocked')
    })
    vi.spyOn(Storage.prototype, 'setItem').mockImplementation(() => {
      throw new Error('blocked')
    })
    expect(nextSequence()).toBeGreaterThanOrEqual(1)
    vi.restoreAllMocks()
  })
})
