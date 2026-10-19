// 字型按需下載：一開始只有基本字型（黑體），其他字型第一次用到才下載，下載過之後瀏覽器會留著，之後離線也能用。
// 下載與交給核心註冊是外部傳進來的函式，這樣可以用假的來測試。

export type FontStatus = 'idle' | 'loading' | 'ready' | 'failed'

export interface FontSpec {
  id: string
  file: string
  /** 一開始就載入（基本字型） */
  preload: boolean
}

export interface FontManager {
  status(id: string): FontStatus
  isReady(id: string): boolean
  /** 確保字型可用；需要的話現在下載。成功回傳 true。 */
  ensure(id: string): Promise<boolean>
  ensureMany(ids: string[]): Promise<Record<string, boolean>>
  onChange(fn: () => void): () => void
}

export interface FontManagerOptions {
  fonts: readonly FontSpec[]
  fetchFont: (file: string) => Promise<Uint8Array>
  registerFont: (id: string, name: string, bytes: Uint8Array) => Promise<{ ok: boolean; message: unknown }>
  /** 已經載入核心的字型（開機時載入的基本字型） */
  preloaded: readonly string[]
}

export function createFontManager(o: FontManagerOptions): FontManager {
  const state = new Map<string, FontStatus>()
  const pending = new Map<string, Promise<boolean>>()
  const listeners = new Set<() => void>()
  for (const f of o.fonts) state.set(f.id, o.preloaded.includes(f.id) ? 'ready' : 'idle')

  const set = (id: string, s: FontStatus) => {
    state.set(id, s)
    for (const fn of listeners) fn()
  }

  async function load(spec: FontSpec): Promise<boolean> {
    set(spec.id, 'loading')
    try {
      const bytes = await o.fetchFont(spec.file)
      const r = await o.registerFont(spec.id, spec.id, bytes)
      if (!r.ok) throw new Error('bad font')
      set(spec.id, 'ready')
      return true
    } catch {
      set(spec.id, 'failed')
      return false
    }
  }

  const manager: FontManager = {
    status: (id) => state.get(id) ?? 'idle',
    isReady: (id) => state.get(id) === 'ready',
    ensure(id) {
      const spec = o.fonts.find((f) => f.id === id)
      if (!spec) return Promise.resolve(false)
      if (state.get(id) === 'ready') return Promise.resolve(true)
      let p = pending.get(id)
      if (!p) {
        p = load(spec).finally(() => pending.delete(id))
        pending.set(id, p)
      }
      return p
    },
    async ensureMany(ids) {
      const out: Record<string, boolean> = {}
      await Promise.all(ids.map(async (id) => (out[id] = await manager.ensure(id))))
      return out
    },
    onChange(fn) {
      listeners.add(fn)
      return () => listeners.delete(fn)
    },
  }
  return manager
}
