// 下載字型檔，並且自己存進瀏覽器的快取：不必等離線服務啟用（第一次打開時它還沒接手），
// 下載過的字型之後斷網也能用。與離線服務的快取規則共用同一個快取名稱。

export const FONT_CACHE = 'fonts-on-demand'

export interface FontFetchEnv {
  caches: CacheStorage | undefined
  fetch: (url: string) => Promise<Response>
}

const browser = (): FontFetchEnv => ({
  caches: typeof caches === 'undefined' ? undefined : caches,
  fetch: (url) => fetch(url),
})

export async function fetchFontBytes(url: string, env: FontFetchEnv = browser()): Promise<Uint8Array> {
  const open = async () => {
    try {
      return (await env.caches?.open(FONT_CACHE)) ?? null
    } catch {
      return null // 私密瀏覽、儲存空間被擋：不用快取，照樣下載
    }
  }
  const cache = await open()
  try {
    const hit = await cache?.match(url)
    if (hit) return new Uint8Array(await hit.arrayBuffer())
  } catch {
    // 快取讀不出來就當作沒有
  }
  const res = await env.fetch(url)
  if (!res.ok) throw new Error(`字型下載失敗（${res.status}）`)
  try {
    await cache?.put(url, res.clone())
  } catch {
    // 存不進去也沒關係，這次還是能用
  }
  return new Uint8Array(await res.arrayBuffer())
}
