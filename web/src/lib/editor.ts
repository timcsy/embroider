// 編輯器的資料與操作：一個設計由好幾個物件（文字、圖片、形狀）組成。
// 全部是不改動原物件的純函式，方便測試與復原。位置單位 0.1mm，原點在框中心。
import { cropRgba, type CropRect } from './crop'
import { fitImageSizeCm } from './fitSize'
import { THREADS } from './threads'
import type { CompositeItem, CompositeRequest, Hoop, ShapeKind } from './types'

interface Base {
  id: string
  /** 物件中心離框中心的偏移 */
  x: number
  y: number
  /** 度，順時針，-180 ~ 180 */
  rotation: number
  scale: number
}

export interface TextItem extends Base {
  kind: 'text'
  text: string
  fontId: string
  heightCm: number
  colorIdx: number
}

export interface ImageItem extends Base {
  kind: 'image'
  imageId: string
  colors: number
  simplify: number
  sizeCm: number
  crop: CropRect | null
}

export interface ShapeItem extends Base {
  kind: 'shape'
  shape: ShapeKind
  widthCm: number
  heightCm: number
  colorIdx: number
  filled: boolean
  /** 空心形狀與直線的粗細（公釐） */
  lineWidthMm: number
}

/** 手繪：好幾筆線，每一筆是相對於物件中心的點（0.1mm，縮放為 1 時）。 */
export interface DrawItem extends Base {
  kind: 'draw'
  strokes: [number, number][][]
  widthMm: number
  colorIdx: number
  /** 封起來的圖形要不要把中間填滿 */
  filled: boolean
}

export type EditorItem = TextItem | ImageItem | ShapeItem | DrawItem

export interface EditorDoc {
  items: EditorItem[]
  selectedId: string | null
  hoopId: string
}

/** 一張選進來的圖（像素很大，不放進復原紀錄）。 */
export interface ImageSource {
  id: string
  name: string
  width: number
  height: number
  rgba: Uint8Array
}

export const MIN_SCALE = 0.1
export const MAX_SCALE = 20
/** 連續新增或複製的物件，每個往右下錯開這麼多（0.1mm） */
const STAGGER = 120

export const emptyDoc = (hoopId: string): EditorDoc => ({ items: [], selectedId: null, hoopId })

const clamp = (v: number, lo: number, hi: number) => Math.min(hi, Math.max(lo, v))

function wrapAngle(deg: number): number {
  const r = ((((deg + 180) % 360) + 360) % 360) - 180
  return r === -180 ? 180 : r
}

function nextId(items: EditorItem[]): string {
  const n = items.reduce((m, i) => Math.max(m, Number(/(\d+)$/.exec(i.id)?.[1] ?? 0)), 0)
  return `item-${n + 1}`
}

/** 新物件放哪裡：中間；已經有東西就往右下錯開，避免疊在一起。 */
function spot(items: EditorItem[]): { x: number; y: number } {
  const k = items.length % 6
  return { x: k * STAGGER, y: k * STAGGER }
}

function put(doc: EditorDoc, make: (id: string, at: { x: number; y: number }) => EditorItem): EditorDoc {
  const id = nextId(doc.items)
  const item = make(id, spot(doc.items))
  return { ...doc, items: [...doc.items, item], selectedId: id }
}

const base = (id: string, at: { x: number; y: number }): Base => ({ id, ...at, rotation: 0, scale: 1 })

export function addText(doc: EditorDoc): EditorDoc {
  return put(doc, (id, at) => ({
    ...base(id, at),
    kind: 'text',
    text: '',
    fontId: 'sans',
    heightCm: 2.5,
    colorIdx: 0,
  }))
}

export function addShape(doc: EditorDoc, shape: ShapeKind): EditorDoc {
  return put(doc, (id, at) => ({
    ...base(id, at),
    kind: 'shape',
    shape,
    widthCm: shape === 'line' ? 6 : 4,
    heightCm: 4,
    colorIdx: 0,
    filled: true,
    lineWidthMm: shape === 'line' ? 2.5 : 3,
  }))
}

export function addImage(doc: EditorDoc, image: ImageSource, hoop: Hoop, colors = 4): EditorDoc {
  return put(doc, (id, at) => ({
    ...base(id, at),
    kind: 'image',
    imageId: image.id,
    colors,
    simplify: 30,
    sizeCm: fitImageSizeCm(hoop, image.width, image.height),
    crop: null,
  }))
}

export interface Brush {
  widthMm: number
  colorIdx: number
  filled: boolean
}

interface P {
  x: number
  y: number
}

const round1 = (v: number) => Math.round(v * 10) / 10

