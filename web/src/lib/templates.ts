// 起手式範本：一組預先擺好的物件，套用後都是普通物件，可以繼續改。
// 空白的畫面對不熟電腦的人是最大的門檻，所以先給幾個最常用的（名字標籤、圓形標籤、花邊）。
import { addFrame, addShape, addText, repeatRow, updateItem, type EditorDoc, type EditorItem } from './editor'
import type { Hoop } from './types'

export interface Template {
  id: string
  name: string
  description: string
}

export const TEMPLATES: Template[] = [
  { id: 'name-tag', name: '名字標籤', description: '一個大大的名字，下面一行小字，可以寫日期或「手作」。' },
  { id: 'round-tag', name: '圓形標籤', description: '一個圓框，裡面的字繞著圓排。' },
  { id: 'border', name: '花邊', description: '一排小愛心，沿著框的上緣。' },
]

/** 框可繡範圍的一半（0.1mm）；圓形與橢圓只能放下內接的方形。 */
function safeHalf(h: Hoop) {
  const inner = h.shape === 'rect' ? 1 : Math.SQRT1_2
  return { w: ((h.width - 2 * h.margin) / 2) * inner, h: ((h.height - 2 * h.margin) / 2) * inner }
}

function put(doc: EditorDoc, id: string, patch: Partial<EditorItem>): EditorDoc {
  return updateItem(doc, id, patch)
}

/** 套用範本：新的物件接在已有的後面。名字標籤會選到第一行字，讓她直接打字。 */
export function applyTemplate(doc: EditorDoc, templateId: string, hoop: Hoop): EditorDoc {
  const half = safeHalf(hoop)
  const cmWide = (half.w * 2) / 100
  if (templateId === 'name-tag') {
    let d = addText(doc)
    const nameId = d.selectedId as string
    d = put(d, nameId, { x: 0, y: -half.h * 0.12, heightCm: Math.min(3, cmWide / 3), text: '' } as Partial<EditorItem>)
    d = addText(d)
    const lineId = d.selectedId as string
    d = put(d, lineId, { x: 0, y: half.h * 0.22, heightCm: Math.min(1.2, cmWide / 8), text: '' } as Partial<EditorItem>)
    return { ...d, selectedId: nameId, extraIds: [] }
  }
  if (templateId === 'round-tag') {
    const side = Math.min(half.w, half.h) * 2 - 100
    let d = addFrame(doc, 'circle', hoop)
    const ring = d.selectedId as string
    d = put(d, ring, { widthCm: side / 100, heightCm: side / 100, x: 0, y: 0 } as Partial<EditorItem>)
    d = addText(d)
    const textId = d.selectedId as string
    d = put(d, textId, {
      x: 0,
      y: 0,
      heightCm: Math.min(2, side / 100 / 5),
      curve: 150,
      text: '',
    } as Partial<EditorItem>)
    return { ...d, selectedId: textId, extraIds: [] }
  }
  if (templateId === 'border') {
    // 先決定幾個，再讓每個的大小剛好排滿一排（含間隔），不超出框
    const width = half.w * 2
    const count = Math.max(5, Math.min(12, Math.floor(width / (300 * 1.3))))
    const size = Math.min(300, width / (count * 1.3))
    const gap = size * 0.3
    let d = addShape(doc, 'heart')
    const id = d.selectedId as string
    d = put(d, id, { x: 0, y: -half.h + size / 2 + 20, widthCm: size / 100, heightCm: size / 100, colorIdx: 1 } as Partial<EditorItem>)
    d = repeatRow(d, id, { count, gap, axis: 'x', size: { width: size, height: size } })
    return { ...d, selectedId: id, extraIds: [] }
  }
  return doc
}
