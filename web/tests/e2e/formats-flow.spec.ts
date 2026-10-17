import { expect, test } from '@playwright/test'

async function ready(page: import('@playwright/test').Page) {
  await page.goto('/')
  const box = page.getByRole('textbox', { name: '想繡什麼字？' })
  await expect(box).toBeVisible({ timeout: 30_000 })
  return box
}

function collectDownloads(page: import('@playwright/test').Page) {
  const names: string[] = []
  page.on('download', (d) => names.push(d.suggestedFilename()))
  return names
}

test('第一次存檔：每種格式都存，並說明原因；選定預設後只存所選，重開仍記得', async ({ page }) => {
  const names = collectDownloads(page)
  const box = await ready(page)
  await box.fill('Amy')
  await expect(page.getByTestId('save')).toBeEnabled({ timeout: 30_000 })
  await expect(page.getByTestId('default-note')).toContainText('每一種都存')

  await page.getByTestId('save').click()
  await expect(page.getByTestId('first-time')).toContainText('第一次')
  await expect.poll(() => names.length, { timeout: 15_000 }).toBe(2)
  expect([...names].sort()).toEqual(['AMY0001.DST', 'AMY0001.EXP'])
  // 檔名都是機器安全的：大寫英數、最多 8 字元主檔名
  for (const n of names) expect(n).toMatch(/^[A-Z0-9]{1,8}\.(DST|EXP)$/)

  // 去試試看，選定其中一種
  await page.getByTestId('try-next').click()
  await expect(page.getByRole('heading', { name: '試試看機器讀得到哪一種' })).toBeVisible()
  await expect(page.getByText('描繪範圍')).toBeVisible()
  await page.getByTestId('choose-exp').click()
  await expect(page.getByTestId('current-default')).toContainText('通用格式 B')
  await page.getByRole('button', { name: /回去繼續做/ }).click()

  // 回來後，輸入的內容還在
  await expect(box).toHaveValue('Amy')
  await expect(page.getByTestId('default-note')).toContainText('通用格式 B')

  names.length = 0
  await page.getByTestId('save').click()
  await expect(page.getByTestId('export-sheet')).toBeVisible()
  await expect(page.getByTestId('first-time')).toHaveCount(0)
  await expect.poll(() => names.length, { timeout: 15_000 }).toBe(1)
  expect(names[0]).toMatch(/\.EXP$/)
  await page.getByRole('button', { name: '好了' }).click()

  // 重新開啟後仍記得預設格式
  await page.reload()
  await expect(page.getByTestId('default-note')).toContainText('通用格式 B', { timeout: 30_000 })
})

test('「全部都存」一律輸出每一種格式，不受預設影響', async ({ page }) => {
  const names = collectDownloads(page)
  const box = await ready(page)
  await page.getByTestId('try-entry').click()
  await page.getByTestId('choose-dst').click()
  await page.getByRole('button', { name: /回去繼續做/ }).click()

  await box.fill('Amy')
  await expect(page.getByTestId('save-all')).toBeEnabled({ timeout: 30_000 })
  await page.getByTestId('save-all').click()
  await expect.poll(() => names.length, { timeout: 15_000 }).toBe(2)
  expect(names.map((n) => n.split('.')[1]).sort()).toEqual(['DST', 'EXP'])
})

test('可以改回「還不確定」，回到每一種都存', async ({ page }) => {
  await ready(page)
  await page.getByTestId('try-entry').click()
  await page.getByTestId('choose-dst').click()
  await expect(page.getByTestId('current-default')).toContainText('通用格式 A')
  await page.getByTestId('reset-default').click()
  await expect(page.getByTestId('current-default')).toContainText('每一種都存')
})

test('連續存兩次，檔名的序號會遞增', async ({ page }) => {
  const names = collectDownloads(page)
  const box = await ready(page)
  await box.fill('Amy')
  await expect(page.getByTestId('save')).toBeEnabled({ timeout: 30_000 })
  await page.getByTestId('save').click()
  await expect.poll(() => names.length, { timeout: 15_000 }).toBe(2)
  await page.getByRole('button', { name: '好了' }).click()
  await page.getByTestId('save').click()
  await expect.poll(() => names.length, { timeout: 15_000 }).toBe(4)
  expect(names.some((n) => n.startsWith('AMY0002'))).toBe(true)
})
