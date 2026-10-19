// 畫布更新的排程：不加任何等待——沒有在算就馬上開始；正在算的時候又改了，只留最新的一份，算完立刻接著算。
// 這樣拖動、拉滑桿時預覽會一路跟著走，而不是等手停下來才更新。內容沒變（鍵一樣）就不重算。

export interface LatestRunner<T> {
  submit(job: T, key: string): void
  /** 清掉排隊中的，並忘記上次的鍵（下次同樣內容也會重算）。正在算的會算完，但不再接著算。 */
  cancel(): void
  busy(): boolean
  onBusyChange(fn: () => void): () => void
}

export function createLatestRunner<T>(run: (job: T) => Promise<void>): LatestRunner<T> {
  let running = false
  let pending: T | null = null
  let lastKey: string | null = null
  const listeners = new Set<() => void>()

  const setRunning = (v: boolean) => {
    if (running === v) return
    running = v
    for (const fn of listeners) fn()
  }

  async function start(job: T) {
    setRunning(true)
    try {
      await run(job)
    } catch {
      // 這一次算壞了，不要卡住後面的
    } finally {
      const next = pending
      pending = null
      if (next !== null) void start(next)
      else setRunning(false)
    }
  }

  return {
    submit(job, key) {
      if (key === lastKey) return
      lastKey = key
      if (running) pending = job
      else void start(job)
    },
    cancel() {
      pending = null
      lastKey = null
    },
    busy: () => running,
    onBusyChange(fn) {
      listeners.add(fn)
      return () => listeners.delete(fn)
    },
  }
}
