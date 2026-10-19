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

// ── 手繪 ──
async function drawPath(page: Page, pts: [number, number][]) {
  const stage = await box(page, 'stage')
  const at = ([fx, fy]: [number, number]) => ({ x: stage.x + stage.width * fx, y: stage.y + stage.height * fy })
  const first = at(pts[0])
  await page.mouse.move(first.x, first.y)
  await page.mouse.down()
  for (const p of pts.slice(1)) {
    const q = at(p)
    await page.mouse.move(q.x, q.y, { steps: 3 })
  }
  await page.mouse.up()
}

test('畫一筆：變成一個手繪物件，畫好了可以存', async ({ page }) => {
  await open(page)
  await page.getByTestId('add-draw').click()
  await expect(page.getByTestId('draw-panel')).toBeVisible()
  await drawPath(page, [[0.3, 0.4], [0.5, 0.5], [0.7, 0.4]])
  await expect(page.locator('[data-testid^="item-item-"]')).toHaveCount(1)
  await expect(page.getByTestId('size')).toBeVisible({ timeout: 30_000 })
  await page.getByTestId('draw-done').click()
  await expect(page.getByTestId('draw-panel')).toHaveCount(0)
  await expect(page.getByTestId('item-panel')).toHaveAttribute('data-kind', 'draw')
  await expect(page.getByTestId('save')).toBeEnabled()
})

test('連畫兩筆算同一個物件；擦掉上一筆、全擦掉就沒有物件', async ({ page }) => {
  await open(page)
  await page.getByTestId('add-draw').click()
  await drawPath(page, [[0.3, 0.3], [0.7, 0.3]])
  await drawPath(page, [[0.3, 0.6], [0.7, 0.6]])
  await expect(page.locator('[data-testid^="item-item-"]')).toHaveCount(1)
  await expect(page.getByTestId('size')).toBeVisible({ timeout: 30_000 })
  await page.getByTestId('draw-undo').click()
  await expect(page.locator('[data-testid^="item-item-"]')).toHaveCount(1)
  await page.getByTestId('draw-undo').click()
  await expect(page.locator('[data-testid^="item-item-"]')).toHaveCount(0)
})

test('畫畫時不會誤拖到別的東西', async ({ page }) => {
  await open(page)
  await shape(page, 'rect')
  const before = (await rectOf(page, 'item-1'))!
  await page.getByTestId('add-draw').click()
  const cx = (before.x + before.width / 2) / 1
  const cy = before.y + before.height / 2
  await page.mouse.move(cx, cy)
  await page.mouse.down()
  await page.mouse.move(cx + 40, cy + 20, { steps: 4 })
  await page.mouse.up()
  const after = (await rectOf(page, 'item-1'))!
  expect(Math.abs(after.x - before.x)).toBeLessThan(2)
  await expect(page.locator('[data-testid^="item-item-"]')).toHaveCount(2)
})

test('畫一個圈：選「填滿」中間才有繡線', async ({ page }) => {
  await open(page)
  await page.getByTestId('add-draw').click()
  const ring: [number, number][] = Array.from({ length: 41 }, (_, i) => {
    const a = (Math.PI * 2 * i) / 40
    return [0.5 + 0.3 * Math.cos(a), 0.4 + 0.2 * Math.sin(a)] as [number, number]
  })
  await drawPath(page, ring)
  await expect(page.getByTestId('size')).toBeVisible({ timeout: 30_000 })
  const centerRed = () =>
    page.getByTestId('preview').evaluate((c: HTMLCanvasElement) => {
      const ctx = c.getContext('2d')!
      const x = Math.round(c.width * 0.5)
      const y = Math.round(c.height * 0.4)
      const d = ctx.getImageData(x - 6, y - 6, 12, 12).data
      let red = 0
      for (let i = 0; i < d.length; i += 4) if (d[i] > 150 && d[i + 1] < 100 && d[i + 2] < 100) red++
      return red
    })
  expect(await centerRed()).toBe(0)
  await page.getByRole('radio', { name: '填滿' }).or(page.getByRole('radio', { name: '實心' })).first().click()
  await expect.poll(centerRed, { timeout: 30_000 }).toBeGreaterThan(20)
})

