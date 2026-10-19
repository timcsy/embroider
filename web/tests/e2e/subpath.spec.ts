import { createServer, type Server } from 'node:http'
import { readFile } from 'node:fs/promises'
import { extname, join, normalize } from 'node:path'
import { expect, test } from '@playwright/test'
import { addText } from './helpers'

// 放在子路徑（例如 GitHub Pages 的 /embroider/）時，程式、字型、Worker 都要找得到。
const DIST = join(process.cwd(), 'dist')
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

test.beforeAll(async () => {
  server = createServer(async (req, res) => {
    const path = decodeURIComponent((req.url ?? '/').split('?')[0])
    if (!path.startsWith('/embroider/')) {
      res.writeHead(404).end()
      return
    }
    const rel = path.slice('/embroider/'.length) || 'index.html'
    try {
      const file = normalize(join(DIST, rel))
      if (!file.startsWith(DIST)) throw new Error('out of dist')
      const body = await readFile(file)
      res.writeHead(200, { 'content-type': TYPES[extname(file)] ?? 'application/octet-stream' }).end(body)
    } catch {
      res.writeHead(404).end()
    }
  })
  await new Promise<void>((ok) => server.listen(0, '127.0.0.1', ok))
  const addr = server.address()
  origin = `http://127.0.0.1:${typeof addr === 'object' && addr ? addr.port : 0}`
})

test.afterAll(() => server.close())

test('放在子路徑也能打字、看到預覽（字型由 Worker 載入）', async ({ page }) => {
  const failed: string[] = []
  page.on('response', (r) => {
    if (r.status() >= 400) failed.push(`${r.status()} ${r.url()}`)
  })
  await page.goto(`${origin}/embroider/`)
  const box = await addText(page)
  await box.fill('王小明')
  await expect(page.getByTestId('size')).toContainText('公分', { timeout: 30_000 })
  await expect(page.getByRole('alert')).toHaveCount(0)
  expect(failed).toEqual([])
})
