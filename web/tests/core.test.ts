import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { beforeAll, describe, expect, it } from 'vitest'
import { initCoreSync, listBuiltinHoops, listFormats, validateHoop } from '../src/lib/core'

beforeAll(() => {
  const bytes = readFileSync(resolve(process.cwd(), 'src/lib/wasm/stitch_wasm_bg.wasm'))
  initCoreSync(bytes)
})

describe('核心載入層', () => {
  it('列出內建三個框', () => {
    const hoops = listBuiltinHoops()
    expect(hoops.map((h) => h.id)).toEqual(['b70-large', 'b70-medium', 'b70-small'])
    expect(hoops[2]).toMatchObject({ width: 500, height: 750, builtin: true })
  })

  it('列出輸出格式，名稱是給人看的', () => {
    const formats = listFormats()
    expect(formats.map((f) => f.id)).toEqual(['dst', 'exp'])
    expect(formats[0].label).not.toMatch(/DST/)
  })

  it('驗證框：合法與不合法', () => {
    const ok = validateHoop({
      id: 'x', name: '我的框', width: 1000, height: 1000, shape: 'rect', margin: 50, builtin: false,
    })
    expect(ok.ok).toBe(true)
    const bad = validateHoop({
      id: 'x', name: '', width: 1000, height: 1000, shape: 'rect', margin: 50, builtin: false,
    })
    expect(bad.ok).toBe(false)
    expect(bad.message?.text).toBeTruthy()
  })
})

import {
  buildTextPreview,
  encodeAll,
  listFonts,
  registerFont,
} from '../src/lib/core'
import type { TextRequest } from '../src/lib/types'

function loadFont(id: string, name: string, file: string) {
  const bytes = readFileSync(resolve(process.cwd(), '..', 'fonts', file))
  return registerFont(id, name, new Uint8Array(bytes))
}

function request(text: string, hoopId = 'b70-medium', height = 150): TextRequest {
  const hoop = listBuiltinHoops().find((h) => h.id === hoopId)!
  return {
    text,
    font_id: 'sans',
    height,
    color: { r: 200, g: 30, b: 30, name: '正紅' },
    layout: { kind: 'single_line' },
    hoop,
  }
}

describe('文字預覽與輸出', () => {
  beforeAll(() => {
    expect(loadFont('sans', '黑體', 'NotoSansTC-subset.ttf').ok).toBe(true)
  })

  it('註冊壞字型回報白話錯誤', () => {
    const r = registerFont('bad', '壞的', new Uint8Array([1, 2, 3]))
    expect(r.ok).toBe(false)
    expect(r.message?.text).toBeTruthy()
  })

  it('列出已註冊的字型', () => {
    expect(listFonts().map((f) => f.id)).toContain('sans')
  })

  it('輸入文字得到合法的針跡計畫', () => {
    const r = buildTextPreview(request('Amy 2026'))
    expect(r.blocked).toBeNull()
    expect(r.plan.stitches.length).toBeGreaterThan(1000)
    expect(r.plan.stitches.at(-1)?.kind).toBe('End')
    expect(r.plan.colors).toHaveLength(1)
  })

  it('空白文字被阻止並帶白話訊息', () => {
    const r = buildTextPreview(request('   '))
    expect(r.blocked?.code).toBe('empty_text')
  })

  it('輸出全部格式，檔名安全，位元組非空', () => {
    const q = request('Amy')
    const r = buildTextPreview(q)
    const out = encodeAll(r.plan, q.hoop, [], 'Amy', 1)
    expect(out.error).toBeNull()
    expect(out.files!.map((f) => f.fileName)).toEqual(['AMY0001.DST', 'AMY0001.EXP'])
    for (const f of out.files!) {
      expect(f.bytes).toBeInstanceOf(Uint8Array)
      expect(f.bytes.length).toBeGreaterThan(100)
    }
  })

  it('指定格式時只輸出該格式', () => {
    const q = request('Amy')
    const out = encodeAll(buildTextPreview(q).plan, q.hoop, ['dst'], 'Amy', 2)
    expect(out.files!.map((f) => f.fileName)).toEqual(['AMY0002.DST'])
  })

  it('超出框的計畫無法輸出', () => {
    const q = request('Amy')
    const plan = buildTextPreview(q).plan
    plan.stitches.unshift({ x: 9000, y: 0, kind: 'Stitch' })
    const out = encodeAll(plan, q.hoop, [], 'Amy', 1)
    expect(out.files).toBeNull()
    expect(out.error?.code).toBe('out_of_hoop')
  })

  it('一般長度文字的預覽在可接受時間內完成', () => {
    const t0 = performance.now()
    buildTextPreview(request('王小明的生日快樂'))
    const ms = performance.now() - t0
    // 規格 SC-002 要求 1 秒內（中階手機）；開發機上應遠低於此
    expect(ms).toBeLessThan(1000)
  })
})

import { buildImagePreview, listThreadColors, suggestColorCount } from '../src/lib/core'
import { THREADS } from '../src/lib/threads'

function logoRgba(size: number): Uint8Array {
  const out = new Uint8Array(size * size * 4)
  for (let y = 0; y < size; y++) {
    for (let x = 0; x < size; x++) {
      const c =
        x < size / 2 && y < size / 2 ? [200, 30, 40] : x >= size / 2 && y >= size / 2 ? [40, 80, 180] : [255, 255, 255]
      out.set([c[0], c[1], c[2], 255], (y * size + x) * 4)
    }
  }
  return out
}

describe('圖片轉刺繡', () => {
  it('前端色票與核心的色票完全一致（避免兩邊漂移）', () => {
    expect(listThreadColors()).toEqual(THREADS)
  })

  it('簡單圖案轉成兩種線色', () => {
    const hoop = listBuiltinHoops().find((h) => h.id === 'b70-medium')!
    const r = buildImagePreview(
      { width: 64, height: 64, color_count: 3, simplify: 30, size: 600, hoop },
      logoRgba(64),
    )
    expect(r.blocked).toBeNull()
    expect(r.plan.colors).toHaveLength(2)
    expect(r.plan.stitches.at(-1)?.kind).toBe('End')
  })

  it('建議的顏色數在範圍內', () => {
    const k = suggestColorCount(logoRgba(64), 64, 64)
    expect(k).toBeGreaterThanOrEqual(2)
    expect(k).toBeLessThanOrEqual(8)
  })

  it('單色圖被阻止並帶白話說明', () => {
    const hoop = listBuiltinHoops()[1]
    const flat = new Uint8Array(16 * 16 * 4).fill(255)
    const r = buildImagePreview(
      { width: 16, height: 16, color_count: 3, simplify: 30, size: 600, hoop },
      flat,
    )
    expect(r.blocked?.code).toBe('single_color')
  })
})
