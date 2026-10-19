import { expect, test, type Page } from '@playwright/test'
import { addText } from './helpers'

// 這些測試要用到離線服務；在 localhost 預設不註冊，所以先打開旗標
test.beforeEach(async ({ page }) => {
  await page.addInitScript(() => localStorage.setItem('embroider.sw', 'on'))
})


const fontRequests = (page: Page) => {
  const urls: string[] = []
  page.on('request', (r) => {
    if (r.url().endsWith('.ttf')) urls.push(r.url().split('/').pop() ?? '')
  })
  return urls
}

test('開機只載入黑體；選楷體才下載，下載好就能用，再選不會重抓', async ({ page }) => {
  const urls = fontRequests(page)
  await page.goto('/')
  const box = await addText(page)
  await box.fill('王小明')
  await expect(page.getByTestId('size')).toBeVisible({ timeout: 30_000 })
  // 一開始只有黑體
  expect(urls.every((u) => u.startsWith('NotoSansTC'))).toBe(true)
  await expect(page.getByTestId('font-kai')).toContainText('要下載')
  await expect(page.getByTestId('font-sans')).not.toContainText('要下載')

  await page.getByTestId('font-kai').click()
  await expect(page.getByTestId('font-kai')).toHaveAttribute('aria-checked', 'true', { timeout: 30_000 })
  await expect(page.getByTestId('font-kai')).not.toContainText('要下載')
  expect(urls.filter((u) => u.startsWith('LXGW')).length).toBe(1)
  await expect(page.getByTestId('size')).toBeVisible({ timeout: 30_000 })
  await expect(page.getByRole('alert')).toHaveCount(0)

  // 換回黑體再換楷體：不會再下載一次
  await page.getByTestId('font-sans').click()
  await page.getByTestId('font-kai').click()
  await expect(page.getByTestId('font-kai')).toHaveAttribute('aria-checked', 'true')
  expect(urls.filter((u) => u.startsWith('LXGW')).length).toBe(1)
})

test('下載不了（沒網路）：用白話說明、維持原本的字型、之後可以再試', async ({ page }) => {
  await page.goto('/')
  const box = await addText(page)
  await box.fill('王小明')
  await expect(page.getByTestId('size')).toBeVisible({ timeout: 30_000 })

  await page.route('**/*.ttf', (route) => (route.request().url().includes('NotoSerif') ? route.abort() : route.continue()))
  await page.getByTestId('font-serif').click()
  await expect(page.getByRole('alert')).toContainText('要先連上網路下載一次', { timeout: 30_000 })
  await expect(page.getByRole('alert')).toContainText('下載過一次之後')
  await expect(page.getByTestId('font-sans')).toHaveAttribute('aria-checked', 'true')
  await expect(page.getByTestId('font-serif')).toContainText('連不上網')

  // 恢復連線後再按一次，就成功了
  await page.unroute('**/*.ttf')
  await page.getByTestId('font-serif').click()
  await expect(page.getByTestId('font-serif')).toHaveAttribute('aria-checked', 'true', { timeout: 30_000 })
  await expect(page.getByRole('alert')).toHaveCount(0)
})

test('下載過的字型，之後斷網重新整理也能用（做到一半的設計接著做）', async ({ page, context }) => {
  await page.goto('/')
  await page.evaluate(() => navigator.serviceWorker.ready.then(() => true))
  const box = await addText(page)
  await box.fill('王小明')
  await page.getByTestId('font-kai').click()
  await expect(page.getByTestId('font-kai')).toHaveAttribute('aria-checked', 'true', { timeout: 30_000 })
  await expect(page.getByTestId('size')).toBeVisible({ timeout: 30_000 })
  await page.waitForTimeout(1500) // 自動儲存
  // 字型要已經進了快取
  await expect
    .poll(() => page.evaluate(async () => (await caches.has('fonts-on-demand')) && (await (await caches.open('fonts-on-demand')).keys()).length), { timeout: 15_000 })
    .toBeGreaterThanOrEqual(1)

  await context.setOffline(true)
  await page.reload()
  await expect(page.getByTestId('resumed-note')).toBeVisible({ timeout: 30_000 })
  await expect(page.getByTestId('size')).toBeVisible({ timeout: 30_000 })
  // 沒有出現「先用黑體顯示」的說明：楷體是從快取來的
  await expect(page.getByText('先用黑體顯示')).toHaveCount(0)
  await expect(page.getByTestId('save')).toBeEnabled()
  await context.setOffline(false)
})

test('還沒下載、又沒網路時打開做到一半的楷體設計：先用黑體顯示並說明，不會當機', async ({ page, context }) => {
  await page.goto('/')
  await page.evaluate(() => navigator.serviceWorker.ready.then(() => true))
  const box = await addText(page)
  await box.fill('王小明')
  await page.getByTestId('font-serif').click()
  await expect(page.getByTestId('font-serif')).toHaveAttribute('aria-checked', 'true', { timeout: 30_000 })
  await page.waitForTimeout(1500)
  // 把快取裡的字型清掉，模擬「這支手機上沒有這個字型」
  await page.evaluate(async () => {
    await caches.delete('fonts-on-demand')
  })
  await context.setOffline(true)
  await page.reload()
  await expect(page.getByText('先用黑體顯示')).toBeVisible({ timeout: 30_000 })
  await expect(page.getByTestId('save')).toBeEnabled({ timeout: 30_000 })
  await context.setOffline(false)
})
