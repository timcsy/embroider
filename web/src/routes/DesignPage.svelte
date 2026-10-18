<script lang="ts">
  import { untrack } from 'svelte'
  import ExportSheet from '../components/ExportSheet.svelte'
  import HoopForm from '../components/HoopForm.svelte'
  import HoopPicker from '../components/HoopPicker.svelte'
  import Notice from '../components/Notice.svelte'
  import PreviewCanvas from '../components/PreviewCanvas.svelte'
  import type { CoreClient } from '../lib/coreClient'
  import { downloadFiles, nextSequence } from '../lib/export'
  import { listRecent, removeRecent, saveRecent, type SavedDesign } from '../lib/designs'
  import { fitImageSizeCm } from '../lib/fitSize'
  import { cropRgba, type CropRect } from '../lib/crop'
  import ImageCrop from '../components/ImageCrop.svelte'
  import { dataUrlToBytes, fileFromBytes, loadImage, stemOf, type LoadedImage } from '../lib/imageInput'
  import { formatsToExport, type Prefs } from '../lib/prefs'
  import { canShareFiles, shareFiles } from '../lib/share'
  import { FONT_FILES } from '../lib/fonts'
  import { History } from '../lib/history'
  import { strings } from '../lib/strings.zh-TW'
  import { THREADS } from '../lib/threads'
  import type { HoopInput, HoopStore } from '../lib/hoops'
  import type { EncodedFile, Hoop, OutputFormatInfo, PreviewResult, UserMessage } from '../lib/types'

  let {
    client,
    store,
    prefs,
    formats,
    onTry,
  }: {
    client: CoreClient
    store: HoopStore
    prefs: Prefs
    formats: OutputFormatInfo[]
    onTry: () => void
  } = $props()

  // 整份替換、不局部修改，用 raw：代理物件無法傳給 Worker
  let hoops = $state.raw<Hoop[]>(untrack(() => store.list()))

  interface Design {
    mode: 'text' | 'image'
    text: string
    fontId: string
    /** 字的大小，公分 */
    sizeCm: number
    colorIdx: number
    hoopId: string
    /** 圖片：長邊大小（公分）、顏色多寡、簡化程度 */
    imageSizeCm: number
    colors: number
    simplify: number
  }

  const initial: Design = {
    mode: 'text',
    imageSizeCm: 8,
    colors: 4,
    simplify: 30,
    text: '',
    fontId: 'sans',
    sizeCm: 2.5,
    colorIdx: 0,
    hoopId: untrack(() => {
      const all = store.list()
      return (
        all.find((h) => h.id === prefs.lastHoopId)?.id ??
        all.find((h) => h.id === 'b70-medium')?.id ??
        all[0].id
      )
    }),
  }
  const history = new History<Design>(initial)
  let design = $state<Design>({ ...initial })
  let canUndo = $state(false)
  let canRedo = $state(false)

  function update(patch: Partial<Design>, key?: string) {
    design = { ...design, ...patch }
    history.push({ ...design }, { coalesce: key })
    canUndo = history.canUndo
    canRedo = history.canRedo
  }
  function undo() {
    design = { ...history.undo() }
    canUndo = history.canUndo
    canRedo = history.canRedo
  }
  function redo() {
    design = { ...history.redo() }
    canUndo = history.canUndo
    canRedo = history.canRedo
  }

  const hoop = $derived(hoops.find((h) => h.id === design.hoopId) ?? hoops[0])

  $effect(() => {
    prefs.setLastHoop(design.hoopId)
  })
  const hasText = $derived(design.text.trim().length > 0)

  // 選到的圖片（像素很大，不放進復原紀錄；整份替換，用 raw）
  let image = $state.raw<LoadedImage | null>(null)
  // 圈出來要繡的範圍（沒有就是整張圖）；跟圖片一樣不放進復原紀錄
  let crop = $state<CropRect | null>(null)
  const source = $derived(image ? cropRgba(image.rgba, image.width, image.height, crop) : null)
  function setCrop(r: CropRect | null) {
    crop = r
    if (image) {
      const c = cropRgba(image.rgba, image.width, image.height, r)
      update({ imageSizeCm: fitImageSizeCm(hoop, c.width, c.height) }, 'imageSize')
    }
  }
  let imageProblem = $state<UserMessage | null>(null)
  let loadingImage = $state(false)
  let fileInput = $state<HTMLInputElement>()
  let cameraInput = $state<HTMLInputElement>()

  const hasInput = $derived(design.mode === 'text' ? hasText : image !== null)

  // 最近做過的設計（存在這支手機上）
  let recent = $state.raw<SavedDesign[]>([])
  const refreshRecent = async () => (recent = await listRecent())
  $effect(() => {
    void refreshRecent()
  })

  const recentId = () =>
    design.mode === 'text' ? `text:${design.text.trim()}` : `image:${image?.name ?? ''}`

  async function rememberCurrent() {
    const params = { ...design }
    const saved = await saveRecent({
      id: recentId(),
      name: design.mode === 'text' ? design.text.trim() : (image?.name ?? ''),
      params,
      image:
        design.mode === 'image' && image
          ? { name: image.name, width: image.width, height: image.height, png: dataUrlToBytes(image.previewUrl) }
          : undefined,
    })
    if (saved) await refreshRecent()
  }

  async function restore(d: SavedDesign) {
    imageProblem = null
    if (d.image) {
      const r = await loadImage(fileFromBytes(d.image.png, d.image.name))
      if ('code' in r) {
        imageProblem = r
        return
      }
      image = r
      crop = null
    }
    const p = d.params as Partial<Design>
    const hoopOk = hoops.some((h) => h.id === p.hoopId)
    design = { ...design, ...p, hoopId: hoopOk ? (p.hoopId as string) : design.hoopId }
    history.push({ ...design })
    canUndo = history.canUndo
    canRedo = history.canRedo
  }

  async function forget(d: SavedDesign) {
    await removeRecent(d.id)
    await refreshRecent()
  }

  async function onFile(e: Event) {
    const input = e.currentTarget as HTMLInputElement
    const file = input.files?.[0]
    input.value = ''
    if (!file) return
    loadingImage = true
    imageProblem = null
    try {
      const r = await loadImage(file)
      if ('code' in r) {
        imageProblem = r
        return
      }
      image = r
      crop = null
      const k = await client.suggestColorCount(r.rgba, r.width, r.height)
      design = {
        ...design,
        colors: Math.min(12, Math.max(2, k + 1)),
        imageSizeCm: fitImageSizeCm(hoop, r.width, r.height),
      }
    } finally {
      loadingImage = false
    }
  }

  // 整份替換、不局部修改，所以用 raw：代理物件無法傳給 Worker（structured clone 會失敗）
  let preview = $state.raw<PreviewResult | null>(null)
  let computing = $state(false)
  let token = 0

  $effect(() => {
    const d = { ...design }
    const h = hoop
    const img = source
    const empty = d.mode === 'text' ? d.text.trim().length === 0 : img === null
    if (empty) {
      preview = null
      computing = false
      token++
      return
    }
    const mine = ++token
    computing = true
    const timer = setTimeout(async () => {
      try {
        const result =
          d.mode === 'text'
            ? await client.buildTextPreview({
                text: d.text,
                font_id: d.fontId,
                height: Math.round(d.sizeCm * 100),
                color: THREADS[d.colorIdx],
                layout: { kind: 'single_line' },
                hoop: h,
              })
            : await client.buildImagePreview(
                {
                  width: img!.width,
                  height: img!.height,
                  color_count: d.colors,
                  simplify: d.simplify,
                  size: Math.round(d.imageSizeCm * 100),
                  hoop: h,
                },
                img!.rgba,
              )
        if (mine === token) preview = result
      } finally {
        if (mine === token) computing = false
      }
    }, 150)
    return () => clearTimeout(timer)
  })

  const sizeText = $derived.by(() => {
    const st = preview?.plan.stitches.filter((s) => s.kind === 'Stitch' || s.kind === 'Jump') ?? []
    if (st.length === 0) return ''
    const xs = st.map((s) => s.x)
    const ys = st.map((s) => s.y)
    const w = (Math.max(...xs) - Math.min(...xs)) / 100
    const h = (Math.max(...ys) - Math.min(...ys)) / 100
    return strings.text.actualSize(w.toFixed(1), h.toFixed(1))
  })

  let saving = $state(false)
  let failure = $state<UserMessage | null>(null)
  let exported = $state<{ name: string; files: EncodedFile[]; firstTime: boolean } | null>(null)

  async function save(mode: 'default' | 'all') {
    if (!preview || preview.blocked || saving) return
    saving = true
    failure = null
    try {
      const firstTime = prefs.isFirstTime || mode === 'all'
      const out = await client.encodeAll(
        preview.plan,
        hoop,
        formatsToExport(prefs.defaultFormats, mode),
        design.mode === 'text' ? design.text : stemOf(image?.name ?? ''),
        nextSequence(),
      )
      if (out.error || !out.files) {
        failure = out.error
        return
      }
      await downloadFiles(out.files)
      void rememberCurrent()
      exported = {
        name: design.mode === 'text' ? design.text : (image?.name ?? ''),
        files: out.files,
        firstTime: firstTime && prefs.isFirstTime,
      }
    } finally {
      saving = false
    }
  }

  async function share() {
    if (!exported) return
    const r = await shareFiles(exported.files)
    if (!r.ok && !r.cancelled) failure = r.message
  }

  // prefs 不是響應式的，訂閱它的變更通知來更新「目前預設」
  let prefsVersion = $state(0)
  $effect(() => prefs.onChange(() => prefsVersion++))

  const defaultLabel = $derived.by(() => {
    void prefsVersion
    const names = formats.filter((f) => prefs.defaultFormats.includes(f.id)).map((f) => f.label)
    return names.length === 0 ? strings.export.defaultAll : strings.export.currentDefault(names.join('、'))
  })

  // 繡框的新增、修改、刪除
  let editing = $state<{ hoop?: Hoop } | null>(null)

  function refreshHoops(selectId?: string) {
    hoops = store.list()
    const wanted = selectId ?? design.hoopId
    const id = hoops.some((h) => h.id === wanted) ? wanted : (hoops.find((h) => h.id === 'b70-medium') ?? hoops[0]).id
    if (id !== design.hoopId) update({ hoopId: id })
  }

  function submitHoop(input: HoopInput): UserMessage | null {
    const target = editing?.hoop
    const r = target ? store.update(target.id, input) : store.add(input)
    if (!r.ok) return r.message
    editing = null
    refreshHoops(r.hoop.id)
    return null
  }

  function removeHoop(h: Hoop) {
    store.remove(h.id)
    refreshHoops()
  }