test('平板上也能畫畫', async ({ page }) => {
  await page.setViewportSize({ width: 1280, height: 800 })
  await open(page)
  await page.getByTestId('add-draw').click()
  await drawPath(page, [[0.3, 0.4], [0.6, 0.5], [0.4, 0.7]])
  await expect(page.getByTestId('size')).toBeVisible({ timeout: 30_000 })
  await page.getByTestId('draw-done').click()
  await expect(page.getByTestId('save')).toBeEnabled()
})

// ── 拼布與刺繡常用 ──
const itemCount = (page: Page) => page.locator('[data-testid^="item-item-"]')

test('做一橫排：變成好幾個，位置左右排開', async ({ page }) => {
  await open(page)
  await shape(page, 'circle')
  await expect(page.getByTestId('size')).toBeVisible({ timeout: 30_000 })
  await page.getByRole('slider', { name: '做幾個？' }).fill('4')
  await page.getByTestId('many-row').click()
  await expect(itemCount(page)).toHaveCount(4)
  const xs: number[] = []
  for (const id of ['item-1', 'item-2', 'item-3', 'item-4']) xs.push((await rectOf(page, id))!.x)
  expect(xs[1]).toBeGreaterThan(xs[0])
  expect(xs[3]).toBeGreaterThan(xs[2])
  await expect(page.getByTestId('save')).toBeEnabled({ timeout: 30_000 })
})

test('圍成一圈，上一步可以還原', async ({ page }) => {
  await open(page)
  await shape(page, 'heart')
  await expect(page.getByTestId('size')).toBeVisible({ timeout: 30_000 })
  await page.getByRole('slider', { name: '做幾個？' }).fill('6')
  await page.getByTestId('many-circle').click()
  await expect(itemCount(page)).toHaveCount(6)
  await page.getByRole('button', { name: '上一步' }).click()
  await expect(itemCount(page)).toHaveCount(1)
})

test('左右對稱複製：多一個，在另一邊', async ({ page }) => {
  await open(page)
  await shape(page, 'heart')
  // 先把愛心拖到左邊
  const r = (await rectOf(page, 'item-1'))!
  const cx = r.x + r.width / 2
  const cy = r.y + r.height / 2
  await page.mouse.move(cx, cy)
  await page.mouse.down()
  await page.mouse.move(cx - 60, cy, { steps: 5 })
  await page.mouse.up()
  await page.getByTestId('mirror-x').click()
  await expect(itemCount(page)).toHaveCount(2)
  const a = (await rectOf(page, 'item-1'))!
  const b = (await rectOf(page, 'item-2'))!
  const stage = await box(page, 'stage')
  const mid = stage.x + stage.width / 2
  expect(a.x + a.width / 2).toBeLessThan(mid)
  expect(b.x + b.width / 2).toBeGreaterThan(mid)
})

test('邊框：方框與圓框是空心的，放得進框', async ({ page }) => {
  await open(page)
  await page.getByTestId('add-shape').click()
  await page.getByTestId('frame-rect').click()
  await expect(page.getByTestId('size')).toBeVisible({ timeout: 30_000 })
  await expect(page.getByTestId('save')).toBeEnabled()
  await expect(page.getByRole('radio', { name: '空心' })).toHaveAttribute('aria-checked', 'true')
  await page.getByTestId('add-shape').click()
  await page.getByTestId('frame-circle').click()
  await expect(itemCount(page)).toHaveCount(2)
  await expect(page.getByRole('alert')).toHaveCount(0)
})

test('字排成弧形：拖「字排成弧形」，預覽的高度變高', async ({ page }) => {
  await open(page)
  const t = await addText(page)
  await t.fill('王小明')
  const heightCm = async () => {
    const text = (await page.getByTestId('size').textContent()) ?? ''
    return Number(/× ([\d.]+) 公分/.exec(text)?.[1])
  }
  await expect(page.getByTestId('size')).toBeVisible({ timeout: 30_000 })
  const flat = await heightCm()
  await page.getByRole('slider', { name: '字排成弧形' }).fill('120')
  await expect.poll(heightCm, { timeout: 30_000 }).toBeGreaterThan(flat * 1.3)
})

