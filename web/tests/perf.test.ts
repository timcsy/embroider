import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { beforeAll, describe, expect, it } from 'vitest'
import {
  buildImagePreview,
  buildTextPreview,
  initCoreSync,
  listBuiltinHoops,
  registerFont,
} from '../src/lib/core'
import type { TextRequest } from '../src/lib/types'

// 規格 SC-002：20 字內的文字預覽 1 秒內（中階 Android 手機）。
// SC-007：簡單圖案轉換 10 秒內。開發機比手機快很多（實測文字 20～100ms、圖片約 30ms），
// 這裡的預算留給手機約 3～5 倍的慢速，仍遠低於規格；退化到接近預算就會失敗。
const TEXT_BUDGET_MS = 300
const IMAGE_BUDGET_MS = 1500

const hoop = () => listBuiltinHoops().find((h) => h.id === 'b70-large')!
const color = { r: 200, g: 30, b: 40, name: '正紅' }

function text(textValue: string, font = 'sans', height = 200): TextRequest {
  return { text: textValue, font_id: font, height, color, layout: { kind: 'single_line' }, hoop: hoop() }
}

// 並行跑整套測試時，機器忙碌會讓單次量測偶爾超時；取三次中最快的一次，
// 真正的退化（每次都慢）仍然抓得到，偶發的忙碌不會誤報。
function time<T>(f: () => T): [T, number] {
  let best = Infinity
  let result!: T
  for (let i = 0; i < 3; i++) {
    const t0 = performance.now()
    result = f()
    best = Math.min(best, performance.now() - t0)
  }
  return [result, best]
}

// 其他測試檔同時在跑時，量到的時間會被拖慢（已出現過多次誤報）：失敗時重試，真的變慢三次都會失敗
describe('效能預算', { retry: 2 }, () => {
  beforeAll(() => {
    initCoreSync(readFileSync(resolve(process.cwd(), 'src/lib/wasm/stitch_wasm_bg.wasm')))
    for (const [id, file] of [
      ['sans', 'NotoSansTC-subset.ttf'],
      ['kai', 'LXGWWenKaiTC-subset.ttf'],
    ]) {
      registerFont(id, id, new Uint8Array(readFileSync(resolve(process.cwd(), '..', 'fonts', file))))
    }
  })

  it.each([
    ['短名字', '王小明'],
    ['八個中文字', '王小明的生日快樂'],
    ['二十個英數字', 'Happy Birthday Amy 2026'],
    ['會自動分行的長句', '祝你生日快樂身體健康萬事如意心想事成'],
  ])('文字預覽：%s 在預算內', (_name, value) => {
    buildTextPreview(text(value)) // 暖機
    const [r, ms] = time(() => buildTextPreview(text(value)))
    expect(r.blocked, JSON.stringify(r.blocked)).toBeNull()
    expect(ms).toBeLessThan(TEXT_BUDGET_MS)
  })

  it('筆畫繁多的楷體文字在預算內', () => {
    const [r, ms] = time(() => buildTextPreview(text('龍鳳呈祥龍鳳呈祥', 'kai', 250)))
    expect(r.blocked).toBeNull()
    expect(ms).toBeLessThan(TEXT_BUDGET_MS)
  })

  it('圖片轉換（512×512 的簡單圖案）在預算內', () => {
    const size = 512
    const rgba = new Uint8Array(size * size * 4)
    for (let y = 0; y < size; y++) {
      for (let x = 0; x < size; x++) {
        const c =
          Math.hypot(x - 256, y - 256) < 150
            ? [200, 30, 40]
            : x > 40 && x < 200 && y > 380 && y < 470
              ? [40, 80, 180]
              : [255, 255, 255]
        rgba.set([c[0], c[1], c[2], 255], (y * size + x) * 4)
      }
    }
    const req = { width: size, height: size, color_count: 4, simplify: 30, size: 1200, hoop: hoop() }
    const [r, ms] = time(() => buildImagePreview(req, rgba))
    expect(r.blocked).toBeNull()
    expect(ms).toBeLessThan(IMAGE_BUDGET_MS)
  })

  it('圖片轉換（顏色多、細碎的插畫，大框）在預算內', () => {
    const rgba = new Uint8Array(readFileSync(resolve(__dirname, '../../crates/stitch-image/tests/fixtures/thanks-256.rgba')))
    const req = { width: 256, height: 171, color_count: 8, simplify: 30, size: 1400, hoop: hoop() }
    const [r, ms] = time(() => buildImagePreview(req, rgba))
    console.log('插畫轉換耗時 ms', Math.round(ms), JSON.stringify(r.blocked))
    expect(r.blocked).toBeNull()
    expect(ms).toBeLessThan(IMAGE_BUDGET_MS)
  })

  it('圖片轉換（512 像素的細碎插畫、顏色多）在預算內', () => {
    // 把 256 的樣本放大一倍，等於使用者實際上傳的最大尺寸
    const small = readFileSync(resolve(__dirname, '../../crates/stitch-image/tests/fixtures/thanks-256.rgba'))
    const [sw, sh] = [256, 171]
    const rgba = new Uint8Array(sw * 2 * sh * 2 * 4)
    for (let y = 0; y < sh * 2; y++)
      for (let x = 0; x < sw * 2; x++)
        for (let c = 0; c < 4; c++) rgba[(y * sw * 2 + x) * 4 + c] = small[((y >> 1) * sw + (x >> 1)) * 4 + c]
    const req = { width: sw * 2, height: sh * 2, color_count: 9, simplify: 30, size: 1400, hoop: hoop() }
    const [r, ms] = time(() => buildImagePreview(req, rgba))
    console.log('512 插畫轉換耗時 ms', Math.round(ms), JSON.stringify(r.blocked))
    expect(r.blocked).toBeNull()
    expect(ms).toBeLessThan(IMAGE_BUDGET_MS * 2)
  })
})
