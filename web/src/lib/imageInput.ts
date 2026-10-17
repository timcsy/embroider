// 選圖片：檢查檔案、解碼成像素、必要時縮小。解碼用瀏覽器內建功能，不經過伺服器。
import type { UserMessage } from './types'

/** 傳給核心前，圖片長邊最多幾個點（核心還會再縮小處理）。 */
export const MAX_SIDE = 512
const MAX_FILE_BYTES = 20 * 1024 * 1024
const OK_TYPES = ['image/png', 'image/jpeg', 'image/webp', 'image/gif', 'image/bmp']

const msg = (code: string, text: string, hint: string): UserMessage => ({ code, text, hint })

export function fitWithin(w: number, h: number, max: number): { width: number; height: number } {
  const long = Math.max(w, h)
  if (long <= max) return { width: w, height: h }
  const f = max / long
  return { width: Math.max(1, Math.round(w * f)), height: Math.max(1, Math.round(h * f)) }
}

export function validateImageFile(file: File): UserMessage | null {
  if (!file.type.startsWith('image/')) {
    return msg('not_image', '這個檔案不是圖片。', '請從相簿選一張照片或圖。')
  }
  if (!OK_TYPES.includes(file.type)) {
    return msg('unsupported_image', '這種圖片格式打不開。', '請改選 JPG 或 PNG 的圖。')
  }
  if (file.size > MAX_FILE_BYTES) {
    return msg('image_too_big', '這張圖的檔案太大了。', '請換一張小一點的，或先縮小再選。')
  }
  return null
}

/** 去掉副檔名。 */
export function stemOf(name: string): string {
  const i = name.lastIndexOf('.')
  return i > 0 ? name.slice(0, i) : name
}

/** `data:...;base64,xxxx` 轉回位元組；不是 data URL 就回傳空。 */
export function dataUrlToBytes(url: string): Uint8Array {
  const m = /^data:[^;,]*;base64,(.*)$/s.exec(url)
  if (!m) return new Uint8Array(0)
  const bin = atob(m[1])
  const out = new Uint8Array(bin.length)
  for (let i = 0; i < bin.length; i++) out[i] = bin.charCodeAt(i)
  return out
}

export function fileFromBytes(bytes: Uint8Array, name: string): File {
  return new File([bytes as BlobPart], name, { type: 'image/png' })
}

export interface LoadedImage {
  name: string
  width: number
  height: number
  rgba: Uint8Array
  /** 給畫面顯示縮圖用 */
  previewUrl: string
}

/** 把選到的檔案解碼成像素。失敗時回傳白話訊息。 */
export async function loadImage(file: File): Promise<LoadedImage | UserMessage> {
  const problem = validateImageFile(file)
  if (problem) return problem
  try {
    const bitmap = await createImageBitmap(file)
    const { width, height } = fitWithin(bitmap.width, bitmap.height, MAX_SIDE)
    const canvas = document.createElement('canvas')
    canvas.width = width
    canvas.height = height
    const ctx = canvas.getContext('2d', { willReadFrequently: true })
    if (!ctx) throw new Error('no canvas')
    ctx.drawImage(bitmap, 0, 0, width, height)
    bitmap.close?.()
    const data = ctx.getImageData(0, 0, width, height)
    return {
      name: file.name,
      width,
      height,
      rgba: new Uint8Array(data.data.buffer.slice(0)),
      previewUrl: canvas.toDataURL('image/png'),
    }
  } catch {
    return msg('image_decode_failed', '這張圖打不開。', '請換一張圖，或改選 JPG、PNG 的圖。')
  }
}
