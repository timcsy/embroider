import { describe, expect, it } from 'vitest'
import { createLatestRunner } from '../src/lib/scheduler'

function deferred() {
  let resolve!: () => void
  const promise = new Promise<void>((r) => (resolve = r))
  return { promise, resolve }
}

describe('只算最新的（畫布更新）', () => {
  it('沒有在算的時候，馬上開始（不用等）', () => {
    const started: string[] = []
    const r = createLatestRunner<string>(async (j) => void started.push(j))
    r.submit('a', 'a')
    expect(started).toEqual(['a'])
  })

  it('正在算的時候又改了很多次：只留最後一次，算完立刻接著算', async () => {
    const started: string[] = []
    const gates = [deferred(), deferred()]
    let n = 0
    const r = createLatestRunner<string>(async (j) => {
      started.push(j)
      await gates[n++]?.promise
    })
    r.submit('1', '1')
    r.submit('2', '2')
    r.submit('3', '3')
    r.submit('4', '4')
    expect(started).toEqual(['1'])
    gates[0].resolve()
    await Promise.resolve()
    await Promise.resolve()
    await new Promise((x) => setTimeout(x, 0))
    expect(started).toEqual(['1', '4'])
    gates[1].resolve()
  })

  it('內容一樣（鍵一樣）就不重算：只是選了別的東西不該重算', () => {
    const started: string[] = []
    const r = createLatestRunner<string>(async (j) => void started.push(j))
    r.submit('a', 'k')
    r.submit('a2', 'k')
    expect(started).toEqual(['a'])
  })

  it('忙碌狀態：開始算是 true，全部算完是 false，並通知畫面', async () => {
    const gate = deferred()
    const r = createLatestRunner<string>(async () => gate.promise)
    const seen: boolean[] = []
    r.onBusyChange(() => seen.push(r.busy()))
    expect(r.busy()).toBe(false)
    r.submit('a', 'a')
    expect(r.busy()).toBe(true)
    gate.resolve()
    await new Promise((x) => setTimeout(x, 0))
    expect(r.busy()).toBe(false)
    expect(seen).toEqual([true, false])
  })

  it('一次算壞了（丟例外）不會卡住後面的', async () => {
    const started: string[] = []
    let first = true
    const r = createLatestRunner<string>(async (j) => {
      started.push(j)
      if (first) {
        first = false
        throw new Error('boom')
      }
    })
    r.submit('a', 'a')
    await new Promise((x) => setTimeout(x, 0))
    r.submit('b', 'b')
    expect(started).toEqual(['a', 'b'])
    expect(r.busy()).toBe(true || false)
  })

  it('取消：清掉排隊中的，目前正在算的算完不再接著算', async () => {
    const started: string[] = []
    const gate = deferred()
    const r = createLatestRunner<string>(async (j) => {
      started.push(j)
      await gate.promise
    })
    r.submit('1', '1')
    r.submit('2', '2')
    r.cancel()
    gate.resolve()
    await new Promise((x) => setTimeout(x, 0))
    expect(started).toEqual(['1'])
    expect(r.busy()).toBe(false)
  })

  it('取消之後再送同樣內容也會重算（鍵被清掉了）', async () => {
    const started: string[] = []
    const r = createLatestRunner<string>(async (j) => void started.push(j))
    r.submit('a', 'k')
    await new Promise((x) => setTimeout(x, 0))
    r.cancel()
    r.submit('a', 'k')
    expect(started).toEqual(['a', 'a'])
  })
})
