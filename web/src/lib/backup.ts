// 設計備份：把做到一半的設計（含圖片）存成一個檔案，之後可以讀回來。
// 檔案是 JSON 文字；圖片用 base64 放在裡面。換手機、傳給別人、保留超過十個都靠它。
import type { CurrentDraft } from './designs'
import type { UserMessage } from './types'

const FORMAT = 1

const bad = (text: string, hint: string): { ok: false; message: UserMessage } => ({
  ok: false,
  message: { code: 'bad_backup', text, hint },
})

function toBase64(bytes: Uint8Array): string {
  let s = ''
  for (let i = 0; i < bytes.length; i += 0x8000) s += String.fromCharCode(...bytes.subarray(i, i + 0x8000))
  return btoa(s)
}
function fromBase64(b64: string): Uint8Array {
  const s = atob(b64)
  const out = new Uint8Array(s.length)
  for (let i = 0; i < s.length; i++) out[i] = s.charCodeAt(i)
  return out
}

export function makeBackup(draft: CurrentDraft): Uint8Array {
  const body = {
    app: 'embroider',
    format: FORMAT,
    params: draft.params,
    images: draft.images.map((im) => ({ id: im.id, name: im.name, width: im.width, height: im.height, png: toBase64(im.png) })),
  }
  return new TextEncoder().encode(JSON.stringify(body))
}

export function parseBackup(bytes: Uint8Array): { ok: true; draft: CurrentDraft } | { ok: false; message: UserMessage } {
  const notBackup = () => bad('這個檔案不是刺繡小幫手的備份。', '請選當初按「備份這個設計」存下來的檔案。')
  try {
    const o = JSON.parse(new TextDecoder().decode(bytes))
    if (!o || o.app !== 'embroider') return notBackup()
    if (o.format !== FORMAT) return bad('這個備份的版本不認得。', '可能是更新的版本做的，請更新後再試一次。')
    if (!o.params || typeof o.params !== 'object' || !Array.isArray(o.images)) return notBackup()
    const images = o.images.map((im: Record<string, unknown>) => ({
      id: String(im.id),
      name: String(im.name),
      width: Number(im.width),
      height: Number(im.height),
      png: fromBase64(String(im.png)),
    }))
    return { ok: true, draft: { params: o.params, images } }
  } catch {
    return notBackup()
  }
}

/** 備份檔名：刺繡備份-設計名稱-日期.json（去掉不能當檔名的字）。 */
export function backupFileName(name: string, now = new Date()): string {
  const clean = name.replace(/[\\/:*?"<>|\s]/g, '').slice(0, 20)
  const day = now.toISOString().slice(0, 10)
  return clean ? `刺繡備份-${clean}-${day}.json` : `刺繡備份-${day}.json`
}
