import { describe, expect, it } from 'vitest'
import { createTips, TIP_ORDER } from '../src/lib/tips'

function memory() {
  const data = new Map<string, string>()
  return {
    getItem: (k: string) => data.get(k) ?? null,
    setItem: (k: string, v: string) => void data.set(k, v),
  }
}

describe('第一次使用的提示', () => {
  it('依序三個：加東西、拖移、改大小與轉方向', () => {
    expect(TIP_ORDER).toEqual(['add', 'move', 'resize'])
  })

  it('一開始顯示第一個', () => {
    const t = createTips(memory())
    expect(t.current()).toBe('add')
  })

  it('做到那個動作，提示就換成下一個；全部做完就沒有了', () => {
    const t = createTips(memory())
    t.did('add')
    expect(t.current()).toBe('move')
    t.did('move')
    expect(t.current()).toBe('resize')
    t.did('resize')
    expect(t.current()).toBeNull()
  })

  it('做了不是現在提示的動作，不會跳過提示', () => {
    const t = createTips(memory())
    t.did('resize')
    expect(t.current()).toBe('add')
  })

  it('已經做過的動作之後再做，不會把提示倒退', () => {
    const t = createTips(memory())
    t.did('add')
    t.did('add')
    expect(t.current()).toBe('move')
  })

  it('按「不用了」：永遠不再顯示', () => {
    const store = memory()
    const t = createTips(store)
    t.dismiss()
    expect(t.current()).toBeNull()
    expect(createTips(store).current()).toBeNull()
  })

  it('重新打開還記得做到哪一步', () => {
    const store = memory()
    const a = createTips(store)
    a.did('add')
    expect(createTips(store).current()).toBe('move')
  })

  it('儲存空間不能用（私密瀏覽）：不會出錯，這次照常提示', () => {
    const broken = {
      getItem: () => {
        throw new Error('blocked')
      },
      setItem: () => {
        throw new Error('blocked')
      },
    }
    const t = createTips(broken)
    expect(t.current()).toBe('add')
    expect(() => t.did('add')).not.toThrow()
    expect(t.current()).toBe('move')
  })

  it('已經有設計的人（重開接著做）不需要「加東西」的提示', () => {
    const t = createTips(memory())
    t.skipTo('move')
    expect(t.current()).toBe('move')
  })

  it('狀態變化會通知畫面', () => {
    const t = createTips(memory())
    let n = 0
    t.onChange(() => n++)
    t.did('add')
    t.dismiss()
    expect(n).toBe(2)
  })
})
