// 第一次使用的提示：一次只出現一個，指向一個動作，做到那個動作就換下一個，全部做完或按「不用了」就再也不出現。
// 進度記在這支手機上。像標記選單的概念：新手看得到提示，熟了就退場。

export type TipId = 'add' | 'move' | 'resize'
export const TIP_ORDER: readonly TipId[] = ['add', 'move', 'resize']

const KEY = 'embroider.tips'

export interface TipStore {
  getItem(key: string): string | null
  setItem(key: string, value: string): void
}

export interface Tips {
  /** 現在該顯示哪一個提示；沒有就是 null。 */
  current(): TipId | null
  /** 使用者做了某個動作：如果正好是現在的提示，就換成下一個。 */
  did(tip: TipId): void
  /** 「不用了」：永遠不再提示。 */
  dismiss(): void
  /** 直接從某一步開始（例如已經有設計的人不需要「加東西」）。 */
  skipTo(tip: TipId): void
  onChange(fn: () => void): () => void
}

/** 進度：做到第幾個（0 到 3；3 以上或被關掉就是結束）。 */
export function createTips(store: TipStore): Tips {
  const read = (): number => {
    try {
      const raw = store.getItem(KEY)
      if (raw === 'off') return TIP_ORDER.length
      const n = Number(raw)
      return Number.isFinite(n) && n >= 0 ? Math.min(n, TIP_ORDER.length) : 0
    } catch {
      return -1 // 讀不到：這次用記憶體裡的進度
    }
  }
  let step = Math.max(0, read())
  const listeners = new Set<() => void>()

  const write = (value: number | 'off') => {
    step = value === 'off' ? TIP_ORDER.length : value
    try {
      store.setItem(KEY, value === 'off' ? 'off' : String(value))
    } catch {
      // 存不進去也沒關係，這次還是照進度走
    }
    for (const fn of listeners) fn()
  }

  return {
    current: () => TIP_ORDER[step] ?? null,
    did(tip) {
      if (TIP_ORDER[step] === tip) write(step + 1)
    },
    dismiss: () => write('off'),
    skipTo(tip) {
      const i = TIP_ORDER.indexOf(tip)
      if (i > step) write(i)
    },
    onChange(fn) {
      listeners.add(fn)
      return () => listeners.delete(fn)
    },
  }
}