/** 畫面上（已整體縮小 fit 倍）的點，換成物件自己的座標。 */
function toLocal(it: Base, p: P, fit: number): [number, number] {
  const dx = p.x / fit - it.x
  const dy = p.y / fit - it.y
  const r = (-it.rotation * Math.PI) / 180
  const lx = (dx * Math.cos(r) - dy * Math.sin(r)) / it.scale
  const ly = (dx * Math.sin(r) + dy * Math.cos(r)) / it.scale
  return [lx, ly]
}

/** 第一筆：開一個手繪物件，中心放在這一筆的外框正中間。`points` 是畫面上的座標（0.1mm，已套用整體縮小）。 */
export function startDraw(doc: EditorDoc, points: P[], brush: Brush, fit: number): EditorDoc {
  const pts = points.map((p) => [p.x / fit, p.y / fit] as [number, number])
  const xs = pts.map((p) => p[0])
  const ys = pts.map((p) => p[1])
  const cx = (Math.min(...xs) + Math.max(...xs)) / 2
  const cy = (Math.min(...ys) + Math.max(...ys)) / 2
  return put(doc, (id) => ({
    id,
    x: round1(cx),
    y: round1(cy),
    rotation: 0,
    scale: 1,
    kind: 'draw',
    strokes: [pts.map(([x, y]) => [round1(x - round1(cx)), round1(y - round1(cy))] as [number, number])],
    ...brush,
  }))
}

/** 再畫一筆。新的點換成物件自己的座標，並把物件的中心重新放回全部筆畫的正中間（畫面上的位置不變）。 */
export function appendStroke(doc: EditorDoc, id: string, points: P[], fit: number): EditorDoc {
  const it = doc.items.find((i) => i.id === id)
  if (!it || it.kind !== 'draw' || points.length === 0) return doc
  const strokes = [...it.strokes, points.map((p) => toLocal(it, p, fit))]
  return recenterDraw(doc, it, strokes)
}

export function removeLastStroke(doc: EditorDoc, id: string): EditorDoc {
  const it = doc.items.find((i) => i.id === id)
  if (!it || it.kind !== 'draw') return doc
  const strokes = it.strokes.slice(0, -1)
  if (strokes.length === 0) return removeItem(doc, id).doc
  return recenterDraw(doc, it, strokes)
}

function recenterDraw(doc: EditorDoc, it: DrawItem, strokes: [number, number][][]): EditorDoc {
  const all = strokes.flat()
  const xs = all.map((p) => p[0])
  const ys = all.map((p) => p[1])
  const cx = (Math.min(...xs) + Math.max(...xs)) / 2
  const cy = (Math.min(...ys) + Math.max(...ys)) / 2
  const r = (it.rotation * Math.PI) / 180
  const [sx, sy] = [cx * it.scale, cy * it.scale]
  return updateItem(doc, it.id, {
    x: round1(it.x + sx * Math.cos(r) - sy * Math.sin(r)),
    y: round1(it.y + sx * Math.sin(r) + sy * Math.cos(r)),
    strokes: strokes.map((s) => s.map(([x, y]) => [round1(x - cx), round1(y - cy)] as [number, number])),
  } as Partial<EditorItem>)
}

export function updateItem(doc: EditorDoc, id: string, patch: Partial<EditorItem>): EditorDoc {
  return {
    ...doc,
    items: doc.items.map((it) => {
      if (it.id !== id) return it
      const next = { ...it, ...patch } as EditorItem
      next.scale = clamp(Number(next.scale) || MIN_SCALE, MIN_SCALE, MAX_SCALE)
      next.rotation = wrapAngle(next.rotation)
      return next
    }),
  }
}

export function selectItem(doc: EditorDoc, id: string | null): EditorDoc {
  return doc.selectedId === id ? doc : { ...doc, selectedId: id }
}

export function centerItem(doc: EditorDoc, id: string, axis: 'x' | 'y' | 'both'): EditorDoc {
  return updateItem(doc, id, {
    ...(axis !== 'y' ? { x: 0 } : {}),
    ...(axis !== 'x' ? { y: 0 } : {}),
  })
}

export function duplicateItem(doc: EditorDoc, id: string): EditorDoc {
  const src = doc.items.find((i) => i.id === id)
  if (!src) return doc
  const copy: EditorItem = { ...src, id: nextId(doc.items), x: src.x + STAGGER, y: src.y + STAGGER }
  const at = doc.items.indexOf(src)
  const items = [...doc.items]
  items.splice(at + 1, 0, copy)
  return { ...doc, items, selectedId: copy.id }
}

export interface Removed {
  item: EditorItem
  index: number
}