test('做好的一排是一組：拖其中一個整排一起動；可以只動一個、也可以拆開', async ({ page }) => {
  await open(page)
  await shape(page, 'circle')
  await expect(page.getByTestId('size')).toBeVisible({ timeout: 30_000 })
  await page.getByRole('slider', { name: '做幾個？' }).fill('3')
  await page.getByTestId('many-row').click()
  await expect(itemCount(page)).toHaveCount(3)
  await expect(page.getByTestId('group-note')).toContainText('共 3 個')

  const drag = async (id: string, dx: number, dy: number) => {
    const r = (await rectOf(page, id))!
    const cx = r.x + r.width / 2
    const cy = r.y + r.height / 2
    await page.mouse.move(cx, cy)
    await page.mouse.down()
    await page.mouse.move(cx + dx, cy + dy, { steps: 5 })
    await page.mouse.up()
  }
  const ys = async () => [(await rectOf(page, 'item-1'))!.y, (await rectOf(page, 'item-2'))!.y, (await rectOf(page, 'item-3'))!.y]
  const y0 = await ys()
  await drag('item-2', 0, 50)
  const y1 = await ys()
  for (let i = 0; i < 3; i++) expect(y1[i] - y0[i]).toBeGreaterThan(30)

  // 只動這一個
  await page.getByTestId('group-solo').click()
  await drag('item-2', 0, 30)
  const y2 = await ys()
  expect(y2[1] - y1[1]).toBeGreaterThan(15)
  expect(Math.abs(y2[0] - y1[0])).toBeLessThan(2)

  // 拆開
  await page.getByTestId('group-break').click()
  await expect(page.getByTestId('group-note')).toHaveCount(0)
})

test('整組一起轉：按「向右轉」，每一個都多轉十五度', async ({ page }) => {
  await open(page)
  await shape(page, 'rect')
  await expect(page.getByTestId('size')).toBeVisible({ timeout: 30_000 })
  await page.getByRole('slider', { name: '做幾個？' }).fill('4')
  await page.getByTestId('many-circle').click()
  await expect(itemCount(page)).toHaveCount(4)
  await page.getByRole('button', { name: /向右轉/ }).click()
  await expect(page.locator('[data-testid="item-item-1"]')).toHaveAttribute('transform', /rotate\(15\)/)
  await expect(page.locator('[data-testid="item-item-2"]')).toHaveAttribute('transform', /rotate\(105\)/)
  // 只動這一個時，其他不跟著轉
  await page.getByTestId('group-solo').click()
  await page.getByRole('button', { name: /向右轉/ }).click()
  await expect(page.locator('[data-testid="item-item-1"]')).toHaveAttribute('transform', /rotate\(30\)/)
  await expect(page.locator('[data-testid="item-item-2"]')).toHaveAttribute('transform', /rotate\(105\)/)
})

test('兩根手指同時縮放與旋轉（真的觸控事件）', async ({ page }) => {
  await open(page)
  await shape(page, 'rect')
  await expect(page.getByTestId('size')).toBeVisible({ timeout: 30_000 })
  const r0 = (await rectOf(page, 'item-1'))!
  const cx = r0.x + r0.width / 2
  const cy = r0.y + r0.height / 2
  const cdp = await page.context().newCDPSession(page)
  const touch = (type: 'touchStart' | 'touchMove' | 'touchEnd', pts: { x: number; y: number }[]) =>
    cdp.send('Input.dispatchTouchEvent', { type, touchPoints: pts.map((p, i) => ({ x: p.x, y: p.y, id: i + 1 })) })
  // 兩指從左右各 30 像素，拉開到各 60 像素，並順時針轉 45 度
  await touch('touchStart', [{ x: cx - 30, y: cy }, { x: cx + 30, y: cy }])
  const steps = 6
  for (let i = 1; i <= steps; i++) {
    const k = i / steps
    const d = 30 + 30 * k
    const a = (Math.PI / 4) * k
    await touch('touchMove', [
      { x: cx - d * Math.cos(a), y: cy - d * Math.sin(a) },
      { x: cx + d * Math.cos(a), y: cy + d * Math.sin(a) },
    ])
  }
  await touch('touchEnd', [])
  const r1 = (await rectOf(page, 'item-1'))!
  expect(Math.max(r1.width, r1.height)).toBeGreaterThan(Math.max(r0.width, r0.height) * 1.3)
  const transform = (await page.locator('[data-testid="item-item-1"]').getAttribute('transform')) ?? ''
  const deg = Number(/rotate\(([-\d.]+)\)/.exec(transform)?.[1])
  expect(deg).toBeGreaterThan(30)
  expect(deg).toBeLessThan(60)
})

