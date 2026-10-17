// 核心（Rust → WASM）的載入與型別化包裝。所有函式都是純函式。
import init, * as wasm from './wasm/stitch_wasm.js'
import type {
  EncodeResult,
  FontEntry,
  Hoop,
  HoopValidation,
  ImageRequest,
  OutputFormatInfo,
  PreviewResult,
  StitchPlan,
  TextRequest,
  ThreadColor,
} from './types'

let loading: Promise<unknown> | null = null

/** 載入 WASM（瀏覽器用）。可重複呼叫。 */
export function initCore(): Promise<unknown> {
  loading ??= init()
  return loading
}

/** 同步載入（測試用）：直接給 WASM 位元組。 */
export function initCoreSync(bytes: BufferSource): void {
  wasm.initSync({ module: bytes })
}

export function listBuiltinHoops(): Hoop[] {
  return wasm.listBuiltinHoops() as Hoop[]
}

export function listFormats(): OutputFormatInfo[] {
  return wasm.listFormats() as OutputFormatInfo[]
}

export function validateHoop(hoop: Hoop): HoopValidation {
  return wasm.validateHoop(hoop) as HoopValidation
}

export function registerFont(id: string, name: string, bytes: Uint8Array): HoopValidation {
  return wasm.registerFont(id, name, bytes) as HoopValidation
}

export function listFonts(): FontEntry[] {
  return wasm.listFonts() as FontEntry[]
}

export function buildTextPreview(request: TextRequest): PreviewResult {
  return wasm.buildTextPreview({ settings: {}, ...request }) as PreviewResult
}

/** 一次輸出多種格式。`formatIds` 為空代表所有格式。 */
export function encodeAll(
  plan: StitchPlan,
  hoop: Hoop,
  formatIds: string[],
  designName: string,
  sequence: number,
): EncodeResult {
  return wasm.encodeAll(plan, hoop, formatIds, designName, sequence) as EncodeResult
}

/** 圖片轉刺繡。`rgba` 是解碼後的像素（每個像素 4 個位元組：紅綠藍透明度）。 */
export function buildImagePreview(request: ImageRequest, rgba: Uint8Array): PreviewResult {
  return wasm.buildImagePreview({ settings: {}, ...request }, rgba) as PreviewResult
}

/** 依圖片內容建議的顏色數（2～8）。 */
export function suggestColorCount(rgba: Uint8Array, width: number, height: number): number {
  return wasm.suggestColorCount(rgba, width, height)
}

/** 線色色票（與前端畫面上的色票一致）。 */
export function listThreadColors(): ThreadColor[] {
  return wasm.listThreadColors() as ThreadColor[]
}
