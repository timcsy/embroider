// Web Worker：在背景執行核心，主畫面不會因為運算而卡住。
import * as core from './core'
import { FONT_FILES } from './fonts'

async function boot(fontsBase: string) {
  await core.initCore()
  for (const f of FONT_FILES) {
    const res = await fetch(`${fontsBase}${f.file}`)
    if (!res.ok) throw new Error(`字型載入失敗：${f.file}`)
    const r = core.registerFont(f.id, f.id, new Uint8Array(await res.arrayBuffer()))
    if (!r.ok) throw new Error(r.message?.text ?? '字型載入失敗')
  }
}

// 主畫面先告知字型所在的網址（Worker 無法自己判斷頁面放在哪個路徑）
let booted = false
function start(fontsBase: string) {
  if (booted) return
  booted = true
  boot(fontsBase).then(
    () => postMessage({ ready: true }),
    (e: Error) => postMessage({ ready: false, error: e.message }),
  )
}

const handlers: Record<string, (...args: never[]) => unknown> = {
  listBuiltinHoops: core.listBuiltinHoops,
  listFormats: core.listFormats,
  buildTextPreview: core.buildTextPreview,
  buildImagePreview: core.buildImagePreview,
  suggestColorCount: core.suggestColorCount,
  encodeAll: core.encodeAll,
}

self.onmessage = (e: MessageEvent<{ init?: { fontsBase: string }; id: number; method: string; args: never[] }>) => {
  if (e.data.init) return start(e.data.init.fontsBase)
  const { id, method, args } = e.data
  try {
    postMessage({ id, result: handlers[method](...args) })
  } catch (err) {
    postMessage({ id, error: (err as Error).message })
  }
}