// ── 自動儲存 ──
test('做到一半關掉再打開：接著上次的設計；「全部清掉」之後不會再回來', async ({ page }) => {
  await open(page)
  const t = await addText(page)
  await t.fill('王小明')
  await shape(page, 'heart')
  await expect(page.getByTestId('size')).toBeVisible({ timeout: 30_000 })
  // 等自動儲存（約 0.6 秒）
  await page.waitForTimeout(1500)

  await page.reload()
  await expect(page.getByTestId('resumed-note')).toBeVisible({ timeout: 30_000 })
  await expect(itemCount(page)).toHaveCount(2)
  await expect(page.getByTestId('save')).toBeEnabled({ timeout: 30_000 })

  await page.getByTestId('start-over').click()
  await expect(itemCount(page)).toHaveCount(0)
  await page.waitForTimeout(1500)
  await page.reload()
  await expect(page.getByTestId('add-text')).toBeVisible({ timeout: 30_000 })
  await expect(itemCount(page)).toHaveCount(0)
  await expect(page.getByTestId('resumed-note')).toHaveCount(0)
})

test('重新開始可以用「上一步」拿回來', async ({ page }) => {
  await open(page)
  await shape(page, 'star')
  await page.getByTestId('start-over').click()
  await expect(itemCount(page)).toHaveCount(0)
  await page.getByRole('button', { name: '上一步' }).click()
  await expect(itemCount(page)).toHaveCount(1)
})

test('圖片設計也能接著做', async ({ page }) => {
  await open(page)
  await page.getByTestId('file-input').setInputFiles('tests/e2e/fixtures/logo.png')
  await expect(page.getByTestId('save')).toBeEnabled({ timeout: 60_000 })
  await page.waitForTimeout(1500)
  await page.reload()
  await expect(page.getByTestId('resumed-note')).toBeVisible({ timeout: 30_000 })
  await expect(page.getByTestId('save')).toBeEnabled({ timeout: 60_000 })
})

