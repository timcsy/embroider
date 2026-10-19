import { expect, test } from '@playwright/test'
import { addText, open } from './helpers'

// 小螢幕手機（例如舊的 Android）與放大字體：畫布與主要按鈕都要看得到、按得到
const phones = [
  { name: '小螢幕手機', width: 360, height: 640 },
  { name: '更小的手機', width: 320, height: 568 },
]

for (const s of phones) {
  test(`${s.name}（${s.width}×${s.height}）：畫布、加東西、存檔都看得到，沒有橫向捲動`, async ({ page }) => {
    await page.setViewportSize({ width: s.width, height: s.height })
    await open(page)
    const t = await addText(page)
    await t.fill('Amy')
    await expect(page.getByTestId('size')).toBeVisible({ timeout: 30_000 })

    const overflow = await page.evaluate(() => document.documentElement.scrollWidth - window.innerWidth)
    expect(overflow).toBeLessThanOrEqual(1)

    // 畫布不會比螢幕寬、也不會高到把下面的工具全擠出畫面
    const stage = (await page.getByTestId('stage').boundingBox())!
    expect(stage.width).toBeLessThanOrEqual(s.width)
    expect(stage.height).toBeLessThanOrEqual(s.height * 0.55)

    // 存檔鈕固定在最下面、按得到
    const save = (await page.getByTestId('save').boundingBox())!
    expect(save.y + save.height).toBeLessThanOrEqual(s.height + 1)
    expect(save.height).toBeGreaterThanOrEqual(44)

    // 往下捲到色票：它不能被固定的畫布或存檔鈕蓋住（人會用手指捲到空檔；這裡用瀏覽器的置中捲動，會留出固定元素的空間）
    const swatch = page.getByRole('radio', { name: '寶藍' })
    await swatch.evaluate((el) => el.scrollIntoView({ block: 'center' }))
    const covered = await swatch.evaluate((el) => {
      const r = el.getBoundingClientRect()
      const top = document.elementFromPoint(r.x + r.width / 2, r.y + r.height / 2)
      return !(top && el.contains(top))
    })
    expect(covered, '色票被固定的畫布或存檔鈕蓋住了').toBe(false)
    await swatch.click({ force: true })
    await expect(swatch).toHaveAttribute('aria-checked', 'true')
  })
}

test('字放到最大：畫面仍不橫向捲動、按鈕沒有被切掉', async ({ page, context }) => {
  // 模擬使用者把系統字體調到最大（瀏覽器的文字縮放）
  await page.setViewportSize({ width: 412, height: 915 })
  await page.addInitScript(() => {
    document.documentElement.style.fontSize = '26px'
  })
  await open(page)
  await addText(page)
  const overflow = await page.evaluate(() => document.documentElement.scrollWidth - window.innerWidth)
  expect(overflow).toBeLessThanOrEqual(1)
  await expect(page.getByTestId('save')).toBeVisible()
  void context
})
