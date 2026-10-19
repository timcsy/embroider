import { createServer, type Server } from 'node:http'
import { cp, mkdtemp, readFile, appendFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { extname, join, normalize } from 'node:path'
import { expect, test, type Page } from '@playwright/test'

// 新版本的更新：開著的人不能一直停在舊版。
// 做法：把建好的程式複製一份當伺服器，載入之後，把 sw.js 改成「新版」（多一段會回報版本的程式），
// 看畫面是不是發現了、更新了，而且做到一半的設計還在。
const TYPES: Record<string, string> = {
  '.html': 'text/html; charset=utf-8',
  '.js': 'text/javascript',
  '.css': 'text/css',
  '.wasm': 'application/wasm',
  '.svg': 'image/svg+xml',
  '.png': 'image/png',
  '.ttf': 'font/ttf',
  '.webmanifest': 'application/manifest+json',
}

let server: Server
let origin = ''
let dir = ''

test.beforeEach(async () => {
  dir = await mkdtemp(join(tmpdir(), 'embroider-dist-'))
  await cp(join(process.cwd(), 'dist'), dir, { recursive: true })
  server = createServer(async (req, res) => {
    const path = decodeURIComponent((req.url ?? '/').split('?')[0])
    try {
      const file = normalize(join(dir, path === '/' ? 'index.html' : path))
      if (!file.startsWith(dir)) throw new Error('out')
      const body = await readFile(file)
      res.writeHead(200, { 'content-type': TYPES[extname(file)] ?? 'application/octet-stream', 'cache-control': 'no-store' }).end(body)
    } catch {
      res.writeHead(404).end()
    }
  })
  await new Promise<void>((ok) => server.listen(0, '127.0.0.1', ok))
  const addr = server.address()
  origin = `http://127.0.0.1:${typeof addr === 'object' && addr ? addr.port : 0}`
})
test.afterEach(() => server.close())

/** 把 sw.js 換成「新版」：多一段會回答自己是哪一版的程式。 */
async function releaseNewVersion() {
  await appendFile(
    join(dir, 'sw.js'),
    `\n;self.addEventListener('message', (e) => { if (e.data === 'which-version') e.source.postMessage('v2') })\n`,
  )
}

async function checkNow(page: Page) {
  await page.evaluate(async () => {
    const r = await navigator.serviceWorker.getRegistration()
    await r?.update()
  })
}

async function version(page: Page): Promise<string> {
  // 頁面可能剛好在重新整理（自動更新），讀不到就當作還沒好
  return page
    .evaluate(
    () =>
      new Promise<string>((resolve) => {
        const c = navigator.serviceWorker.controller
        if (!c) return resolve('沒有接手')
        navigator.serviceWorker.addEventListener('message', (e) => resolve(String(e.data)), { once: true })
        c.postMessage('which-version')
        setTimeout(() => resolve('v1'), 1500)
      }),
    )
    .catch(() => '重新整理中')
}

test('做到一半時有新版本：出現提示，按「馬上更新」之後換成新版，設計還在', async ({ page }) => {
  await page.goto(`${origin}/`)
  await page.evaluate(() => navigator.serviceWorker.ready.then(() => true))
  await expect(page.getByTestId('add-shape')).toBeVisible({ timeout: 30_000 })
  await page.getByTestId('add-shape').click()
  await page.getByTestId('shape-heart').click()
  await expect(page.locator('[data-testid^="item-item-"]')).toHaveCount(1)
  await page.waitForTimeout(1200) // 自動儲存
  // 重新整理一次，讓離線服務接手
  await page.reload()
  await expect(page.locator('[data-testid^="item-item-"]')).toHaveCount(1, { timeout: 30_000 })
  expect(await version(page)).toBe('v1')

  await releaseNewVersion()
  await checkNow(page)
  await expect(page.getByTestId('update-banner')).toBeVisible({ timeout: 30_000 })
  // 做到一半的時候不會擅自重新整理
  await expect(page.locator('[data-testid^="item-item-"]')).toHaveCount(1)

  await page.getByTestId('update-now').click()
  await expect(page.getByTestId('resumed-note')).toBeVisible({ timeout: 30_000 })
  await expect(page.locator('[data-testid^="item-item-"]')).toHaveCount(1)
  await expect(page.getByTestId('update-banner')).toHaveCount(0)
  await expect.poll(() => version(page), { timeout: 15_000 }).toBe('v2')
})

test('沒有做到一半的東西時，新版本自動更新', async ({ page }) => {
  await page.goto(`${origin}/`)
  await page.evaluate(() => navigator.serviceWorker.ready.then(() => true))
  await page.reload()
  await expect(page.getByTestId('add-text')).toBeVisible({ timeout: 30_000 })
  expect(await version(page)).toBe('v1')
  await releaseNewVersion()
  await checkNow(page)
  await expect.poll(() => version(page), { timeout: 30_000, intervals: [300, 500, 1000] }).toBe('v2')
  await expect(page.getByTestId('add-text')).toBeVisible({ timeout: 30_000 })
})

test('畫面上看得到版本編號（才分得出開的是不是最新的）', async ({ page }) => {
  await page.goto(`${origin}/`)
  await expect(page.getByTestId('build-id')).toContainText('版本', { timeout: 30_000 })
  await expect(page.getByTestId('build-id')).toContainText('20')
})
