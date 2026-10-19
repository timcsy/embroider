import { describe, expect, it, vi } from 'vitest'
import { createUpdater, type Register } from '../src/lib/updates'

function setup(opts: { flush?: () => Promise<void> } = {}) {
  let needRefresh = () => {}
  const updateSW = vi.fn(async () => {})
  const registration = { update: vi.fn(async () => {}) }
  const register: Register = (o) => {
    needRefresh = o.onNeedRefresh
    o.onRegistered(registration)
    return updateSW
  }
  const timers: { fn: () => void; ms: number }[] = []
  const u = createUpdater(register, {
    flush: opts.flush,
    intervalMs: 60_000,
    setInterval: (fn, ms) => void timers.push({ fn, ms }),
  })
  return { u, updateSW, registration, timers, fire: () => needRefresh() }
}

describe('新版本更新', () => {
  it('一開始沒有新版本；發現新版本就通知畫面', () => {
    const { u, fire } = setup()
    const seen: boolean[] = []
    u.onChange(() => seen.push(u.available()))
    expect(u.available()).toBe(false)
    fire()
    expect(u.available()).toBe(true)
    expect(seen).toEqual([true])
  })

  it('每隔一分鐘自己檢查有沒有新版本', () => {
    const { timers, registration } = setup()
    expect(timers).toHaveLength(1)
    expect(timers[0].ms).toBe(60_000)
    timers[0].fn()
    expect(registration.update).toHaveBeenCalledTimes(1)
  })

  it('馬上檢查：回到分頁時用', async () => {
    const { u, registration } = setup()
    await u.checkNow()
    expect(registration.update).toHaveBeenCalledTimes(1)
  })

  it('檢查時沒網路（丟例外）也不會出事', async () => {
    const { u, registration } = setup()
    registration.update.mockRejectedValueOnce(new Error('offline'))
    await expect(u.checkNow()).resolves.toBeUndefined()
  })

  it('更新：先把做到一半的設計存好，再切換到新版本並重新整理', async () => {
    const order: string[] = []
    const { u, updateSW, fire } = setup({ flush: async () => void order.push('flush') })
    updateSW.mockImplementation(async () => void order.push('update'))
    fire()
    await u.apply()
    expect(order).toEqual(['flush', 'update'])
    expect(updateSW).toHaveBeenCalledWith(true)
  })

  it('存檔失敗也照樣更新（不能因為這樣卡在舊版）', async () => {
    const { u, updateSW, fire } = setup({
      flush: async () => {
        throw new Error('disk')
      },
    })
    fire()
    await u.apply()
    expect(updateSW).toHaveBeenCalledTimes(1)
  })

  it('連按兩次只更新一次', async () => {
    const { u, updateSW, fire } = setup()
    fire()
    await Promise.all([u.apply(), u.apply()])
    expect(updateSW).toHaveBeenCalledTimes(1)
  })

  it('沒有新版本時按更新什麼都不做', async () => {
    const { u, updateSW } = setup()
    await u.apply()
    expect(updateSW).not.toHaveBeenCalled()
  })
})