// ── 電腦版 ──
test.describe('電腦版（1440×900）', () => {
  test.use({ viewport: { width: 1440, height: 900 }, hasTouch: false, isMobile: false })

  test('整個畫面不捲動：工具列、畫布、工具區都在畫面裡，畫布夠大', async ({ page }) => {
    await open(page)
    await addText(page)
    await shape(page, 'heart')
    await expect(page.getByTestId('size')).toBeVisible({ timeout: 30_000 })
    const scrolls = await page.evaluate(() => document.documentElement.scrollHeight - window.innerHeight)
    expect(scrolls, '整頁不該需要捲動').toBeLessThanOrEqual(1)
    // 加東西與存檔都在最上面一條
    const save = (await page.getByTestId('save').boundingBox())!
    const add = (await page.getByTestId('add-text').boundingBox())!
    expect(save.y).toBeLessThan(80)
    expect(add.y).toBeLessThan(80)
    // 畫布高度至少佔畫面一半以上
    const stage = (await page.getByTestId('stage').boundingBox())!
    expect(stage.height).toBeGreaterThan(450)
    // 工具區自己會捲：最底下的「放正中間」之類的按鈕要捲得到、點得到
    await page.getByTestId('duplicate').scrollIntoViewIfNeeded()
    await expect(page.getByTestId('duplicate')).toBeVisible()
  })

  test('鍵盤快捷鍵：Ctrl+Z 上一步、Ctrl+Shift+Z 下一步、Ctrl+D 複製', async ({ page }) => {
    await open(page)
    await shape(page, 'star')
    await expect(itemCount(page)).toHaveCount(1)
    await page.keyboard.press('Control+d')
    await expect(itemCount(page)).toHaveCount(2)
    await page.keyboard.press('Control+z')
    await expect(itemCount(page)).toHaveCount(1)
    await page.keyboard.press('Control+Shift+z')
    await expect(itemCount(page)).toHaveCount(2)
    await page.keyboard.press('Control+y') // 沒有可以重做的了，不會出事
    await expect(itemCount(page)).toHaveCount(2)
  })

  test('選到的東西按 Delete 刪掉（可以拿回來）', async ({ page }) => {
    await open(page)
    await shape(page, 'star')
    await page.keyboard.press('Delete')
    await expect(itemCount(page)).toHaveCount(0)
    await expect(page.getByTestId('removed-note')).toBeVisible()
  })

  test('在輸入框裡按 Ctrl+Z 是復原打的字，不會把整個設計復原', async ({ page }) => {
    await open(page)
    const t = await addText(page)
    await t.focus()
    await page.keyboard.press('Control+z')
    await expect(itemCount(page)).toHaveCount(1)
  })

  test('滑鼠移到物件上游標變成移動、移到把手上變成縮放', async ({ page }) => {
    await open(page)
    await shape(page, 'rect')
    await expect(page.getByTestId('size')).toBeVisible({ timeout: 30_000 })
    const r = (await rectOf(page, 'item-1'))!
    await page.mouse.move(r.x + r.width / 2, r.y + r.height / 2)
    await expect(page.getByTestId('stage')).toHaveCSS('cursor', 'move')
    const h = (await page.getByTestId('handle-resize').nth(2).boundingBox())!
    await page.mouse.move(h.x + h.width / 2, h.y + h.height / 2)
    await expect(page.getByTestId('stage')).toHaveCSS('cursor', 'nwse-resize')
    const stage = (await box(page, 'stage'))
    await page.mouse.move(stage.x + 5, stage.y + 5)
    await expect(page.getByTestId('stage')).toHaveCSS('cursor', 'default')
  })
})

