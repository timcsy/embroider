// @vitest-environment node
import { readFileSync } from 'node:fs'
import { describe, expect, it } from 'vitest'

// 放在子路徑（例如 GitHub Pages 的 /embroider/）也要能運作、能加到主畫面：
// 網址一律相對，不寫死根目錄 "/"。
const config = readFileSync(new URL('../vite.config.ts', import.meta.url), 'utf8')
const html = readFileSync(new URL('../index.html', import.meta.url), 'utf8')

describe('部署到子路徑', () => {
  it('建置的基準路徑是相對的', () => {
    expect(config).toMatch(/base:\s*'\.\/'/)
  })
  it('manifest 的起始網址、範圍與圖示都是相對的', () => {
    expect(config).not.toMatch(/start_url:\s*'\//)
    expect(config).not.toMatch(/scope:\s*'\//)
    expect(config).not.toMatch(/src:\s*'\/icons/)
  })
  it('離線備援頁是相對的', () => {
    expect(config).not.toMatch(/navigateFallback:\s*'\//)
  })
  it('index.html 的資源都是相對的', () => {
    expect(html).not.toMatch(/(href|src)="\/(?!\/)/)
  })
})

describe('頁面基本資訊', () => {
  it('語言是繁體中文，標題是產品名稱', () => {
    expect(html).toMatch(/<html lang="zh-TW"/)
    expect(html).toMatch(/<title>刺繡小幫手<\/title>/)
  })
})

// 新版本出來時，不能在使用者操作到一半時自己重新整理：
// 正在選圖、打字的內容都在畫面的記憶體裡，重新整理會全部消失、畫面跳回打字。
describe('更新不打斷使用', () => {
  it('不使用「自動更新」（它會在新版啟用時強制重新整理）', () => {
    expect(config).not.toMatch(/registerType:\s*'autoUpdate'/)
  })
  it('新版等到下次重新開啟才接手，不搶著接管正在用的頁面', () => {
    expect(config).not.toMatch(/skipWaiting:\s*true/)
    expect(config).not.toMatch(/clientsClaim:\s*true/)
  })
})
