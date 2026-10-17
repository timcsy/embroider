// 最近的設計：存在這支手機的瀏覽器裡（IndexedDB），重開後仍在。最多十個，同一個設計只留一份。
// 儲存空間不能用（私密瀏覽、被清掉）時，所有函式都不會丟例外：存失敗、列出為空。
import { clear, createStore, del, entries, set } from 'idb-keyval'

const MAX_RECENT = 10

export interface SavedImage {
  name: string
  width: number
  height: number
  /** 縮小後的圖（PNG 位元組；用位元組而不是 Blob，各種環境都能可靠儲存） */
  png: Uint8Array
}

export interface SavedDesign {
  /** 同一個設計的識別（例如 text:王小明），再存一次會更新而不是新增 */
  id: string
  name: string
  savedAt: number
  /** 畫面上的設定（模式、字型、大小、顏色、框…） */
  params: Record<string, unknown>
  image?: SavedImage
}

export type NewDesign = Omit<SavedDesign, 'savedAt'>

// 每次都重新開啟，這樣儲存空間壞掉時下一次呼叫會重新嘗試
const store = () => createStore('embroider', 'recent')

export async function listRecent(): Promise<SavedDesign[]> {
  try {
    const all = (await entries(store())) as [string, SavedDesign][]
    return all.map(([, v]) => v).sort((a, b) => b.savedAt - a.savedAt)
  } catch {
    return []
  }
}

export async function saveRecent(design: NewDesign, now = Date.now()): Promise<boolean> {
  try {
    const s = store()
    await set(design.id, { ...design, savedAt: now }, s)
    const all = (await entries(s)) as [string, SavedDesign][]
    const extra = all.sort((a, b) => b[1].savedAt - a[1].savedAt).slice(MAX_RECENT)
    for (const [key] of extra) await del(key, s)
    return true
  } catch {
    return false
  }
}

export async function removeRecent(id: string): Promise<void> {
  try {
    await del(id, store())
  } catch {
    // 沒關係
  }
}

export async function clearRecent(): Promise<void> {
  try {
    await clear(store())
  } catch {
    // 沒關係
  }
}