// ── 畫布放大與平移 ──
test.describe('畫布放大與平移', () => {
  const stageBox = async (page: Page) => (await page.getByTestId('stage').boundingBox())!

  test('按「放大」物件變大、「看全部」回到原樣', async ({ page }) => {
    await open(page)
    await shape(page, 'rect')
    await expect(page.getByTestId('size')).toBeVisible({ timeout: 30_000 })
    const r0 = (await rectOf(page, 'item-1'))!
    await expect(page.getByTestId('zoom-fit')).toHaveCount(0)
    await page.getByTestId('zoom-in').click()
    await page.getByTestId('zoom-in').click()
    const r1 = (await rectOf(page, 'item-1'))!
    expect(r1.width).toBeGreaterThan(r0.width * 1.8)
    await page.getByTestId('zoom-fit').click()
    const r2 = (await rectOf(page, 'item-1'))!
    expect(Math.abs(r2.width - r0.width)).toBeLessThan(1)
    await expect(page.getByTestId('zoom-fit')).toHaveCount(0)
  })

  test('滾輪以游標為中心放大；放大後拖空白處可以平移，點一下空白才取消選取', async ({ page }) => {
    await open(page)
    await shape(page, 'rect')
    await expect(page.getByTestId('size')).toBeVisible({ timeout: 30_000 })
    const s = await stageBox(page)
    const r0 = (await rectOf(page, 'item-1'))!
    const cx = r0.x + r0.width / 2
    const cy = r0.y + r0.height / 2
    await page.mouse.move(cx, cy)
    await page.mouse.wheel(0, -400)
    const r1 = (await rectOf(page, 'item-1'))!
    expect(r1.width).toBeGreaterThan(r0.width * 1.3)
    // 游標在物件中心，放大後物件中心仍在游標底下
    expect(Math.abs(r1.x + r1.width / 2 - cx)).toBeLessThan(3)

    // 拖空白處平移
    const empty = { x: s.x + 12, y: s.y + s.height - 12 }
    await page.mouse.move(empty.x, empty.y)
    await page.mouse.down()
    await page.mouse.move(empty.x + 40, empty.y - 30, { steps: 5 })
    await page.mouse.up()
    const r2 = (await rectOf(page, 'item-1'))!
    expect(r2.x - r1.x).toBeGreaterThan(25)
    expect(r1.y - r2.y).toBeGreaterThan(15)
    await expect(page.getByTestId('item-panel')).toBeVisible() // 平移不會取消選取

    // 只是點一下空白：取消選取
    await page.mouse.click(empty.x, empty.y)
    await expect(page.getByTestId('item-panel')).toHaveCount(0)
  })

  test('放大後拖移物件仍然正確', async ({ page }) => {
    await open(page)
    await shape(page, 'rect')
    await expect(page.getByTestId('size')).toBeVisible({ timeout: 30_000 })
    await page.getByTestId('zoom-in').click()
    await page.getByTestId('zoom-in').click()
    const r0 = (await rectOf(page, 'item-1'))!
    const cx = r0.x + r0.width / 2
    const cy = r0.y + r0.height / 2
    await page.mouse.move(cx, cy)
    await page.mouse.down()
    await page.mouse.move(cx + 30, cy, { steps: 5 })
    await page.mouse.up()
    const r1 = (await rectOf(page, 'item-1'))!
    expect(Math.abs(r1.x - r0.x - 30)).toBeLessThan(2)
  })

  test('兩根手指落在空白處：整個畫布放大；落在物件上才是縮放物件', async ({ page }) => {
    await open(page)
    await shape(page, 'rect')
    await expect(page.getByTestId('size')).toBeVisible({ timeout: 30_000 })
    const s = await stageBox(page)
    const r0 = (await rectOf(page, 'item-1'))!
    const cdp = await page.context().newCDPSession(page)
    const touch = (type: 'touchStart' | 'touchMove' | 'touchEnd', pts: { x: number; y: number }[]) =>
      cdp.send('Input.dispatchTouchEvent', { type, touchPoints: pts.map((p, i) => ({ x: p.x, y: p.y, id: i + 1 })) })
    // 兩指在畫布左下角的空白處撐開
    const mx = s.x + s.width * 0.2
    const my = s.y + s.height * 0.85
    await touch('touchStart', [{ x: mx - 20, y: my }, { x: mx + 20, y: my }])
    for (let i = 1; i <= 5; i++) {
      const d = 20 + 12 * i
      await touch('touchMove', [{ x: mx - d, y: my }, { x: mx + d, y: my }])
    }
    await touch('touchEnd', [])
    const r1 = (await rectOf(page, 'item-1'))!
    expect(r1.width).toBeGreaterThan(r0.width * 1.5)
    // 物件自己的縮放沒有被改
    await page.getByTestId('zoom-fit').click()
    const r2 = (await rectOf(page, 'item-1'))!
    expect(Math.abs(r2.width - r0.width)).toBeLessThan(2)
  })

  test('鍵盤 0 回到看全部；換框也會回到看全部', async ({ page }) => {
    await open(page)
    await shape(page, 'rect')
    await page.getByTestId('zoom-in').click()
    await expect(page.getByTestId('zoom-fit')).toBeVisible()
    await page.getByTestId('stage').focus()
    await page.keyboard.press('0')
    await expect(page.getByTestId('zoom-fit')).toHaveCount(0)
    await page.getByTestId('zoom-in').click()
    await page.getByRole('radio', { name: /小框/ }).click()
    await expect(page.getByTestId('zoom-fit')).toHaveCount(0)
  })
})

test('拖動時畫面上方顯示數字：位置、大小、角度，放開就消失', async ({ page }) => {
  await open(page)
  await shape(page, 'rect')
  await expect(page.getByTestId('size')).toBeVisible({ timeout: 30_000 })
  const r = (await rectOf(page, 'item-1'))!
  const cx = r.x + r.width / 2
  const cy = r.y + r.height / 2
  await expect(page.getByTestId('readout')).toHaveCount(0)
  await page.mouse.move(cx, cy)
  await page.mouse.down()
  await page.mouse.move(cx + 40, cy + 10, { steps: 4 })
  await expect(page.getByTestId('readout')).toContainText('公分')
  await page.mouse.up()
  await expect(page.getByTestId('readout')).toHaveCount(0)

  const h = (await page.getByTestId('handle-resize').nth(2).boundingBox())!
  await page.mouse.move(h.x + h.width / 2, h.y + h.height / 2)
  await page.mouse.down()
  await page.mouse.move(h.x + h.width / 2 + 30, h.y + h.height / 2 + 30, { steps: 4 })
  await expect(page.getByTestId('readout')).toContainText('寬')
  await page.mouse.up()

  const rot = (await page.getByTestId('handle-rotate').boundingBox())!
  const r2 = (await rectOf(page, 'item-1'))!
  await page.mouse.move(rot.x + rot.width / 2, rot.y + rot.height / 2)
  await page.mouse.down()
  await page.mouse.move(r2.x + r2.width / 2 + 120, r2.y + r2.height / 2, { steps: 6 })
  await expect(page.getByTestId('readout')).toContainText('向右轉')
  await page.mouse.up()
})

