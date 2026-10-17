// 把檔案交給瀏覽器下載。檔案之間留一點間隔，避免瀏覽器把連續的下載當成惡意行為擋掉。
import type { EncodedFile } from './types'

const GAP_MS = 400
const SEQ_KEY = 'embroider.sequence'

const sleep = (ms: number) => new Promise<void>((r) => setTimeout(r, ms))

export async function downloadFiles(files: EncodedFile[]): Promise<void> {
  for (let i = 0; i < files.length; i++) {
    const f = files[i]
    const url = URL.createObjectURL(new Blob([f.bytes as BlobPart], { type: 'application/octet-stream' }))
    const a = document.createElement('a')
    a.href = url
    a.download = f.fileName
    a.style.display = 'none'
    document.body.appendChild(a)
    a.click()
    a.remove()
    setTimeout(() => URL.revokeObjectURL(url), 30_000)
    if (i < files.length - 1) await sleep(GAP_MS)
  }
}

let memorySeq = 0

/** 檔名用的流水號，每次輸出加一。儲存空間不能用時退回記憶體內計數。 */
export function nextSequence(): number {
  try {
    const n = (Number(localStorage.getItem(SEQ_KEY)) || 0) + 1
    localStorage.setItem(SEQ_KEY, String(n))
    return n
  } catch {
    return ++memorySeq
  }
}
