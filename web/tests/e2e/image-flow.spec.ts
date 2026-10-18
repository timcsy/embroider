import { expect, test } from '@playwright/test'
import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'

const fixture = (name: string) => resolve(process.cwd(), 'tests/e2e/fixtures', name)

async function openImageTab(page: import('@playwright/test').Page) {
  await page.goto('/')
  await expect(page.getByTestId('tab-image')).toBeVisible({ timeout: 30_000 })
  await page.getByTestId('tab-image').click()
}

test('選一張三色圖：預覽出現三種顏色，可以調整，可以存出', async ({ page }) => {
  const downloads: { name: string; path: string }[] = []
  page.on('download', async (d) => downloads.push({ name: d.suggestedFilename(), path: (await d.path()) ?? '' }))

  await openImageTab(page)
  await expect(page.getByText('還沒有選圖片')).toBeVisible()
  await expect(page.getByTestId('save')).toBeDisabled()

  await page.getByTestId('file-input').setInputFiles(fixture('logo.png'))
  await expect(page.getByTestId('size')).toBeVisible({ timeout: 60_000 })
  await expect(page.getByTestId('save')).toBeEnabled({ timeout: 60_000 })

  // 預覽裡確實出現紅、藍、綠三種線色
  const seen = await page.getByTestId('preview').evaluate((c: HTMLCanvasElement) => {
    const d = c.getContext('2d')!.getImageData(0, 0, c.width, c.height).data
    let red = 0, blue = 0, green = 0
    for (let i = 0; i < d.length; i += 4) {
      const [r, g, b] = [d[i], d[i + 1], d[i + 2]]
      if (r > 150 && g < 90 && b < 90) red++
      if (b > 130 && r < 90 && g < 120) blue++
      if (g > 120 && r < 120 && b < 100) green++
    }
    return { red, blue, green }
  })
  expect(seen.red).toBeGreaterThan(100)
  expect(seen.blue).toBeGreaterThan(100)
  expect(seen.green).toBeGreaterThan(100)

  // 調整「簡化程度」後預覽會更新、不會壞掉
  await page.getByRole('slider', { name: '簡化程度' }).fill('80')
  await expect(page.getByTestId('size')).toBeVisible({ timeout: 60_000 })
  await expect(page.getByTestId('save')).toBeEnabled({ timeout: 60_000 })

  await page.getByTestId('save').click()
  await expect.poll(() => downloads.length, { timeout: 30_000 }).toBe(2)
  // 檔名取自圖片檔名（logo → LOGO0001）
  expect(downloads.map((d) => d.name).sort()).toEqual(['LOGO0001.DST', 'LOGO0001.EXP'])
  for (const d of downloads) expect(readFileSync(d.path).length).toBeGreaterThan(512)
})

test('只有一種顏色的圖：說明沒有東西可繡，不能存', async ({ page }) => {
  await openImageTab(page)
  await page.getByTestId('file-input').setInputFiles(fixture('flat.png'))
  await expect(page.getByRole('alert')).toContainText('只有一種顏色', { timeout: 60_000 })
  await expect(page.getByTestId('save')).toBeDisabled()
})

test('選到不是圖片的檔案：白話提示，畫面不壞', async ({ page }) => {
  await openImageTab(page)
  await page.getByTestId('file-input').setInputFiles({
    name: 'note.txt',
    mimeType: 'text/plain',
    buffer: readFileSync(fixture('not-image.txt')),
  })
  await expect(page.getByRole('alert')).toContainText('不是圖片')
  await expect(page.getByTestId('save')).toBeDisabled()
  // 之後還能正常選圖
  await page.getByTestId('file-input').setInputFiles(fixture('logo.png'))
  await expect(page.getByTestId('save')).toBeEnabled({ timeout: 60_000 })
})

