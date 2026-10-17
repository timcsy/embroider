// 新增／編輯繡框的表單：把使用者填的字串（公釐）整理成核心要的資料。
// 數字是否在合理範圍，仍由核心的驗證判斷；這裡只處理「填的不是數字」這類輸入問題。
import { mmToUnits, type HoopInput } from './hoops'
import type { HoopShape, UserMessage } from './types'

export interface HoopFormValues {
  name: string
  widthMm: string
  heightMm: string
  shape: HoopShape
}

export type FormResult = { ok: true; input: HoopInput } | { ok: false; message: UserMessage }

const bad = (code: string, text: string, hint: string): FormResult => ({
  ok: false,
  message: { code, text, hint },
})

/** 全形轉半角、逗號當小數點；不是單純的數字就回傳 null。 */
function parseMm(raw: string): number | null {
  const s = raw.normalize('NFKC').trim().replace(/,/g, '.')
  return /^\d+(\.\d+)?$/.test(s) ? Number(s) : null
}

export function parseHoopForm(v: HoopFormValues): FormResult {
  if (v.name.trim().length === 0) {
    return bad('hoop_name_empty', '請幫這個框取個名字。', '例如「10×10 方框」。')
  }
  const w = parseMm(v.widthMm)
  if (w === null) {
    return bad('hoop_width_nan', '寬度要填數字。', '請填框裡面可以繡的寬度，單位是公釐，例如 100。')
  }
  let h: number
  if (v.shape === 'circle') {
    h = w
  } else {
    const parsed = parseMm(v.heightMm)
    if (parsed === null) {
      return bad('hoop_height_nan', '高度要填數字。', '請填框裡面可以繡的高度，單位是公釐，例如 100。')
    }
    h = parsed
  }
  return {
    ok: true,
    input: { name: v.name.trim(), width: mmToUnits(w), height: mmToUnits(h), shape: v.shape },
  }
}
