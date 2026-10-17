import { expect, test } from '@playwright/test'
import { readFileSync } from 'node:fs'

test('輸入文字 → 預覽 → 選框 → 存出檔案', async ({ page }) => {
  const downloads: { name: string; path: string }[] = []
  page.on('download', async (d) => {
    downloads.push({ name: d.suggestedFilename(), path: (await d.path()) ?? '' })
  })

  await page.goto('/')
  const box = page.getByRole('textbox')
  await expect(box).toBeVisible({ timeout: 30_000 })

  // 一開始沒有文字：不能存
  await expect(page.getByTestId('save')).toBeDisabled()

  await box.fill('王小明')
  await expect(page.getByTestId('size')).toBeVisible({ timeout: 30_000 })
  await expect(page.getByTestId('size')).toContainText('公分')
  await expect(page.getByTestId('save')).toBeEnabled()

  // 預覽畫布有實際畫出東西（不是一片空白）
  const painted = await page.getByTestId('preview').evaluate((c: HTMLCanvasElement) => {
    const ctx = c.getContext('2d')!
    const data = ctx.getImageData(0, 0, c.width, c.height).data
    let red = 0
    for (let i = 0; i < data.length; i += 4) {
      if (data[i] > 150 && data[i + 1] < 90 && data[i + 2] < 90) red++
    }
    return red
  })
  expect(painted).toBeGreaterThan(200)

  await page.getByRole('radio', { name: /中框/ }).click()
  await page.getByTestId('save').click()
  await expect(page.getByTestId('export-sheet')).toBeVisible({ timeout: 30_000 })
  await expect.poll(() => downloads.length, { timeout: 15_000 }).toBe(2)

  expect(downloads.map((d) => d.name).sort()).toEqual(['EMB0001.DST', 'EMB0001.EXP'])
  for (const d of downloads) {
    expect(readFileSync(d.path).length).toBeGreaterThan(512)
  }
  // 提示文字是白話，並列出檔名對照
  await expect(page.getByTestId('export-sheet')).toContainText('USB')
  await expect(page.getByTestId('export-sheet')).toContainText('「下載」資料夾')
  await expect(page.getByTestId('export-sheet')).toContainText('EMB0001.DST')
})

test('空白文字不能存，也沒有嚇人的錯誤', async ({ page }) => {
  await page.goto('/')
  const box = page.getByRole('textbox')
  await expect(box).toBeVisible({ timeout: 30_000 })
  await box.fill('   ')
  await expect(page.getByTestId('save')).toBeDisabled()
  await expect(page.getByRole('alert')).toHaveCount(0)
})

test('復原與重做', async ({ page }) => {
  await page.goto('/')
  const box = page.getByRole('textbox')
  await expect(box).toBeVisible({ timeout: 30_000 })
  await box.fill('Amy')
  await page.getByRole('radio', { name: '宋體' }).click()
  await expect(page.getByRole('radio', { name: '宋體' })).toHaveAttribute('aria-checked', 'true')
  await page.getByRole('button', { name: '上一步' }).click()
  await expect(page.getByRole('radio', { name: '黑體' })).toHaveAttribute('aria-checked', 'true')
  await expect(box).toHaveValue('Amy')
  await page.getByRole('button', { name: '下一步' }).click()
  await expect(page.getByRole('radio', { name: '宋體' })).toHaveAttribute('aria-checked', 'true')
})

test('拖動「字的大小」，實際大小跟著變', async ({ page }) => {
  await page.goto('/')
  const box = page.getByRole('textbox', { name: '想繡什麼字？' })
  await expect(box).toBeVisible({ timeout: 30_000 })
  await box.fill('王小明')
  const widthCm = async () => {
    const t = (await page.getByTestId('size').textContent()) ?? ''
    return Number(/約 ([\d.]+) ×/.exec(t)?.[1])
  }
  await expect(page.getByTestId('size')).toBeVisible({ timeout: 30_000 })
  await page.getByRole('slider', { name: '字的大小' }).fill('2')
  await expect.poll(widthCm, { timeout: 30_000 }).toBeGreaterThan(4)
  const small = await widthCm()
  await page.getByRole('slider', { name: '字的大小' }).fill('4')
  await expect.poll(widthCm, { timeout: 30_000 }).toBeGreaterThan(small * 1.5)
})

test('換線色：預覽的顏色跟著變', async ({ page }) => {
  await page.goto('/')
  const box = page.getByRole('textbox', { name: '想繡什麼字？' })
  await expect(box).toBeVisible({ timeout: 30_000 })
  await box.fill('王小明')
  await expect(page.getByTestId('size')).toBeVisible({ timeout: 30_000 })
  const dominant = () =>
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
  expect((await dominant()).red).toBeGreaterThan(100)
  await page.getByRole('radio', { name: '寶藍' }).click()
  await expect.poll(async () => (await dominant()).blue, { timeout: 30_000 }).toBeGreaterThan(100)
  expect((await dominant()).red).toBeLessThan(50)
})

test('整個過程沒有任何對外連線（圖片與設計不離開手機）', async ({ page }) => {
  const external: string[] = []
  page.on('request', (r) => {
    const u = new URL(r.url())
    if (!['localhost', '127.0.0.1'].includes(u.hostname) && u.protocol.startsWith('http')) {
      external.push(r.url())
    }
  })
  await page.goto('/')
  const box = page.getByRole('textbox', { name: '想繡什麼字？' })
  await expect(box).toBeVisible({ timeout: 30_000 })
  await box.fill('王小明')
  await expect(page.getByTestId('save')).toBeEnabled({ timeout: 30_000 })
  await page.getByTestId('save').click()
  await expect(page.getByTestId('export-sheet')).toBeVisible({ timeout: 30_000 })
  await page.getByRole('button', { name: '好了' }).click()
  await page.getByTestId('tab-image').click()
  await page.getByTestId('file-input').setInputFiles('tests/e2e/fixtures/logo.png')
  await expect(page.getByTestId('save')).toBeEnabled({ timeout: 60_000 })
  expect(external).toEqual([])
})
