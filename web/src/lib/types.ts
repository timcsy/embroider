// 與 Rust 核心共用的資料形狀。長度單位為 0.1mm 的整數。

export type HoopShape = 'rect' | 'circle' | 'ellipse'

export interface Hoop {
  id: string
  name: string
  width: number
  height: number
  shape: HoopShape
  margin: number
  builtin: boolean
}

export interface UserMessage {
  code: string
  text: string
  hint: string | null
}

export interface OutputFormatInfo {
  id: string
  extension: string
  label: string
  description: string
}

export interface HoopValidation {
  ok: boolean
  message: UserMessage | null
}

export interface ThreadColor {
  r: number
  g: number
  b: number
  /** 給使用者看的中文色名 */
  name: string
}

export type StitchKind = 'Stitch' | 'Jump' | 'Trim' | 'ColorChange' | 'End'

export interface Stitch {
  x: number
  y: number
  kind: StitchKind
}

export interface StitchPlan {
  stitches: Stitch[]
  colors: ThreadColor[]
}

export type LayoutSpec = { kind: 'single_line' } | { kind: 'wrap'; max_width: number }

export interface Settings {
  min_text_height: number
  min_stroke_width: number
  pull_comp: number
  fill_angle: number
  row_spacing: number
  max_stitch: number
  jump_threshold: number
  trim_threshold: number
  edge_outline: boolean
}

export interface TextRequest {
  text: string
  font_id: string
  /** 字級，單位 0.1mm */
  height: number
  color: ThreadColor
  layout: LayoutSpec
  hoop: Hoop
  settings?: Partial<Settings>
}

export interface PreviewResult {
  plan: StitchPlan
  fitted_scale: number
  enlarge_scale: number
  adjustments: UserMessage[]
  blocked: UserMessage | null
}

export interface FontEntry {
  id: string
  name: string
}

export interface EncodedFile {
  fileName: string
  bytes: Uint8Array
}

export interface EncodeResult {
  files: EncodedFile[] | null
  error: UserMessage | null
}

export interface ImageRequest {
  width: number
  height: number
  /** 顏色多寡（2～12） */
  color_count: number
  /** 簡化程度（0～100） */
  simplify: number
  /** 圖長邊的大小，單位 0.1mm */
  size: number
  hoop: Hoop
  settings?: Partial<Settings>
}

// ── 多物件設計（編輯器）──

export type ShapeKind = 'circle' | 'rect' | 'heart' | 'star' | 'line'

/** 送進核心的物件。位置是物件中心離框中心的偏移（0.1mm）。 */
export type CompositeContent =
  | { kind: 'text'; text: string; font_id: string; height: number; color: ThreadColor; max_width?: number | null }
  | { kind: 'image'; image: number; width: number; height: number; color_count: number; simplify: number; size: number }
  | {
      kind: 'shape'
      shape: ShapeKind
      width: number
      height: number
      color: ThreadColor
      filled: boolean
      line_width: number
    }

export type CompositeItem = CompositeContent & {
  id: string
  x: number
  y: number
  rotation: number
  scale: number
}

export interface CompositeRequest {
  items: CompositeItem[]
  hoop: Hoop
  settings?: Partial<Settings>
}

export interface ItemInfo {
  id: string
  /** 縮放後的大小（0.1mm），還沒套用「整體縮小放進框」 */
  width: number
  height: number
  /** 實際用的縮放 */
  scale: number
  problem: UserMessage | null
}

export interface CompositeResult {
  preview: PreviewResult
  items: ItemInfo[]
}
