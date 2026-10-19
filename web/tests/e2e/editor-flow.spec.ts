import { expect, test, type Page } from '@playwright/test'
import { addText, open } from './helpers'

const box = async (page: Page, testid: string) => (await page.getByTestId(testid).boundingBox())!
const rectOf = (page: Page, id: string) => page.locator(`[data-testid="item-${id}"] rect`).boundingBox()

async function shape(page: Page, kind: string) {
  await page.getByTestId('add-shape').click()
  await page.getByTestId(`shape-${kind}`).click()
}

test('加一個形狀：出現在畫面上、被選取、可以存', async ({ page }) => {
  await open(page)
  await expect(page.getByTestId('save')).toBeDisabled()
  await shape(page, 'heart')
  await expect(page.locator('[data-testid^="item-item-"]')).toHaveCount(1)
  await expect(page.getByTestId('item-panel')).toHaveAttribute('data-kind', 'shape')
  await expect(page.getByTestId('size')).toBeVisible({ timeout: 30_000 })
  await expect(page.getByTestId('save')).toBeEnabled()
})

test('文字加形狀：兩個物件各有各的顏色', async ({ page }) => {
  await open(page)
  const t = await addText(page)
  await t.fill('Amy')
  await shape(page, 'circle')
  await page.getByRole('radio', { name: '寶藍' }).click()
  await expect(page.getByTestId('size')).toBeVisible({ timeout: 30_000 })
  const seen = () =>
    page.getByTestId('preview').evaluate((c: HTMLCanvasElement) => {
      const d = c.getContext('2d')!.getImageData(0, 0, c.width, c.height).data
      let red = 0
      let blue = 0
      for (let i = 0; i < d.length; i += 4) {
        if (d[i] > 150 && d[i + 1] < 90 && d[i + 2] < 90) red++
        if (d[i + 2] > 130 && d[i] < 90 && d[i + 1] < 120) blue++
      }
      return { red, blue }
    })
  await expect.poll(async () => (await seen()).blue, { timeout: 30_000 }).toBeGreaterThan(100)
  expect((await seen()).red).toBeGreaterThan(30)
})

test('用手指拖移：物件跟著動，放開後預覽也更新', async ({ page }) => {
  await open(page)
  await shape(page, 'rect')
  await expect(page.getByTestId('size')).toBeVisible({ timeout: 30_000 })
  const before = (await rectOf(page, 'item-1'))!
  const cx = before.x + before.width / 2
  const cy = before.y + before.height / 2
  await page.mouse.move(cx, cy)
  await page.mouse.down()
  await page.mouse.move(cx + 60, cy + 40, { steps: 6 })
  await page.mouse.up()
  const after = (await rectOf(page, 'item-1'))!
  expect(after.x - before.x).toBeGreaterThan(40)
  expect(after.y - before.y).toBeGreaterThan(25)
  await expect(page.getByTestId('size')).toBeVisible({ timeout: 30_000 })
})

test('拉角改大小、拉上方圓點轉方向', async ({ page }) => {
  await open(page)
  await shape(page, 'rect')
  await expect(page.getByTestId('size')).toBeVisible({ timeout: 30_000 })
  const r0 = (await rectOf(page, 'item-1'))!

  const corner = page.getByTestId('handle-resize').nth(2) // 右下角
  const cb = (await corner.boundingBox())!
  await page.mouse.move(cb.x + cb.width / 2, cb.y + cb.height / 2)
  await page.mouse.down()
  await page.mouse.move(cb.x + cb.width / 2 + 50, cb.y + cb.height / 2 + 50, { steps: 6 })
  await page.mouse.up()
  const r1 = (await rectOf(page, 'item-1'))!
  expect(r1.width).toBeGreaterThan(r0.width + 20)

  const rot = page.getByTestId('handle-rotate')
  const rb = (await rot.boundingBox())!
  const center = { x: r1.x + r1.width / 2, y: r1.y + r1.height / 2 }
  await page.mouse.move(rb.x + rb.width / 2, rb.y + rb.height / 2)
  await page.mouse.down()
  // 把圓點拉到正右邊 → 轉 90 度
  await page.mouse.move(center.x + 150, center.y, { steps: 8 })
  await page.mouse.up()
  const transform = await page.locator('[data-testid="item-item-1"]').getAttribute('transform')
  expect(transform).toMatch(/rotate\(90\)/)
})

test('點空白處取消選取；點物件再選回來', async ({ page }) => {
  await open(page)
  await shape(page, 'circle')
  await expect(page.getByTestId('item-panel')).toBeVisible()
  const stage = await box(page, 'stage')
  await page.mouse.click(stage.x + 8, stage.y + 8)
  await expect(page.getByTestId('item-panel')).toHaveCount(0)
  const r = (await rectOf(page, 'item-1'))!
  await page.mouse.click(r.x + r.width / 2, r.y + r.height / 2)
  await expect(page.getByTestId('item-panel')).toBeVisible()
})

