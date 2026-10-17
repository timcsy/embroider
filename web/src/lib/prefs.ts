// 使用者設定：預設輸出格式、最近用的框。存在這支手機上，重開後仍在。
// 預設格式「尚未選定」（空陣列）時，輸出會把每一種格式都存一份，讓媽媽拿去機器上逐一試。

const STORAGE_KEY = 'embroider.prefs'

export interface Prefs {
  /** 空陣列代表尚未選定（輸出全部格式） */
  readonly defaultFormats: string[]
  readonly lastHoopId: string | null
  /** 還沒選定預設格式，也就是還沒確認過機器讀得到哪一種 */
  readonly isFirstTime: boolean
  setDefaultFormats(ids: string[]): void
  clearDefaultFormats(): void
  setLastHoop(id: string): void
  /** 設定改變時通知（畫面據此更新）；回傳取消訂閱的函式 */
  onChange(listener: () => void): () => void
}

interface Saved {
  defaultFormats: string[]
  lastHoopId: string | null
}

export function createPrefs(knownFormatIds: string[]): Prefs {
  const known = new Set(knownFormatIds)
  const onlyKnown = (ids: unknown): string[] =>
    Array.isArray(ids) ? ids.filter((x): x is string => typeof x === 'string' && known.has(x)) : []

  let state: Saved = { defaultFormats: [], lastHoopId: null }
  try {
    const raw = localStorage.getItem(STORAGE_KEY)
    if (raw) {
      const parsed: unknown = JSON.parse(raw)
      if (parsed && typeof parsed === 'object') {
        const p = parsed as Partial<Saved>
        state = {
          defaultFormats: onlyKnown(p.defaultFormats),
          lastHoopId: typeof p.lastHoopId === 'string' ? p.lastHoopId : null,
        }
      }
    }
  } catch {
    // 儲存空間不能用或內容壞掉：用預設值
  }

  const listeners = new Set<() => void>()
  const changed = (before: Saved) => {
    if (JSON.stringify(before) === JSON.stringify(state)) return
    save()
    listeners.forEach((l) => l())
  }

  const save = () => {
    try {
      localStorage.setItem(STORAGE_KEY, JSON.stringify(state))
    } catch {
      // 存不進去也沒關係，這次使用中仍然有效
    }
  }

  return {
    get defaultFormats() {
      return state.defaultFormats
    },
    get lastHoopId() {
      return state.lastHoopId
    },
    get isFirstTime() {
      return state.defaultFormats.length === 0
    },
    setDefaultFormats(ids) {
      const before = state
      state = { ...state, defaultFormats: onlyKnown(ids) }
      changed(before)
    },
    clearDefaultFormats() {
      const before = state
      state = { ...state, defaultFormats: [] }
      changed(before)
    },
    setLastHoop(id) {
      const before = state
      state = { ...state, lastHoopId: id }
      changed(before)
    },
    onChange(listener) {
      listeners.add(listener)
      return () => listeners.delete(listener)
    },
  }
}

/** 傳給核心的格式清單；空陣列代表全部。 */
export function formatsToExport(defaultFormats: string[], mode: 'default' | 'all'): string[] {
  return mode === 'all' ? [] : defaultFormats
}
