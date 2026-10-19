// 正式版的離線服務與更新：開機就註冊，之後每分鐘、以及回到分頁時去看有沒有新版本。
import { registerSW } from 'virtual:pwa-register'
import { createUpdater, type Updater } from './updates'

/** 編輯頁在這裡掛上「把做到一半的設計存起來」，更新之前會先呼叫它。 */
export const flushHook: { current: () => Promise<void> } = { current: async () => {} }

export const updater: Updater | null = import.meta.env.PROD
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
