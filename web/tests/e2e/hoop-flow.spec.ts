import { expect, test } from '@playwright/test'
import { startText } from './helpers'

async function ready(page: import('@playwright/test').Page) {
  const box = await startText(page)
  return box
}

test('小框放不下長文字：被縮小或擋下並用白話說明', async ({ page }) => {
  const box = await ready(page)
  await page.getByRole('radio', { name: /小框/ }).click()
  await box.fill('王小明')
  // 小框很小，預設大小會被自動縮小，並說明
  await expect(page.getByText('已幫你縮小')).toBeVisible({ timeout: 30_000 })
  await expect(page.getByTestId('save')).toBeEnabled()

  // 七個字：小框（可繡約 4 公分寬）放不下，擋下並說明
  await box.fill('王小明的刺繡作')
  await expect(page.getByRole('alert')).toContainText('放不進', { timeout: 30_000 })
  await expect(page.getByRole('alert')).toContainText('比較大的框')
  await expect(page.getByTestId('save')).toBeDisabled()

  // 改選大框（可繡約 15 公分寬），同一段文字縮小一點就能存
  await page.getByRole('radio', { name: /大框/ }).click()
  await expect(page.getByTestId('save')).toBeEnabled({ timeout: 30_000 })
  await expect(page.getByText('已幫你縮小')).toBeVisible()

  // 長句子：一行放不下，自動分成幾行，不用擔心
  await box.fill('王小明的刺繡作品展覽很長的一段文字')
  await expect(page.getByText('已經幫你分成幾行')).toBeVisible({ timeout: 30_000 })
  await expect(page.getByTestId('save')).toBeEnabled()

  // 長到連分行也放不下：擋下並說明
  await box.fill('王小明的刺繡作品展覽很長的一段文字'.repeat(6))
  await expect(page.getByRole('alert')).toContainText('放不進', { timeout: 30_000 })
  await expect(page.getByTestId('save')).toBeDisabled()
})

test('新增 10×10 公分的框，重新整理後仍在，可以選用', async ({ page }) => {
  const box = await ready(page)
  await page.getByTestId('hoop-add').click()
  const form = page.getByTestId('hoop-form')
  await expect(form).toContainText('量框裡面可以繡的範圍')
  await form.getByLabel('框的名字').fill('我的 10×10 方框')
  await form.getByLabel('可以繡的寬（公釐）').fill('100')
  await form.getByLabel('可以繡的高（公釐）').fill('100')
  await page.getByTestId('hoop-save').click()
  await expect(form).toBeHidden()
  await expect(page.getByRole('radio', { name: /我的 10×10 方框/ })).toHaveAttribute('aria-checked', 'true')
  await box.fill('Amy')
  await expect(page.getByTestId('size')).toBeVisible({ timeout: 30_000 })

  await page.reload()
  await expect(page.getByRole('radio', { name: /我的 10×10 方框/ })).toBeVisible({ timeout: 30_000 })
})

test('新增框時填錯會用白話提示，不會關掉表單', async ({ page }) => {
  await ready(page)
  await page.getByTestId('hoop-add').click()
  const form = page.getByTestId('hoop-form')
  await form.getByLabel('框的名字').fill('壞的')
  await form.getByLabel('可以繡的寬（公釐）').fill('abc')
  await form.getByLabel('可以繡的高（公釐）').fill('100')
  await page.getByTestId('hoop-save').click()
  await expect(form).toBeVisible()
  await expect(form.getByRole('alert')).toContainText('寬度要填數字')
  // 太大的數字由核心判斷
  await form.getByLabel('可以繡的寬（公釐）').fill('9999')
  await page.getByTestId('hoop-save').click()
  await expect(form.getByRole('alert')).toBeVisible()
  await expect(form).toBeVisible()
})

test('原廠框沒有刪除與修改按鈕；自己的框要點兩次才會刪除', async ({ page }) => {
  await ready(page)
  await page.getByRole('radio', { name: /小框/ }).click()
  await expect(page.getByTestId('hoop-remove')).toHaveCount(0)

  await page.getByTestId('hoop-add').click()
  const form = page.getByTestId('hoop-form')
  await form.getByLabel('框的名字').fill('暫時的框')
  await form.getByLabel('可以繡的寬（公釐）').fill('80')
  await form.getByLabel('可以繡的高（公釐）').fill('80')
  await page.getByTestId('hoop-save').click()
  await expect(page.getByTestId('hoop-remove')).toBeVisible()
  await page.getByTestId('hoop-remove').click()
  await expect(page.getByRole('radio', { name: /暫時的框/ })).toBeVisible() // 還沒刪
  await page.getByTestId('hoop-remove-confirm').click()
  await expect(page.getByRole('radio', { name: /暫時的框/ })).toHaveCount(0)
})
