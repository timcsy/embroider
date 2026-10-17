import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { describe, expect, it } from 'vitest'
import { createDirectClient } from '../src/lib/coreClient'
import { FONT_FILES } from '../src/lib/fonts'
import type { TextRequest } from '../src/lib/types'

const wasmBytes = () => readFileSync(resolve(process.cwd(), 'src/lib/wasm/stitch_wasm_bg.wasm'))
const loadFont = async (file: string) =>
  new Uint8Array(readFileSync(resolve(process.cwd(), '..', 'fonts', file)))

describe('核心用戶端（直接呼叫版）', () => {
  it('字型清單固定四種且檔名存在', () => {
    expect(FONT_FILES.map((f) => f.id)).toEqual(['sans', 'serif', 'round', 'kai'])
    for (const f of FONT_FILES) {
      expect(() => readFileSync(resolve(process.cwd(), '..', 'fonts', f.file))).not.toThrow()
    }
  })

  it('準備好之後可以預覽與輸出', async () => {
    const client = createDirectClient({ wasm: wasmBytes(), loadFont })
    await client.ready
    const hoops = await client.listBuiltinHoops()
    const req: TextRequest = {
      text: '王小明',
      font_id: 'kai',
      height: 150,
      color: { r: 200, g: 30, b: 40, name: '正紅' },
      layout: { kind: 'single_line' },
      hoop: hoops[1],
    }
    const preview = await client.buildTextPreview(req)
    expect(preview.blocked).toBeNull()
    const out = await client.encodeAll(preview.plan, hoops[1], [], '王小明', 1)
    expect(out.files!.map((f) => f.fileName)).toEqual(['EMB0001.DST', 'EMB0001.EXP'])
  })

  it('載入失敗時 ready 會被拒絕', async () => {
    const client = createDirectClient({
      wasm: wasmBytes(),
      loadFont: async () => {
        throw new Error('讀不到')
      },
    })
    await expect(client.ready).rejects.toThrow()
  })
})
