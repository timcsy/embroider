// 字型等靜態檔的網址。基準路徑是相對的（可放在任何子路徑），
// 所以必須以「頁面」的網址換算；Worker 自己的位置在 assets/ 底下，不能拿來當基準。
export function fontsBaseUrl(base: string, pageUrl: string): string {
  return new URL(`${base}fonts/`, pageUrl).href
}

export const pageFontsBase = (): string => fontsBaseUrl(import.meta.env.BASE_URL, document.baseURI)
