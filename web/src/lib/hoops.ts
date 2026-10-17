// 繡框清單：原廠框加上使用者自己新增的框。自己的框存在這支手機上，重開後仍在。
// 尺寸是否合理一律交給核心判斷（validate），這裡只管編號、保護原廠框與儲存。
import type { Hoop, HoopShape, HoopValidation, UserMessage } from './types'

const STORAGE_KEY = 'embroider.hoops'
const DEFAULT_MARGIN = 50

/** 毫米換成核心使用的 0.1mm 單位。 */
export const mmToUnits = (mm: number): number => Math.round(mm * 10)

export interface HoopInput {
  name: string
  /** 可繡寬（0.1mm） */
  width: number
  /** 可繡高（0.1mm） */
  height: number
  shape: HoopShape
  /** 安全邊距（0.1mm），不給就用預設 5mm */
  margin?: number
}

export type HoopResult = { ok: true; hoop: Hoop } | { ok: false; message: UserMessage }
export type DoneResult = { ok: true } | { ok: false; message: UserMessage }

export interface HoopStore {
  list(): Hoop[]
  add(input: HoopInput): HoopResult
  update(id: string, patch: Partial<HoopInput>): HoopResult
  remove(id: string): DoneResult
}

const fail = (code: string, text: string, hint: string | null = null) => ({
  ok: false as const,
  message: { code, text, hint },
})

interface Saved {
  next: number
  hoops: Hoop[]
}

function isHoop(v: unknown): v is Hoop {
  if (!v || typeof v !== 'object') return false
  const h = v as Record<string, unknown>
  return (
    typeof h.id === 'string' &&
    typeof h.name === 'string' &&
    typeof h.width === 'number' &&
    typeof h.height === 'number' &&
    typeof h.margin === 'number' &&
    (h.shape === 'rect' || h.shape === 'circle' || h.shape === 'ellipse')
  )
}

export function createHoopStore(
  builtins: Hoop[],
  validate: (h: Hoop) => HoopValidation,
): HoopStore {
  let saved: Saved = { next: 1, hoops: [] }

  try {
    const raw = localStorage.getItem(STORAGE_KEY)
    if (raw) {
      const parsed = JSON.parse(raw) as Partial<Saved>
      const hoops = Array.isArray(parsed.hoops) ? parsed.hoops.filter(isHoop) : []
      saved = {
        next: typeof parsed.next === 'number' ? parsed.next : hoops.length + 1,
        hoops: hoops.filter((h) => validate({ ...h, builtin: false }).ok).map((h) => ({ ...h, builtin: false })),
      }
    }
  } catch {
    // 儲存空間不能用或內容壞掉：只用原廠框，這次使用中新增的仍然有效
  }

  const persist = () => {
    try {
      localStorage.setItem(STORAGE_KEY, JSON.stringify(saved))
    } catch {
      // 存不進去也沒關係，這次使用中仍然有效
    }
  }

  const lockedMessage = () =>
    fail('builtin_locked', '原廠的框不能修改或刪除。', '想用不一樣的尺寸，請新增自己的框。')
  const notFound = () => fail('hoop_not_found', '找不到這個框。', '請回到清單重新選一次。')

  return {
    list: () => [...builtins, ...saved.hoops],

    add(input) {
      const hoop: Hoop = {
        id: `custom-${saved.next}`,
        name: input.name,
        width: input.width,
        height: input.height,
        shape: input.shape,
        margin: input.margin ?? DEFAULT_MARGIN,
        builtin: false,
      }
      const v = validate(hoop)
      if (!v.ok) return { ok: false, message: v.message! }
      saved = { next: saved.next + 1, hoops: [...saved.hoops, hoop] }
      persist()
      return { ok: true, hoop }
    },

    update(id, patch) {
      if (builtins.some((h) => h.id === id)) return lockedMessage()
      const current = saved.hoops.find((h) => h.id === id)
      if (!current) return notFound()
      const next: Hoop = { ...current, ...patch, id, builtin: false }
      const v = validate(next)
      if (!v.ok) return { ok: false, message: v.message! }
      saved = { ...saved, hoops: saved.hoops.map((h) => (h.id === id ? next : h)) }
      persist()
      return { ok: true, hoop: next }
    },

    remove(id) {
      if (builtins.some((h) => h.id === id)) return lockedMessage()
      if (!saved.hoops.some((h) => h.id === id)) return notFound()
      saved = { ...saved, hoops: saved.hoops.filter((h) => h.id !== id) }
      persist()
      return { ok: true }
    },
  }
}
