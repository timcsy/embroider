// 內嵌的字型檔（由 ../fonts 子集化產生，建置時複製到 public/fonts）。
export const FONT_FILES = [
  { id: 'sans', file: 'NotoSansTC-subset.ttf' },
  { id: 'serif', file: 'NotoSerifTC-subset.ttf' },
  { id: 'round', file: 'jf-openhuninn-subset.ttf' },
  { id: 'kai', file: 'LXGWWenKaiTC-subset.ttf' },
] as const

export type FontId = (typeof FONT_FILES)[number]['id']
