<script lang="ts">
  import { pageFontsBase } from './lib/assets'
  import * as core from './lib/core'
  import { createDirectClient, createWorkerClient, type CoreClient } from './lib/coreClient'
  import { updater } from './lib/appUpdater'
  import { createFontManager, type FontManager } from './lib/fontManager'
  import { fetchFontBytes } from './lib/fontFetch'
  import { FONT_FILES, PRELOADED_FONTS } from './lib/fonts'
  import { createHoopStore, type HoopStore } from './lib/hoops'
  import { createPrefs, type Prefs } from './lib/prefs'
  import { strings } from './lib/strings.zh-TW'
  import type { OutputFormatInfo } from './lib/types'
  import EditorPage from './routes/EditorPage.svelte'
  import TryOnMachine from './routes/TryOnMachine.svelte'

  function makeClient(): CoreClient {
    if (typeof Worker !== 'undefined') return createWorkerClient()
    return createDirectClient({
      loadFont: async (file) => new Uint8Array(await (await fetch(`${pageFontsBase()}${file}`)).arrayBuffer()),
    })
  }

  const client = makeClient()

  interface Ready {
    fonts: FontManager
    store: HoopStore
    prefs: Prefs
    formats: OutputFormatInfo[]
  }

  // 重運算（預覽、輸出）在 Worker；繡框驗證這類小函式在主執行緒也載入一份核心
  const boot: Promise<Ready> = Promise.all([client.ready, core.initCore()]).then(() => {
    const formats = core.listFormats()
    // 基本字型開機就載入；其他字型第一次用到才下載（下載過的，瀏覽器會留著，之後離線也能用）
    const fonts = createFontManager({
      fonts: FONT_FILES,
      preloaded: PRELOADED_FONTS,
      fetchFont: (file) => fetchFontBytes(`${pageFontsBase()}${file}`),
      registerFont: (id, name, bytes) => client.registerFont(id, name, bytes),
    })
    return {
      fonts,
      store: createHoopStore(core.listBuiltinHoops(), core.validateHoop),
      prefs: createPrefs(formats.map((f) => f.id)),
      formats,
    }
  })

  let screen = $state<'text' | 'try'>('text')

  // 新版本：下載好了就通知；沒有做到一半的東西時直接自動更新（不然使用者開著的一直是舊版）
  let updateReady = $state(updater?.available() ?? false)
  let hasWork = $state(false)
  $effect(() => updater?.onChange(() => (updateReady = updater?.available() ?? false)))
  $effect(() => {
    if (updateReady && !hasWork) void updater?.apply()
  })
</script>

{#if updateReady && hasWork}
  <div class="update" role="status" data-testid="update-banner">
    <span>{strings.update.available}<small>{strings.update.note}</small></span>
    <button onclick={() => updater?.apply()} data-testid="update-now">{strings.update.now}</button>
  </div>
{/if}

{#await boot}
  <p class="status" role="status">{strings.loading}</p>
{:then ready}
  <!-- 切換畫面時不卸載編輯頁，這樣回來時做到一半的設計與復原紀錄都還在 -->
  <div hidden={screen !== 'text'}>
    <EditorPage {client} fonts={ready.fonts} store={ready.store} prefs={ready.prefs} formats={ready.formats} onTry={() => (screen = 'try')} onWork={(w) => (hasWork = w)} />
  </div>
  {#if screen === 'try'}
    <TryOnMachine formats={ready.formats} prefs={ready.prefs} onBack={() => (screen = 'text')} />
  {/if}
{:catch}
  <p class="status" role="alert">{strings.loadFailed}</p>
{/await}

<style>
  .update { position: sticky; top: 0; z-index: 30; display: flex; align-items: center; justify-content: space-between; gap: 12px; padding: 8px 16px; background: #e8f3ff; border-bottom: 2px solid #9db8e8; }
  .update span { display: flex; flex-direction: column; }
  .update small { color: var(--muted); font-size: 0.8em; }
  .update button { min-height: 48px; padding: 0 18px; border: 2px solid var(--accent); border-radius: 12px; background: #fff; font-weight: 700; white-space: nowrap; }
  .status { text-align: center; margin: 30vh 16px 0; font-size: 1.2em; }
</style>
