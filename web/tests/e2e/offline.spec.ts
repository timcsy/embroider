import { expect, test, type Page } from '@playwright/test'

async function ready(page: Page) {
  const box = page.getByRole('textbox', { name: '想繡什麼字？' })
  await expect(box).toBeVisible({ timeout: 30_000 })
  return box
}

/** 等 service worker 啟用、程式與字型都預先快取好。 */
async function waitUntilCached(page: Page) {
  await page.evaluate(() => navigator.serviceWorker.ready.then(() => true))
  await expect
    .poll(
      () =>
        page.evaluate(async () => {
          const names = await caches.keys()
          let total = 0
          for (const n of names) total += (await (await caches.open(n)).keys()).length
          return total
        }),
      { timeout: 60_000 },
    )
    .toBeGreaterThanOrEqual(15)
}

test('可以加到主畫面：有 manifest，名稱與圖示是繁體中文應用程式', async ({ page, request }) => {
  await page.goto('/')
  await ready(page)
  const href = await page.locator('link[rel=manifest]').getAttribute('href')
  expect(href).toBeTruthy()
  const m = await (await request.get(new URL(href!, page.url()).toString())).json()
  expect(m).toMatchObject({ name: '刺繡小幫手', display: 'standalone', lang: 'zh-TW' })
  expect(m.icons.map((i: { sizes: string }) => i.sizes)).toEqual(
    expect.arrayContaining(['192x192', '512x512']),
  )
})

test('載入一次後斷網：打字、預覽、存出都能做，自訂的框與預設格式仍在', async ({ page, context }) => {
  const downloads: string[] = []
  page.on('download', (d) => downloads.push(d.suggestedFilename()))

  await page.goto('/')
  await ready(page)
  await waitUntilCached(page)

  // 在有網路時先設定：新增自己的框、選定預設格式
  await page.getByTestId('hoop-add').click()
  const form = page.getByTestId('hoop-form')
  await form.getByLabel('框的名字').fill('離線測試框')
  await form.getByLabel('可以繡的寬（公釐）').fill('100')
  await form.getByLabel('可以繡的高（公釐）').fill('100')
  await page.getByTestId('hoop-save').click()
  await page.getByTestId('try-entry').click()
  await page.getByTestId('choose-dst').click()
  await page.getByRole('button', { name: /回去繼續做/ }).click()

  // 斷網後重新整理
  await context.setOffline(true)
  // 先證明斷網是真的：沒被快取的網址連不上，所以下面重新整理後還能用，才代表真的是從快取來的
  const reachable = await page.evaluate(() =>
    fetch(`/definitely-not-cached-${Date.now()}.txt`).then(
      () => 'reachable',
      () => 'unreachable',
    ),
  )
  expect(reachable).toBe('unreachable')
  await page.reload()
  const box = await ready(page)
  await expect(page.getByRole('radio', { name: /離線測試框/ })).toBeVisible()
  await expect(page.getByTestId('default-note')).toContainText('通用格式 A')

  await box.fill('Amy')
  await expect(page.getByTestId('size')).toBeVisible({ timeout: 30_000 })
  await page.getByTestId('save').click()
  await expect(page.getByTestId('export-sheet')).toBeVisible({ timeout: 30_000 })
  await expect.poll(() => downloads.length, { timeout: 15_000 }).toBe(1)
  expect(downloads[0]).toMatch(/\.DST$/)

  // 圖片流程斷網也能用
  await page.getByRole('button', { name: '好了' }).click()
  await page.getByTestId('tab-image').click()
  await page
    .getByTestId('file-input')
    .setInputFiles('tests/e2e/fixtures/logo.png')
  await expect(page.getByTestId('save')).toBeEnabled({ timeout: 60_000 })
})

test('最近做過的設計：存出後重新整理仍在，可以拿來用或刪掉', async ({ page }) => {
  await page.goto('/')
  const box = await ready(page)
  await expect(page.getByTestId('recent')).toHaveCount(0)

  await box.fill('王小明')
  await page.getByRole('radio', { name: '楷體' }).click()
  await expect(page.getByTestId('save')).toBeEnabled({ timeout: 30_000 })
  await page.getByTestId('save').click()
  await expect(page.getByTestId('export-sheet')).toBeVisible({ timeout: 30_000 })
  await page.getByRole('button', { name: '好了' }).click()
  await expect(page.getByTestId('recent')).toContainText('王小明')

  await page.reload()
  const box2 = await ready(page)
  await expect(page.getByTestId('recent')).toContainText('王小明')
  await expect(box2).toHaveValue('')

  await page.getByTestId('restore').click()
  await expect(box2).toHaveValue('王小明')
  await expect(page.getByRole('radio', { name: '楷體' })).toHaveAttribute('aria-checked', 'true')

  await page.getByRole('button', { name: /刪掉 王小明/ }).click()
  await expect(page.getByTestId('recent')).toHaveCount(0)
})