</script>

<main>
  <header>
    <h1>{strings.appName}</h1>
    <div class="history">
      <button onclick={undo} disabled={!canUndo} aria-label={strings.actions.undo}>↶ {strings.actions.undo}</button>
      <button onclick={redo} disabled={!canRedo} aria-label={strings.actions.redo}>{strings.actions.redo} ↷</button>
    </div>
  </header>

  <div class="tabs" role="tablist" aria-label={strings.appName}>
    <button role="tab" aria-selected={design.mode === 'text'} class:on={design.mode === 'text'} onclick={() => update({ mode: 'text' })} data-testid="tab-text">{strings.tabs.text}</button>
    <button role="tab" aria-selected={design.mode === 'image'} class:on={design.mode === 'image'} onclick={() => update({ mode: 'image' })} data-testid="tab-image">{strings.tabs.image}</button>
  </div>

  {#if design.mode === 'text'}
    <section class="card">
      <label for="text"><h2>{strings.text.title}</h2></label>
      <textarea
        id="text"
        rows="2"
        placeholder={strings.text.placeholder}
        value={design.text}
        oninput={(e) => update({ text: e.currentTarget.value }, 'text')}
      ></textarea>
    </section>
  {:else}
    <section class="card">
      <h2>{strings.image.title}</h2>
      <input type="file" accept="image/*" hidden bind:this={fileInput} onchange={onFile} data-testid="file-input" />
      <input type="file" accept="image/*" capture="environment" hidden bind:this={cameraInput} onchange={onFile} />
      {#if image}
        <ImageCrop src={image.previewUrl} alt={image.name} rect={crop} onchange={setCrop} />
      {/if}
      <div class="pick">
        <button class="secondary" onclick={() => fileInput?.click()} data-testid="choose-image">
          {image ? strings.image.change : strings.image.choose}
        </button>
        <button class="secondary" onclick={() => cameraInput?.click()}>{strings.image.takePhoto}</button>
      </div>
      <p class="muted">{strings.image.hint}</p>
      {#if loadingImage}<p class="muted" role="status">{strings.image.loading}</p>{/if}
      {#if imageProblem}<Notice message={imageProblem} tone="warn" />{/if}
    </section>
  {/if}

  <section class="card">
    <h2>{strings.text.preview}</h2>
    <PreviewCanvas plan={preview && !preview.blocked ? preview.plan : null} {hoop} label={strings.text.preview} />
    {#if !hasInput}
      <p class="muted">{design.mode === 'text' ? strings.text.empty : strings.image.empty}</p>
    {:else if computing && !preview}
      <p class="muted" role="status">{strings.loading}</p>
    {:else if sizeText}
      <p class="muted" data-testid="size">{sizeText}</p>
    {/if}
    {#if computing && preview}
      <p class="muted" role="status">{strings.updating}</p>
    {/if}
    {#if preview?.blocked}
      <Notice message={preview.blocked} tone="warn" />
    {/if}
    {#each preview?.adjustments ?? [] as m (m.code + m.text)}
      <Notice message={m} />
    {/each}
    {#if failure}
      <Notice message={failure} tone="warn" />
    {/if}
  </section>

  <section class="card">
    {#if design.mode === 'text'}
    <h2>{strings.text.font}</h2>
    <div class="chips" role="radiogroup" aria-label={strings.text.font}>
      {#each FONT_FILES as f (f.id)}
        <button
          role="radio"
          aria-checked={design.fontId === f.id}
          class:on={design.fontId === f.id}
          onclick={() => update({ fontId: f.id })}
        >{strings.fonts[f.id]}</button>
      {/each}
    </div>

    <h2>{strings.text.size}</h2>
    <div class="size">
      <input
        type="range"
        min="0.5"
        max="10"
        step="0.1"
        value={design.sizeCm}
        aria-label={strings.text.size}
        oninput={(e) => update({ sizeCm: Number(e.currentTarget.value) }, 'size')}
      />
      <output>{design.sizeCm.toFixed(1)} {strings.text.sizeUnit}</output>
    </div>

    <h2>{strings.text.color}</h2>
    <div class="swatches" role="radiogroup" aria-label={strings.text.color}>
      {#each THREADS as t, i (t.name)}
        <button
          role="radio"
          aria-checked={design.colorIdx === i}
          aria-label={t.name}
          class:on={design.colorIdx === i}
          style="--c: rgb({t.r},{t.g},{t.b})"
          onclick={() => update({ colorIdx: i })}
        ><span class="dot"></span><span class="name">{t.name}</span></button>
      {/each}
    </div>

    {:else}
    <h2>{strings.image.colors}</h2>
    <div class="size">
      <span class="end">{strings.image.colorsFew}</span>
      <input
        type="range"
        min="2"
        max="12"
        step="1"
        value={design.colors}
        aria-label={strings.image.colors}
        oninput={(e) => update({ colors: Number(e.currentTarget.value) }, 'colors')}
      />
      <span class="end">{strings.image.colorsMany}</span>
    </div>
    <p class="muted center">{strings.image.colorsCount(design.colors)}</p>

    <h2>{strings.image.simplify}</h2>
    <div class="size">
      <span class="end">{strings.image.simplifyLow}</span>
      <input
        type="range"
        min="0"
        max="100"
        step="5"
        value={design.simplify}
        aria-label={strings.image.simplify}
        oninput={(e) => update({ simplify: Number(e.currentTarget.value) }, 'simplify')}
      />
      <span class="end">{strings.image.simplifyHigh}</span>
    </div>

    <h2>{strings.image.size}</h2>
    <div class="size">
      <input
        type="range"
        min="2"
        max="20"
        step="0.5"
        value={design.imageSizeCm}
        aria-label={strings.image.size}
        oninput={(e) => update({ imageSizeCm: Number(e.currentTarget.value) }, 'imageSize')}
      />
      <output>{design.imageSizeCm.toFixed(1)} {strings.image.sizeUnit}</output>
    </div>

    {/if}

    <h2>{strings.text.hoop}</h2>
    <HoopPicker
      {hoops}
      selectedId={design.hoopId}
      onSelect={(id) => update({ hoopId: id })}
      onAdd={() => (editing = {})}
      onEdit={(h) => (editing = { hoop: h })}
      onRemove={removeHoop}
    />
  </section>

  {#if recent.length > 0}
    <section class="card" data-testid="recent">
      <h2>{strings.recent.title}</h2>
      <ul class="recent">
        {#each recent as d (d.id)}
          <li>
            <span class="rname">{d.name}</span>
            <span class="kind">{d.params.mode === 'image' ? strings.recent.kindImage : strings.recent.kindText}</span>
            <button onclick={() => restore(d)} data-testid="restore">{strings.recent.restore}</button>
            <button class="danger" onclick={() => forget(d)} aria-label="{strings.recent.remove} {d.name}">{strings.recent.remove}</button>
          </li>
        {/each}
      </ul>
    </section>
  {/if}

  <section class="card extra">
    <button class="secondary" onclick={() => save('all')} disabled={!preview || !!preview.blocked || saving || computing} data-testid="save-all">
      {strings.actions.saveAll}
    </button>
    <button class="secondary" onclick={onTry} data-testid="try-entry">{strings.actions.tryEntry}</button>
  </section>

  <div class="save">
    <button
      class="primary"
      onclick={() => save('default')}
      disabled={!preview || !!preview.blocked || saving || computing}
      data-testid="save"
    >{saving ? strings.actions.saving : strings.actions.saveToUsb}</button>
    <p class="default" data-testid="default-note">{defaultLabel}</p>
  </div>
</main>

{#if editing}
  <HoopForm initial={editing.hoop} onSubmit={submitHoop} onCancel={() => (editing = null)} />
{/if}

{#if exported}
  <ExportSheet
    designName={exported.name}
    fileNames={exported.files.map((f) => f.fileName)}
    firstTime={exported.firstTime}
    canShare={canShareFiles(exported.files)}
    onShare={share}
    onTry={() => {
      exported = null
      onTry()
    }}
    onClose={() => (exported = null)}
  />
{/if}

<style>
  main { max-width: 560px; margin: 0 auto; padding: 12px 16px 150px; }
  header { display: flex; align-items: center; justify-content: space-between; gap: 8px; flex-wrap: wrap; }
  h1 { font-size: 1.4em; margin: 6px 0; }
  h2 { font-size: 1.05em; margin: 14px 0 8px; }
  .history { display: flex; gap: 8px; }
  .history button { min-height: 48px; padding: 0 14px; border: 2px solid var(--line); border-radius: 12px; background: var(--card); }
  .history button:disabled { opacity: 0.4; }
  .tabs { display: flex; gap: 8px; margin: 10px 0 0; }
  .tabs button { flex: 1; min-height: 52px; border: 2px solid var(--line); border-radius: 12px; background: #fff; font-weight: 700; }
  .tabs button.on { border-color: var(--accent); background: #fdecec; }
  .pick { display: flex; gap: 10px; }
  .pick .secondary { flex: 1; }
  .end { font-size: 0.8em; color: var(--muted); min-width: 4.2em; text-align: center; }
  .center { text-align: center; }
  .recent { list-style: none; margin: 0; padding: 0; }
  .recent li { display: flex; align-items: center; gap: 8px; padding: 8px 0; border-bottom: 1px solid var(--line); }
  .recent .rname { flex: 1; font-weight: 700; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .recent .kind { color: var(--muted); font-size: 0.8em; }
  .recent button { min-height: 44px; padding: 0 12px; border: 2px solid var(--line); border-radius: 10px; background: #fff; }
  .recent .danger { border-color: #d89a9a; color: #9c1c1c; }
  .card { background: var(--card); border: 1px solid var(--line); border-radius: var(--radius); padding: 4px 16px 16px; margin: 12px 0; }
  .card :first-child { margin-top: 12px; }
  textarea { width: 100%; font-size: 1.3em; padding: 12px; border: 2px solid var(--line); border-radius: 12px; resize: vertical; background: #fff; }
  .muted { color: var(--muted); margin: 8px 0 0; font-size: 0.92em; }
  .chips { display: flex; gap: 10px; flex-wrap: wrap; }
  .chips button { min-height: 56px; min-width: 80px; padding: 6px 16px; border: 2px solid var(--line); border-radius: 12px; background: #fff; display: flex; flex-direction: column; align-items: center; justify-content: center; }
  .chips button.on { border-color: var(--accent); background: #fdecec; font-weight: 700; }
  .size { display: flex; align-items: center; gap: 12px; }
  .size input { flex: 1; height: 44px; accent-color: var(--accent); }
  .size output { min-width: 5.5em; text-align: right; font-weight: 700; }
  .swatches { display: grid; grid-template-columns: repeat(4, 1fr); gap: 8px; }
  .swatches button { min-height: 64px; border: 2px solid var(--line); border-radius: 12px; background: #fff; display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 2px; padding: 4px; }
  .swatches .dot { width: 28px; height: 28px; border-radius: 50%; background: var(--c); border: 1px solid rgba(0, 0, 0, 0.25); }
  .swatches .name { font-size: 0.72em; }
  .swatches button.on { border-color: var(--accent); background: #fdecec; font-weight: 700; }
  .save { position: fixed; left: 0; right: 0; bottom: 0; padding: 12px 16px calc(8px + env(safe-area-inset-bottom)); background: linear-gradient(transparent, var(--bg) 30%); }
  .extra { display: flex; flex-direction: column; gap: 10px; padding-top: 16px; }
  .secondary { min-height: 56px; border: 2px solid var(--line); border-radius: var(--radius); background: #fff; font-weight: 700; }
  .secondary:disabled { opacity: 0.5; }
  .default { text-align: center; margin: 6px 0 0; font-size: 0.85em; color: var(--muted); }
  .primary { display: block; width: 100%; max-width: 560px; margin: 0 auto; min-height: 60px; border: 0; border-radius: var(--radius); background: var(--accent); color: var(--accent-ink); font-size: 1.2em; font-weight: 700; }
  .primary:disabled { background: #c9b8b0; }
</style>