/** 刪掉一個物件，並回傳它被刪掉前的樣子與位置，讓畫面能「拿回來」。 */
export function removeItem(doc: EditorDoc, id: string): { doc: EditorDoc; removed: Removed | null } {
  const index = doc.items.findIndex((i) => i.id === id)
  if (index < 0) return { doc, removed: null }
  const item = doc.items[index]
  return {
    doc: {
      ...doc,
      items: doc.items.filter((i) => i.id !== id),
      selectedId: doc.selectedId === id ? null : doc.selectedId,
    },
    removed: { item, index },
  }
}

export function restoreRemoved(doc: EditorDoc, r: Removed): EditorDoc {
  if (doc.items.some((i) => i.id === r.item.id)) return doc
  const items = [...doc.items]
  items.splice(Math.min(r.index, items.length), 0, r.item)
  return { ...doc, items, selectedId: r.item.id }
}

/** 上下層：越後面的在越上面。 */
export function moveLayer(doc: EditorDoc, id: string, how: 'forward' | 'backward' | 'front' | 'back'): EditorDoc {
  const from = doc.items.findIndex((i) => i.id === id)
  if (from < 0) return doc
  const last = doc.items.length - 1
  const to = { forward: from + 1, backward: from - 1, front: last, back: 0 }[how]
  const clamped = clamp(to, 0, last)
  if (clamped === from) return doc
  const items = [...doc.items]
  const [it] = items.splice(from, 1)
  items.splice(clamped, 0, it)
  return { ...doc, items }
}

const cm = (v: number) => Math.round(v * 100)

/**
 * 組成送給核心的請求。圖片像素另外放在 `images`（同一張圖、同一個範圍只放一份）。
 * 找不到像素的圖片物件略過。
 */
export function toRequest(
  doc: EditorDoc,
  hoop: Hoop,
  sources: ImageSource[],
): { request: CompositeRequest; images: Uint8Array[]; ids: string[] } {
  const images: Uint8Array[] = []
  const slot = new Map<string, number>()
  const out: CompositeItem[] = []
  const ids: string[] = []
  for (const it of doc.items) {
    const placement = { id: it.id, x: it.x, y: it.y, rotation: it.rotation, scale: it.scale }
    if (it.kind === 'text') {
      out.push({
        ...placement,
        kind: 'text',
        text: it.text,
        font_id: it.fontId,
        height: cm(it.heightCm),
        color: THREADS[it.colorIdx] ?? THREADS[0],
      })
    } else if (it.kind === 'draw') {
      out.push({
        ...placement,
        kind: 'draw',
        strokes: it.strokes,
        width: Math.round(it.widthMm * 10),
        color: THREADS[it.colorIdx] ?? THREADS[0],
        filled: it.filled,
      })
    } else if (it.kind === 'shape') {
      out.push({
        ...placement,
        kind: 'shape',
        shape: it.shape,
        width: cm(it.widthCm),
        height: cm(it.heightCm),
        color: THREADS[it.colorIdx] ?? THREADS[0],
        filled: it.filled,
        line_width: Math.round(it.lineWidthMm * 10),
      })
    } else {
      const src = sources.find((s) => s.id === it.imageId)
      if (!src) continue
      const key = `${it.imageId}|${JSON.stringify(it.crop)}`
      const px = cropRgba(src.rgba, src.width, src.height, it.crop)
      let index = slot.get(key)
      if (index === undefined) {
        index = images.length
        slot.set(key, index)
        images.push(px.rgba)
      }
      out.push({
        ...placement,
        kind: 'image',
        image: index,
        width: px.width,
        height: px.height,
        color_count: it.colors,
        simplify: it.simplify,
        size: cm(it.sizeCm),
      })
    }
    ids.push(it.id)
  }
  return { request: { items: out, hoop }, images, ids }
}

/** 舊版（一次只有文字或一張圖）存下來的設計，轉成編輯器的一個物件。 */
export function fromLegacy(p: Record<string, unknown>, imageId?: string): EditorItem | null {
  const num = (v: unknown, d: number) => (typeof v === 'number' && Number.isFinite(v) ? v : d)
  const base = { id: 'item-1', x: 0, y: 0, rotation: 0, scale: 1 }
  if (p.mode === 'text') {
    return {
      ...base,
      kind: 'text',
      text: typeof p.text === 'string' ? p.text : '',
      fontId: typeof p.fontId === 'string' ? p.fontId : 'sans',
      heightCm: num(p.sizeCm, 2.5),
      colorIdx: num(p.colorIdx, 0),
    }
  }
  if (p.mode === 'image' && imageId) {
    return {
      ...base,
      kind: 'image',
      imageId,
      colors: num(p.colors, 4),
      simplify: num(p.simplify, 30),
      sizeCm: num(p.imageSizeCm, 8),
      crop: null,
    }
  }
  return null
}