test('切換回打字再切回圖片，選好的圖還在', async ({ page }) => {
  await openImageTab(page)
  await page.getByTestId('file-input').setInputFiles(fixture('logo.png'))
  await expect(page.getByTestId('save')).toBeEnabled({ timeout: 60_000 })
  await page.getByTestId('tab-text').click()
  await expect(page.getByTestId('save')).toBeDisabled()
  await page.getByTestId('tab-image').click()
  await expect(page.getByTestId('save')).toBeEnabled({ timeout: 60_000 })
})

test('拖動「顏色多寡」：顏色少時合併成較少的線色', async ({ page }) => {
  await openImageTab(page)
  await page.getByTestId('file-input').setInputFiles(fixture('logo.png'))
  await expect(page.getByTestId('save')).toBeEnabled({ timeout: 60_000 })
  const colorCount = () =>
    page.getByTestId('preview').evaluate((c: HTMLCanvasElement) => {
      const d = c.getContext('2d')!.getImageData(0, 0, c.width, c.height).data
      const seen = { red: 0, blue: 0, green: 0 }
      for (let i = 0; i < d.length; i += 4) {
        if (d[i] > 150 && d[i + 1] < 90 && d[i + 2] < 90) seen.red++
        if (d[i + 2] > 130 && d[i] < 90 && d[i + 1] < 120) seen.blue++
        if (d[i + 1] > 120 && d[i] < 120 && d[i + 2] < 100) seen.green++
      }
      return Object.values(seen).filter((n) => n > 100).length
    })
  expect(await colorCount()).toBe(3)
  // 只留兩種顏色：紅方塊、藍圓、綠條至少有一個會被併進別的顏色或當成背景
  await page.getByRole('slider', { name: '顏色多寡' }).fill('2')
  await expect.poll(colorCount, { timeout: 60_000 }).toBeLessThan(3)
})

test('在圖上圈出一部分：只繡框內的，可以還原成整張圖', async ({ page }) => {
  await openImageTab(page)
  await page.getByTestId('file-input').setInputFiles(fixture('logo.png'))
  await expect(page.getByTestId('size')).toBeVisible({ timeout: 60_000 })

  const colorsSeen = () =>
    page.getByTestId('preview').evaluate((c: HTMLCanvasElement) => {
      const d = c.getContext('2d')!.getImageData(0, 0, c.width, c.height).data
      let red = 0, blue = 0, green = 0
      for (let i = 0; i < d.length; i += 4) {
        const [r, g, b] = [d[i], d[i + 1], d[i + 2]]
        if (r > 150 && g < 90 && b < 90) red++
        if (b > 130 && r < 90 && g < 120) blue++
        if (g > 120 && r < 120 && b < 100) green++
      }
      return { red, blue, green }
    })
  const before = await colorsSeen()
  expect(before.blue).toBeGreaterThan(100)
  expect(before.green).toBeGreaterThan(100)

  // 圖的左上角是紅色方塊：只圈左上四分之一強
  const area = page.getByTestId('crop-area')
  const b = (await area.boundingBox())!
  await page.mouse.move(b.x + b.width * 0.02, b.y + b.height * 0.02)
  await page.mouse.down()
  await page.mouse.move(b.x + b.width * 0.5, b.y + b.height * 0.5, { steps: 5 })
  await page.mouse.up()
  await expect(page.getByTestId('crop-frame')).toBeVisible()
  await expect(page.getByTestId('crop-reset')).toBeVisible()

  await expect.poll(async () => (await colorsSeen()).blue, { timeout: 30_000 }).toBeLessThan(20)
  const cropped = await colorsSeen()
  expect(cropped.red).toBeGreaterThan(100)
  expect(cropped.green).toBeLessThan(20)

  // 還原成整張圖
  await page.getByTestId('crop-reset').click()
  await expect.poll(async () => (await colorsSeen()).blue, { timeout: 30_000 }).toBeGreaterThan(100)
  await expect(page.getByTestId('crop-frame')).toHaveCount(0)
})
