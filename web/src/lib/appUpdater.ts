// 正式版的離線服務與更新：開機就註冊，之後每分鐘、以及回到分頁時去看有沒有新版本。
import { registerSW } from 'virtual:pwa-register'
import { createUpdater, type Updater } from './updates'

/** 編輯頁在這裡掛上「把做到一半的設計存起來」，更新之前會先呼叫它。 */
export const flushHook: { current: () => Promise<void> } = { current: async () => {} }

/**
 * 在自己的電腦上開發、試用（localhost）時不用離線快取：每次改動重新建置，開著的畫面就是最新的，
 * 不會卡在舊版。要在 localhost 測離線，在瀏覽器的 localStorage 設 embroider.sw=on。
 * 同時把先前註冊過的離線服務與快取清掉，免得留著舊版。
 */
export function wantsServiceWorker(host: string, flag: string | null): boolean {
  const local = host === 'localhost' || host === '127.0.0.1' || host === '[::1]' || host.endsWith('.local')
  return !local || flag === 'on'
}

async function clearOldWorkers(): Promise<void> {
  try {
    const regs = await navigator.serviceWorker?.getRegistrations()
    await Promise.all((regs ?? []).map((r) => r.unregister()))
    if (typeof caches !== 'undefined') await Promise.all((await caches.keys()).map((k) => caches.delete(k)))
  } catch {
    // 清不掉也沒關係
  }
}

const useWorker = (() => {
  if (typeof location === 'undefined') return false
  let flag: string | null = null
  try {
    flag = localStorage.getItem('embroider.sw')
  } catch {
    // 讀不到就當作沒設
  }
  return wantsServiceWorker(location.hostname, flag)
})()

if (import.meta.env.PROD && !useWorker) void clearOldWorkers()

export const updater: Updater | null = import.meta.env.PROD && useWorker
  ? createUpdater(
      (o) =>
        registerSW({
          immediate: true,
          onNeedRefresh: o.onNeedRefresh,
          onRegisteredSW: (_url, r) => o.onRegistered(r),
        }),
      { flush: () => flushHook.current() },
    )
  : null

if (updater && typeof document !== 'undefined') {
  document.addEventListener('visibilitychange', () => {
    if (document.visibilityState === 'visible') void updater.checkNow()
  })
}
