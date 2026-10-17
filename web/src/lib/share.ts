// 分享備援：下載之外，也可以用手機的「分享」把檔案送到「檔案」App，再存進插著的 USB。
import type { EncodedFile, UserMessage } from './types'

export type ShareResult =
  | { ok: true }
  | { ok: false; cancelled: true }
  | { ok: false; cancelled?: false; message: UserMessage }

function toFiles(files: EncodedFile[]): File[] {
  return files.map(
    (f) => new File([f.bytes as BlobPart], f.fileName, { type: 'application/octet-stream' }),
  )
}

export function canShareFiles(files: EncodedFile[]): boolean {
  const nav = globalThis.navigator as Navigator | undefined
  if (!nav || typeof nav.canShare !== 'function' || typeof nav.share !== 'function') return false
  try {
    return nav.canShare({ files: toFiles(files) })
  } catch {
    return false
  }
}

export async function shareFiles(files: EncodedFile[]): Promise<ShareResult> {
  if (!canShareFiles(files)) {
    return {
      ok: false,
      message: {
        code: 'share_unavailable',
        text: '這支手機沒辦法用分享的方式傳檔案。',
        hint: '請改用「存到 USB」下載檔案，再用「檔案」App 放進隨身碟。',
      },
    }
  }
  try {
    await navigator.share({ files: toFiles(files) })
    return { ok: true }
  } catch (e) {
    if ((e as Error).name === 'AbortError') return { ok: false, cancelled: true }
    return {
      ok: false,
      message: {
        code: 'share_failed',
        text: '分享沒有成功。',
        hint: '請再試一次，或改用「存到 USB」。',
      },
    }
  }
}