test('刪掉不用確認，可以拿回來；上一步也能復原', async ({ page }) => {
  await open(page)
  await shape(page, 'star')
  await page.getByTestId('remove').click()
  await expect(page.locator('[data-testid^="item-item-"]')).toHaveCount(0)
  await expect(page.getByTestId('removed-note')).toContainText('已刪掉')
  await page.getByTestId('take-back').click()
  await expect(page.locator('[data-testid^="item-item-"]')).toHaveCount(1)
  await expect(page.getByTestId('item-panel')).toBeVisible()

  await page.getByRole('button', { name: '上一步' }).click() // 取消「拿回來」
  await expect(page.locator('[data-testid^="item-item-"]')).toHaveCount(0)
  await page.getByRole('button', { name: '上一步' }).click() // 取消「刪掉」
  await expect(page.locator('[data-testid^="item-item-"]')).toHaveCount(1)
  await page.getByRole('button', { name: '上一步' }).click() // 取消「加形狀」
  await expect(page.locator('[data-testid^="item-item-"]')).toHaveCount(0)
})

test('複製：多一個，位置錯開', async ({ page }) => {
  await open(page)
  await shape(page, 'heart')
  await page.getByTestId('duplicate').click()
  await expect(page.locator('[data-testid^="item-item-"]')).toHaveCount(2)
  const a = (await rectOf(page, 'item-1'))!
  const b = (await rectOf(page, 'item-2'))!
  expect(Math.abs(a.x - b.x) + Math.abs(a.y - b.y)).toBeGreaterThan(5)
})

test('鍵盤：方向鍵微調位置、刪除鍵刪掉', async ({ page }) => {
  await open(page)
  await shape(page, 'rect')
  await page.getByTestId('stage').focus()
  const before = (await rectOf(page, 'item-1'))!
  await page.keyboard.press('ArrowRight')
  await page.keyboard.press('ArrowRight')
  const after = (await rectOf(page, 'item-1'))!
  expect(after.x).toBeGreaterThan(before.x)
  await page.keyboard.press('Delete')
  await expect(page.locator('[data-testid^="item-item-"]')).toHaveCount(0)
})

test('轉向按鈕與擺正', async ({ page }) => {
  await open(page)
  await shape(page, 'rect')
  await page.getByRole('button', { name: /向右轉/ }).click()
  await expect(page.locator('[data-testid="item-item-1"]')).toHaveAttribute('transform', /rotate\(15\)/)
  await page.getByRole('button', { name: '擺正' }).click()
  await expect(page.locator('[data-testid="item-item-1"]')).toHaveAttribute('transform', /rotate\(0\)/)
})

// 手機、平板直放、平板橫放、電腦：每個畫面都要能操作、不能橫向捲動、按鈕夠大
const screens = [
  { name: '手機直放', width: 412, height: 915 },
  { name: '平板直放', width: 800, height: 1280 },
  { name: '平板橫放', width: 1280, height: 800 },
  { name: '電腦', width: 1440, height: 900 },
]

for (const s of screens) {
  test(`${s.name}（${s.width}×${s.height}）：能做完整流程，沒有橫向捲動，按鈕夠大`, async ({ page }) => {
    await page.setViewportSize({ width: s.width, height: s.height })
    await open(page)
    const t = await addText(page)
    await t.fill('王小明')
    await shape(page, 'heart')
    await expect(page.getByTestId('size')).toBeVisible({ timeout: 30_000 })
    await expect(page.getByTestId('save')).toBeEnabled()

    const overflow = await page.evaluate(() => document.documentElement.scrollWidth - window.innerWidth)
    expect(overflow, '不該有橫向捲動').toBeLessThanOrEqual(1)

    // 所有看得到的按鈕、選項、滑桿、輸入框，高度至少 44 像素
    const small = await page.evaluate(() => {
      const out: string[] = []
      const els = document.querySelectorAll('button, [role=radio], input[type=range], textarea')
      for (const el of els) {
        const r = (el as HTMLElement).getBoundingClientRect()
        const visible = r.width > 0 && r.height > 0 && getComputedStyle(el).visibility !== 'hidden'
        if (visible && r.height < 44) out.push(`${el.tagName} ${(el.textContent || el.getAttribute('aria-label') || '').trim().slice(0, 12)} ${Math.round(r.height)}px`)
      }
      return out
    })
    expect(small, `太小的按鈕：${small.join('、')}`).toEqual([])

    // 畫布與工具的位置：寬畫面左右並排，窄畫面上下排列
    const stage = await box(page, 'stage')
    const panel = (await page.getByTestId('item-panel').boundingBox())!
    if (s.width >= 900) expect(panel.x).toBeGreaterThan(stage.x + stage.width - 5)
    else expect(panel.y).toBeGreaterThan(stage.y + stage.height - 5)
    expect(stage.width).toBeGreaterThan(200)
    expect(stage.x).toBeGreaterThanOrEqual(0)
    expect(stage.x + stage.width).toBeLessThanOrEqual(s.width + 1)
  })
}
