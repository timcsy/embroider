// 前端與核心之間的用戶端。瀏覽器裡用 Web Worker 執行（運算時介面不會凍結）；
// 測試或不支援 Worker 時用直接呼叫版，介面完全相同。
import * as core from './core'
import { FONT_FILES } from './fonts'
import type { HoopValidation } from './types'
import type {
  CompositeRequest,
  CompositeResult,
  EncodeResult,
  Hoop,
  ImageRequest,
  OutputFormatInfo,
  PreviewResult,
  StitchPlan,
  TextRequest,
} from './types'

export interface CoreClient {
  ready: Promise<void>
  listBuiltinHoops(): Promise<Hoop[]>
  listFormats(): Promise<OutputFormatInfo[]>
  buildTextPreview(req: TextRequest): Promise<PreviewResult>
  buildImagePreview(req: ImageRequest, rgba: Uint8Array): Promise<PreviewResult>
  buildCompositePreview(req: CompositeRequest, images: Uint8Array[]): Promise<CompositeResult>
  /** 把下載好的字型交給核心（之後文字才能用這個字型）。 */
  registerFont(id: string, name: string, bytes: Uint8Array): Promise<HoopValidation>
  suggestColorCount(rgba: Uint8Array, width: number, height: number): Promise<number>
  encodeAll(
    plan: StitchPlan,
    hoop: Hoop,
    formatIds: string[],
    designName: string,
    sequence: number,
  ): Promise<EncodeResult>
}

export interface DirectOptions {
  /** 給了就同步載入（測試用）；沒給就用瀏覽器的方式載入 */
  wasm?: BufferSource
  loadFont: (file: string) => Promise<Uint8Array>
}

async function boot(opts: DirectOptions): Promise<void> {
  if (opts.wasm) core.initCoreSync(opts.wasm)
  else await core.initCore()
  for (const f of FONT_FILES.filter((x) => x.preload)) {
    const r = core.registerFont(f.id, f.id, await opts.loadFont(f.file))
    if (!r.ok) throw new Error(r.message?.text ?? '字型載入失敗')
  }
}

/** 直接在目前的執行緒呼叫核心。 */
export function createDirectClient(opts: DirectOptions): CoreClient {
  const ready = boot(opts)
  // 避免沒人接 ready 時出現未處理的拒絕
  ready.catch(() => {})
  return {
    ready,
    listBuiltinHoops: async () => (await ready, core.listBuiltinHoops()),
    listFormats: async () => (await ready, core.listFormats()),
    buildTextPreview: async (req) => (await ready, core.buildTextPreview(req)),
    buildImagePreview: async (req, rgba) => (await ready, core.buildImagePreview(req, rgba)),
    buildCompositePreview: async (req, images) => (await ready, core.buildCompositePreview(req, images)),
    registerFont: async (id, name, bytes) => (await ready, core.registerFont(id, name, bytes)),
    suggestColorCount: async (rgba, w, h) => (await ready, core.suggestColorCount(rgba, w, h)),
    encodeAll: async (plan, hoop, ids, name, seq) =>
      (await ready, core.encodeAll(plan, hoop, ids, name, seq)),
  }
}

import { pageFontsBase } from './assets'

type Method = Exclude<keyof CoreClient, 'ready'>

/** 以 Web Worker 執行核心。 */
export function createWorkerClient(): CoreClient {
  const worker = new Worker(new URL('./worker.ts', import.meta.url), { type: 'module' })
  let nextId = 1
  const pending = new Map<number, { resolve: (v: unknown) => void; reject: (e: Error) => void }>()
  let readyResolve!: () => void
  let readyReject!: (e: Error) => void
  const ready = new Promise<void>((res, rej) => {
    readyResolve = res
    readyReject = rej
  })
  ready.catch(() => {})

  worker.onmessage = (e: MessageEvent) => {
    const d = e.data
    if (d.ready === true) return readyResolve()
    if (d.ready === false) return readyReject(new Error(d.error))
    const p = pending.get(d.id)
    if (!p) return
    pending.delete(d.id)
    if (d.error) p.reject(new Error(d.error))
    else p.resolve(d.result)
  }
  worker.onerror = (e) => readyReject(new Error(e.message))
  worker.postMessage({ init: { fontsBase: pageFontsBase() } })

  const call = <T>(method: Method, args: unknown[]): Promise<T> =>
    ready.then(
      () =>
        new Promise<T>((resolve, reject) => {
          const id = nextId++
          pending.set(id, { resolve: resolve as (v: unknown) => void, reject })
          const message = { id, method, args }
          try {
            worker.postMessage(message)
          } catch {
            // 傳進來的若是 Svelte 的代理物件，瀏覽器無法複製；資料都是純 JSON，轉一次就好
            worker.postMessage(JSON.parse(JSON.stringify(message)))
          }
        }),
    )

  return {
    ready,
    listBuiltinHoops: () => call('listBuiltinHoops', []),
    listFormats: () => call('listFormats', []),
    buildTextPreview: (req) => call('buildTextPreview', [req]),
    buildImagePreview: (req, rgba) => call('buildImagePreview', [req, rgba]),
    buildCompositePreview: (req, images) => call('buildCompositePreview', [req, images]),
    registerFont: (id, name, bytes) => call('registerFont', [id, name, bytes]),
    suggestColorCount: (rgba, w, h) => call('suggestColorCount', [rgba, w, h]),
    encodeAll: (plan, hoop, ids, name, seq) => call('encodeAll', [plan, hoop, ids, name, seq]),
  }
}
