// 新版本更新：離線服務會把程式快取起來，新版本下載好之後要「切換」才會用到。
// 沒有切換，使用者開著的就一直是舊版（舊的畫面、舊的速度），看起來像是「改了又沒改」。
// 這裡負責：定時與回到分頁時去看有沒有新版本、發現了就通知畫面、切換之前先把做到一半的設計存好。

export interface Registration {
  update(): Promise<unknown>
}

export interface RegisterOptions {
  onNeedRefresh: () => void
  onRegistered: (r?: Registration) => void
}

/** 註冊離線服務；回傳「切換到新版本」的函式（參數 true 會一併重新整理）。 */
export type Register = (o: RegisterOptions) => (reload?: boolean) => Promise<void>

export interface Updater {
  /** 有沒有已經下載好、等著切換的新版本 */
  available(): boolean
  /** 馬上去看有沒有新版本 */
  checkNow(): Promise<void>
  /** 切換到新版本並重新整理；沒有新版本就什麼都不做 */
  apply(): Promise<void>
  onChange(fn: () => void): () => void
}

export interface UpdaterOptions {
  /** 切換之前要做的事（把做到一半的設計存起來） */
  flush?: () => Promise<void>
  /** 多久檢查一次（毫秒） */
  intervalMs?: number
  setInterval?: (fn: () => void, ms: number) => unknown
}

export function createUpdater(register: Register, o: UpdaterOptions = {}): Updater {
  let available = false
  let applying: Promise<void> | null = null
  let registration: Registration | undefined
  const listeners = new Set<() => void>()
  const schedule = o.setInterval ?? ((fn, ms) => setInterval(fn, ms))

  const check = async () => {
    try {
      await registration?.update()
    } catch {
      // 沒有網路就下次再看
    }
  }

  const updateSW = register({
    onNeedRefresh: () => {
      available = true
      for (const fn of listeners) fn()
    },
    onRegistered: (r) => {
      registration = r
    },
  })
  schedule(() => void check(), o.intervalMs ?? 60_000)

  return {
    available: () => available,
    checkNow: check,
    apply() {
      if (!available) return Promise.resolve()
      applying ??= (async () => {
        try {
          await o.flush?.()
        } catch {
          // 存不起來也要更新，不能卡在舊版
        }
        await updateSW(true)
      })()
      return applying
    },
    onChange(fn) {
      listeners.add(fn)
      return () => listeners.delete(fn)
    },
  }
}
