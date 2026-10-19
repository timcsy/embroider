import { expect, type Page } from '@playwright/test'

/** 打開頁面、按「加文字」，回傳文字輸入框。 */
export async function startText(page: Page) {
  await page.goto('/')
  await expect(page.getByTestId('add-text')).toBeVisible({ timeout: 30_000 })
  return addText(page)
}

/** 在已經打開的頁面再加一個文字物件。 */
export async function addText(page: Page) {
  await expect(page.getByTestId('add-text')).toBeVisible({ timeout: 30_000 })
  await page.getByTestId('add-text').click()
  const box = page.getByRole('textbox', { name: '想繡什麼字？' })
  await expect(box).toBeVisible()
  return box
}

/** 打開頁面等到可以操作。 */
export async function open(page: Page) {
  await page.goto('/')
  await expect(page.getByTestId('add-text')).toBeVisible({ timeout: 30_000 })
}
