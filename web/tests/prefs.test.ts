import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPrefs, formatsToExport } from '../src/lib/prefs'

const KNOWN = ['dst', 'exp']

describe('使用者設定', () => {
  beforeEach(() => localStorage.clear())

  it('一開始還沒選定預設格式', () => {
    const p = createPrefs(KNOWN)
    expect(p.defaultFormats).toEqual([])
    expect(p.isFirstTime).toBe(true)
  })

  it('選定預設格式後，重新載入仍記得', () => {
    const p = createPrefs(KNOWN)
    p.setDefaultFormats(['exp'])
    expect(p.isFirstTime).toBe(false)
    const again = createPrefs(KNOWN)
    expect(again.defaultFormats).toEqual(['exp'])
    expect(again.isFirstTime).toBe(false)
  })

  it('可以改回「尚未選定」', () => {
    const p = createPrefs(KNOWN)
    p.setDefaultFormats(['dst'])
    p.clearDefaultFormats()
    expect(createPrefs(KNOWN).defaultFormats).toEqual([])
    expect(createPrefs(KNOWN).isFirstTime).toBe(true)
  })

  it('不認得的格式會被忽略（例如日後移除了某種格式）', () => {
    localStorage.setItem('embroider.prefs', JSON.stringify({ defaultFormats: ['dst', 'gone'] }))
    expect(createPrefs(KNOWN).defaultFormats).toEqual(['dst'])
    const p = createPrefs(KNOWN)
    p.setDefaultFormats(['nope'])
    expect(p.defaultFormats).toEqual([])
  })

  it('記住最近用的框', () => {
    const p = createPrefs(KNOWN)
    expect(p.lastHoopId).toBeNull()
    p.setLastHoop('custom-1')
    expect(createPrefs(KNOWN).lastHoopId).toBe('custom-1')
  })

  it('儲存內容壞掉時不會當機', () => {
    localStorage.setItem('embroider.prefs', '{壞掉')
    const p = createPrefs(KNOWN)
    expect(p.defaultFormats).toEqual([])
    localStorage.setItem('embroider.prefs', '"字串"')
    expect(createPrefs(KNOWN).defaultFormats).toEqual([])
  })

  it('儲存空間不能用時仍然可以設定（只在這次使用中有效）', () => {
    vi.spyOn(Storage.prototype, 'setItem').mockImplementation(() => {
      throw new Error('blocked')
    })
    const p = createPrefs(KNOWN)
    p.setDefaultFormats(['dst'])
    expect(p.defaultFormats).toEqual(['dst'])
    vi.restoreAllMocks()
  })
})

describe('設定改變時會通知畫面', () => {
  beforeEach(() => localStorage.clear())

  it('設定預設格式、清除、記住框都會通知', () => {
    const p = createPrefs(KNOWN)
    const listener = vi.fn()
    p.onChange(listener)
    p.setDefaultFormats(['dst'])
    p.clearDefaultFormats()
    p.setLastHoop('custom-1')
    expect(listener).toHaveBeenCalledTimes(3)
  })

  it('取消訂閱之後不再通知', () => {
    const p = createPrefs(KNOWN)
    const listener = vi.fn()
    const off = p.onChange(listener)
    off()
    p.setDefaultFormats(['dst'])
    expect(listener).not.toHaveBeenCalled()
  })

  it('值沒有真的改變時不通知（避免畫面無謂更新）', () => {
    const p = createPrefs(KNOWN)
    const listener = vi.fn()
    p.onChange(listener)
    p.clearDefaultFormats()
    p.setLastHoop('x')
    p.setLastHoop('x')
    expect(listener).toHaveBeenCalledTimes(1)
  })
})

describe('要輸出哪些格式', () => {
  it('尚未選定時輸出全部（空陣列代表全部）', () => {
    expect(formatsToExport([], 'default')).toEqual([])
  })
  it('選定後只輸出所選', () => {
    expect(formatsToExport(['exp'], 'default')).toEqual(['exp'])
  })
  it('按「全部都存」時一律輸出全部', () => {
    expect(formatsToExport(['exp'], 'all')).toEqual([])
  })
})