test('拖到快對齊別的東西時：出現參考線並吸過去', async ({ page }) => {
  await open(page)
  await shape(page, 'rect')
  await page.getByTestId('duplicate').click() // item-2 在 item-1 右下方
  await expect(itemCount(page)).toHaveCount(2)
  const a = (await rectOf(page, 'item-1'))!
  const b = (await rectOf(page, 'item-2'))!
  const bx = b.x + b.width / 2
  const by = b.y + b.height / 2
  const targetX = a.x + a.width / 2 + 3 // 離 item-1 的中心還差 3 像素
  await page.mouse.move(bx, by)
  await page.mouse.down()
  await page.mouse.move(targetX, by + 30, { steps: 8 })
  await expect(page.getByTestId('guide')).not.toHaveCount(0) // 線沒有寬度，不能用「看得到」判斷
  await page.mouse.up()
  await expect(page.getByTestId('guide')).toHaveCount(0)
  const b2 = (await rectOf(page, 'item-2'))!
  const a2 = (await rectOf(page, 'item-1'))!
  expect(Math.abs(b2.x + b2.width / 2 - (a2.x + a2.width / 2))).toBeLessThan(1.5)
})

test('吸附不會讓一般拖移卡住：離得遠就照手指走', async ({ page }) => {
  await open(page)
  await shape(page, 'circle')
  const r = (await rectOf(page, 'item-1'))!
  const cx = r.x + r.width / 2
  const cy = r.y + r.height / 2
  await page.mouse.move(cx, cy)
  await page.mouse.down()
  await page.mouse.move(cx + 37, cy + 53, { steps: 6 })
  await page.mouse.up()
  const r2 = (await rectOf(page, 'item-1'))!
  expect(Math.abs(r2.x - r.x - 37)).toBeLessThan(10)
})

test('鎖住：拖不動、沒有把手、可以改顏色、解開後又能動', async ({ page }) => {
  await open(page)
  await shape(page, 'rect')
  await expect(page.getByTestId('size')).toBeVisible({ timeout: 30_000 })
  await page.getByTestId('lock').click()
  await expect(page.getByTestId('lock-note')).toBeVisible()
  await expect(page.getByTestId('handle-resize')).toHaveCount(0)
  await expect(page.getByTestId('handle-rotate')).toHaveCount(0)

  const r0 = (await rectOf(page, 'item-1'))!
  const cx = r0.x + r0.width / 2
  const cy = r0.y + r0.height / 2
  await page.mouse.move(cx, cy)
  await page.mouse.down()
  await page.mouse.move(cx + 60, cy + 40, { steps: 5 })
  await expect(page.getByTestId('readout')).toContainText('鎖住')
  await page.mouse.up()
  const r1 = (await rectOf(page, 'item-1'))!
  expect(Math.abs(r1.x - r0.x)).toBeLessThan(1)

  // 鎖住的仍然能改顏色
  await page.getByRole('radio', { name: '寶藍' }).click()
  await expect(page.getByRole('radio', { name: '寶藍' })).toHaveAttribute('aria-checked', 'true')

  // 解開
  await page.getByTestId('lock').click()
  await expect(page.getByTestId('handle-resize')).toHaveCount(4)
  await page.mouse.move(cx, cy)
  await page.mouse.down()
  await page.mouse.move(cx + 60, cy + 40, { steps: 5 })
  await page.mouse.up()
  const r2 = (await rectOf(page, 'item-1'))!
  expect(r2.x - r0.x).toBeGreaterThan(40)
})
