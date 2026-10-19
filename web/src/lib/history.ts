// 可復原、可重做的編輯歷史。連續輸入同一欄位會合併成一步。

export interface PushOptions {
  /** 相同的鍵且間隔很短，就合併成同一步（例如連續打字） */
  coalesce?: string
  /** 現在時間（毫秒），測試用 */
  now?: number
}

const COALESCE_WINDOW_MS = 1000

export class History<T> {
  private stack: T[]
  private index = 0
  private lastKey: string | undefined
  private lastTime = -Infinity
  /** 目前這一步的 JSON（比較用）；快取起來，免得每次修改都要把整份設計轉兩次 */
  private currentJson: string | null = null

  constructor(
    initial: T,
    private readonly maxSteps = 100,
  ) {
    this.stack = [initial]
  }

  get current(): T {
    return this.stack[this.index]
  }
  get canUndo(): boolean {
    return this.index > 0
  }
  get canRedo(): boolean {
    return this.index < this.stack.length - 1
  }

  push(value: T, opts: PushOptions = {}): void {
    const json = JSON.stringify(value)
    this.currentJson ??= JSON.stringify(this.current)
    if (json === this.currentJson) return
    const now = opts.now ?? Date.now()
    const merge =
      opts.coalesce !== undefined &&
      opts.coalesce === this.lastKey &&
      now - this.lastTime < COALESCE_WINDOW_MS &&
      this.index > 0
    this.stack.length = this.index + 1 // 丟掉重做的部分
    this.currentJson = json
    if (merge) {
      this.stack[this.index] = value
    } else {
      this.stack.push(value)
      this.index++
      if (this.stack.length > this.maxSteps + 1) {
        this.stack.shift()
        this.index--
      }
    }
    this.lastKey = opts.coalesce
    this.lastTime = now
  }

  undo(): T {
    if (this.canUndo) this.index--
    this.lastKey = undefined
    this.currentJson = null
    return this.current
  }

  redo(): T {
    if (this.canRedo) this.index++
    this.lastKey = undefined
    this.currentJson = null
    return this.current
  }
}
