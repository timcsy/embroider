import { expect, test } from '@playwright/test'
import { startText } from './helpers'

async function ready(page: import('@playwright/test').Page) {
  const box = await startText(page)
  return box
}

function collectDownloads(page: import('@playwright/test').Page) {
  const names: string[] = []
  page.on('download', (d) => names.push(d.suggestedFilename()))
  return names
}

test('存檔只輸出 DST，檔名是機器安全的（大寫英數、主檔名最多 8 字元）', async ({ page }) => {
  const names = collectDownloads(page)
  const box = await ready(page)
  await box.fill('Amy')
  await expect(page.getByTestId('save')).toBeEnabled({ timeout: 30_000 })
  await page.getByTestId('save').click()
  await expect.poll(() => names.length, { timeout: 15_000 }).toBe(1)
  expect(names[0]).toBe('AMY0001.DST')
  expect(names[0]).toMatch(/^[A-Z0-9]{1,8}\.DST$/)
})

test('「全部都存」也只有 DST', async ({ page }) => {
  const names = collectDownloads(page)
  const box = await ready(page)
  await box.fill('Amy')
  await expect(page.getByTestId('save-all')).toBeEnabled({ timeout: 30_000 })
  await page.getByTestId('save-all').click()
  await expect.poll(() => names.length, { timeout: 15_000 }).toBe(1)
  expect(names[0]).toMatch(/\.DST$/)
})

test('連續存兩次，檔名的序號會遞增', async ({ page }) => {
  const names = collectDownloads(page)
  const box = await ready(page)
  await box.fill('Amy')
  await expect(page.getByTestId('save')).toBeEnabled({ timeout: 30_000 })
  await page.getByTestId('save').click()
  await expect.poll(() => names.length, { timeout: 15_000 }).toBe(1)
  await page.getByRole('button', { name: '好了' }).click()
  await page.getByTestId('save').click()
  await expect.poll(() => names.length, { timeout: 15_000 }).toBe(2)
  expect(names.some((n) => n.startsWith('AMY0002'))).toBe(true)
})
