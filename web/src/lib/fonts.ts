// 字型檔（由 ../fonts 子集化產生，建置時複製到 public/fonts）。
// 基本字型（preload）跟著程式一起快取、開機就載入；其他字型第一次用到才下載，下載過之後離線也能用。
export const FONT_FILES = [
  { id: 'sans', file: 'NotoSansTC-subset.ttf', preload: true },
  { id: 'serif', file: 'NotoSerifTC-subset.ttf', preload: false },
  { id: 'round', file: 'jf-openhuninn-subset.ttf', preload: false },
  { id: 'kai', file: 'LXGWWenKaiTC-subset.ttf', preload: false },
] as const

export type FontId = (typeof FONT_FILES)[number]['id']

/** 開機就載入的字型。 */
export const PRELOADED_FONTS: readonly string[] = FONT_FILES.filter((f) => f.preload).map((f) => f.id)
