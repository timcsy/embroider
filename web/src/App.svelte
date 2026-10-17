<script lang="ts">
  import { pageFontsBase } from './lib/assets'
  import * as core from './lib/core'
  import { createDirectClient, createWorkerClient, type CoreClient } from './lib/coreClient'
  import { createHoopStore, type HoopStore } from './lib/hoops'
  import { createPrefs, type Prefs } from './lib/prefs'
  import { strings } from './lib/strings.zh-TW'
  import type { OutputFormatInfo } from './lib/types'
  import DesignPage from './routes/DesignPage.svelte'
  import TryOnMachine from './routes/TryOnMachine.svelte'

  function makeClient(): CoreClient {
    if (typeof Worker !== 'undefined') return createWorkerClient()
    return createDirectClient({
      loadFont: async (file) => new Uint8Array(await (await fetch(`${pageFontsBase()}${file}`)).arrayBuffer()),
    })
  }

  const client = makeClient()

  interface Ready {
    store: HoopStore
    prefs: Prefs
    formats: OutputFormatInfo[]
  }

  // 重運算（預覽、輸出）在 Worker；繡框驗證這類小函式在主執行緒也載入一份核心
  const boot: Promise<Ready> = Promise.all([client.ready, core.initCore()]).then(() => {
    const formats = core.listFormats()
    return {
      store: createHoopStore(core.listBuiltinHoops(), core.validateHoop),
      prefs: createPrefs(formats.map((f) => f.id)),
      formats,
    }
  })

  let screen = $state<'text' | 'try'>('text')
</script>

{#await boot}
  <p class="status" role="status">{strings.loading}</p>
{:then ready}
  <!-- 切換畫面時不卸載文字頁，這樣回來時輸入的內容與復原紀錄都還在 -->
  <div hidden={screen !== 'text'}>
    <DesignPage {client} store={ready.store} prefs={ready.prefs} formats={ready.formats} onTry={() => (screen = 'try')} />
  </div>
  {#if screen === 'try'}
    <TryOnMachine formats={ready.formats} prefs={ready.prefs} onBack={() => (screen = 'text')} />
  {/if}
{:catch}
  <p class="status" role="alert">{strings.loadFailed}</p>
{/await}

<style>
  .status { text-align: center; margin: 30vh 16px 0; font-size: 1.2em; }
</style>
