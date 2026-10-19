<script lang="ts">
  import { untrack } from 'svelte'
  import EditorCanvas from '../components/EditorCanvas.svelte'
  import ExportSheet from '../components/ExportSheet.svelte'
  import HoopForm from '../components/HoopForm.svelte'
  import HoopPicker from '../components/HoopPicker.svelte'
  import ImageCrop from '../components/ImageCrop.svelte'
  import Notice from '../components/Notice.svelte'
  import { cropRgba, type CropRect } from '../lib/crop'
  import type { CoreClient } from '../lib/coreClient'
  import {
    clearCurrent,
    listRecent,
    loadCurrent,
    removeRecent,
    saveCurrent,
    saveRecent,
    type SavedDesign,
    type SavedImageWithId,
  } from '../lib/designs'
  import {
    appendStroke,
    removeLastStroke,
    startDraw,
    type Brush,
    addFrame,
    flipItem,
    groupSize,
    moveBy,
    toggleLock,
    transformGroup,
    ungroup,
    mirrorCopy,
    repeatCircle,
    repeatRow,
    addImage,
    addShape,
    addText,
    centerItem,
    duplicateItem,
    emptyDoc,
    fromLegacy,
    moveLayer,
    removeItem,
    restoreRemoved,
    selectItem,
    toRequest,
    updateItem,
    type EditorDoc,
    type EditorItem,
    type ImageSource,
    type Removed,
  } from '../lib/editor'
  import { downloadFiles, nextSequence } from '../lib/export'
  import { fitImageSizeCm } from '../lib/fitSize'
  import { FONT_FILES } from '../lib/fonts'
  import { History } from '../lib/history'
  import type { HoopInput, HoopStore } from '../lib/hoops'
  import { dataUrlToBytes, fileFromBytes, loadImage, stemOf, type LoadedImage } from '../lib/imageInput'
  import { formatsToExport, type Prefs } from '../lib/prefs'
  import { canShareFiles, shareFiles } from '../lib/share'
  import { strings } from '../lib/strings.zh-TW'
  import { THREADS } from '../lib/threads'
  import type {
    CompositeResult,
    EncodedFile,
    Hoop,
    OutputFormatInfo,
    ShapeKind,
    UserMessage,
  } from '../lib/types'

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

  // 寬螢幕（平板橫放、電腦）：加東西與存檔放在上面的工具列，畫布與工具區各自捲動
  let winW = $state(typeof window === 'undefined' ? 1000 : window.innerWidth)
  const wide = $derived(winW >= 900)

  // 整份替換、不局部修改，用 raw：代理物件無法傳給 Worker
  let hoops = $state.raw<Hoop[]>(untrack(() => store.list()))

  const firstHoopId = untrack(() => {
    const all = store.list()
    return (
      all.find((h) => h.id === prefs.lastHoopId)?.id ??
      all.find((h) => h.id === 'b70-medium')?.id ??
      all[0].id
    )
  })

  // ── 設計與復原 ──
  interface Snapshot {
    items: EditorItem[]
    hoopId: string
  }
  const history = new History<Snapshot>({ items: [], hoopId: firstHoopId })
  let doc = $state.raw<EditorDoc>(emptyDoc(firstHoopId))
  let canUndo = $state(false)
  let canRedo = $state(false)
  let removedNote = $state<{ removed: Removed; name: string } | null>(null)
  let removedTimer: ReturnType<typeof setTimeout> | undefined

  const hoop = $derived(hoops.find((h) => h.id === doc.hoopId) ?? hoops[0])
  const selected = $derived(doc.items.find((i) => i.id === doc.selectedId) ?? null)

  function sync() {
    canUndo = history.canUndo
    canRedo = history.canRedo
  }
  /** 套用一次修改。同一個鍵連續的修改（拖移、打字）合成一步復原。 */
  function commit(next: EditorDoc, key?: string) {
    doc = next
    history.push({ items: next.items, hoopId: next.hoopId }, { coalesce: key })
    sync()
  }
  function restoreSnapshot(s: Snapshot) {
    const keep = doc.items.some((i) => i.id === doc.selectedId) ? doc.selectedId : null
    doc = { items: s.items, hoopId: s.hoopId, selectedId: s.items.some((i) => i.id === keep) ? keep : null }
    sync()
  }
  const undo = () => restoreSnapshot(history.undo())
  const redo = () => restoreSnapshot(history.redo())

  $effect(() => {
    prefs.setLastHoop(doc.hoopId)
  })

  // ── 圖片來源（像素很大，不放進復原紀錄） ──
  interface Source extends ImageSource {
    previewUrl: string
  }
  let sources = $state.raw<Source[]>([])
  let nextImage = 1
  let imageProblem = $state<UserMessage | null>(null)
  let loadingImage = $state(false)
  let fileInput = $state<HTMLInputElement>()
  let cameraInput = $state<HTMLInputElement>()
  let replacing = $state<string | null>(null)

  function toSource(r: LoadedImage): Source {
    return { id: `img-${nextImage++}`, name: r.name, width: r.width, height: r.height, rgba: r.rgba, previewUrl: r.previewUrl }
  }
  const sourceOf = (it: EditorItem) => (it.kind === 'image' ? sources.find((s) => s.id === it.imageId) : undefined)

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
      const src = toSource(r)
      sources = [...sources, src]
      const target = replacing
      replacing = null
      if (target) {
        commit(
          updateItem(doc, target, {
            imageId: src.id,
            crop: null,
            sizeCm: fitImageSizeCm(hoop, src.width, src.height),
          } as Partial<EditorItem>),
        )
      } else {
        const k = await client.suggestColorCount(r.rgba, r.width, r.height)
        commit(addImage(doc, src, hoop, Math.min(12, Math.max(2, k + 1))))
      }
    } finally {
      loadingImage = false
    }
  }
  function pickImage(forItem?: string) {
    replacing = forItem ?? null
    fileInput?.click()
  }

  // ── 加東西、改東西 ──
  // 同一組的物件預設一起動；選了「只動這一個」才單獨動
  let solo = $state(false)

  function patch(id: string, p: Partial<EditorItem>, key?: string) {
    const it = doc.items.find((i) => i.id === id)
    // 只有位置改變、而且這個物件屬於某一組：整組跟著移動同樣的距離
    if (it && it.groupId && !solo && ('x' in p || 'y' in p) && Object.keys(p).every((k) => k === 'x' || k === 'y')) {
      commit(moveBy(doc, id, (p.x ?? it.x) - it.x, (p.y ?? it.y) - it.y, false), key ? `${key}:${id}` : undefined)
      return
    }
    // 縮放、旋轉也是：整組一起（繞著這一組的中心）
    if (it && it.groupId && !solo && Object.keys(p).length > 0 && Object.keys(p).every((k) => k === 'scale' || k === 'rotation')) {
      const factor = (p.scale ?? it.scale) / it.scale
      const turn = (((((p.rotation ?? it.rotation) - it.rotation + 180) % 360) + 360) % 360) - 180
      commit(transformGroup(doc, id, { factor, turn }), key ? `${key}:${id}` : undefined)
      return
    }
    commit(updateItem(doc, id, p), key ? `${key}:${id}` : undefined)
  }
  function addShapeOf(kind: ShapeKind) {
    commit(addShape(doc, kind))
  }
  function remove(id: string) {
    const it = doc.items.find((i) => i.id === id)
    const r = removeItem(doc, id)
    if (!r.removed || !it) return
    commit(r.doc)
    removedNote = { removed: r.removed, name: nameOf(it) }
    clearTimeout(removedTimer)
    removedTimer = setTimeout(() => (removedNote = null), 10_000)
  }
  function takeBack() {
    if (!removedNote) return
    commit(restoreRemoved(doc, removedNote.removed))
    removedNote = null
  }
  const select = (id: string | null) => {
    doc = selectItem(doc, id)
  }

  function nameOf(it: EditorItem): string {
    if (it.kind === 'text') return it.text.trim() || strings.editor.kinds.text
    if (it.kind === 'image') return stemOf(sourceOf(it)?.name ?? '') || strings.editor.kinds.image
    if (it.kind === 'draw') return strings.editor.kinds.draw
    return strings.editor.shapes[it.shape]
  }

  /** 「大小」滑桿顯示與編輯的是實際公分數：基準大小 × 縮放。 */
  function baseCm(it: EditorItem): number {
    if (it.kind === 'draw') return 1
    return it.kind === 'text' ? it.heightCm : it.kind === 'image' ? it.sizeCm : Math.max(it.widthCm, it.heightCm)
  }
  const sizeRange = (it: EditorItem) =>
    it.kind === 'text'
      ? { min: 0.5, max: 10, step: 0.1 }
      : it.kind === 'image'
        ? { min: 2, max: 20, step: 0.5 }
        : { min: 1, max: 15, step: 0.5 }

  // ── 預覽（核心算好之後才更新，期間畫面上的外框跟著手指即時動） ──
  let result = $state.raw<CompositeResult | null>(null)
  let computing = $state(false)
  let token = 0

  $effect(() => {
    const d = doc
    const h = hoop
    const srcs = sources
    const { request, images } = toRequest(d, h, srcs)
    if (request.items.length === 0) {
      result = null
      computing = false
      token++
      return
    }
    const mine = ++token
    computing = true
    const timer = setTimeout(async () => {
      try {
        const r = await client.buildCompositePreview(request, images)
        if (mine === token) result = r
      } finally {
        if (mine === token) computing = false
      }
    }, 120)
    return () => clearTimeout(timer)
  })

  const preview = $derived(result?.preview ?? null)
  const plan = $derived(preview && !preview.blocked ? preview.plan : null)
  const infos = $derived(result?.items ?? [])
  const fit = $derived(plan ? preview!.fitted_scale : 1)
  const stitchCount = $derived(plan?.stitches.filter((s) => s.kind === 'Stitch').length ?? 0)
  const canSave = $derived(!!plan && stitchCount > 0)
  const selectedInfo = $derived(infos.find((i) => i.id === doc.selectedId) ?? null)
  const isEmptyHint = $derived(preview?.blocked?.code === 'empty_design')

  const sizeText = $derived.by(() => {
    const st = plan?.stitches.filter((s) => s.kind === 'Stitch' || s.kind === 'Jump') ?? []
    if (st.length === 0) return ''
    const xs = st.map((s) => s.x)
    const ys = st.map((s) => s.y)
    const w = (Math.max(...xs) - Math.min(...xs)) / 100
    const h = (Math.max(...ys) - Math.min(...ys)) / 100
    return strings.text.actualSize(w.toFixed(1), h.toFixed(1))
  })

  // ── 最近做過的設計 ──
  let recent = $state.raw<SavedDesign[]>([])
  const refreshRecent = async () => (recent = await listRecent())
  $effect(() => {
    void refreshRecent()
  })

  function designName(): string {
    const t = doc.items.find((i) => i.kind === 'text' && i.text.trim())
    if (t && t.kind === 'text') return t.text.trim()
    const im = doc.items.find((i) => i.kind === 'image')
    if (im) return stemOf(sourceOf(im)?.name ?? '') || strings.editor.kinds.image
    return strings.editor.kinds.shape
  }

  async function rememberCurrent() {
    const used = new Set(doc.items.flatMap((i) => (i.kind === 'image' ? [i.imageId] : [])))
    const images = sources
      .filter((s) => used.has(s.id))
      .map((s) => ({ id: s.id, name: s.name, width: s.width, height: s.height, png: dataUrlToBytes(s.previewUrl) }))
    const name = designName()
    const saved = await saveRecent({
      id: `doc:${name}`,
      name,
      params: { version: 2, items: doc.items, hoopId: doc.hoopId },
      images,
    })
    if (saved) await refreshRecent()
  }

  /** 把存下來的設計放回畫面：圖片重新解碼、物件指到新的圖片編號。成功回傳 true。 */
  async function applySaved(
    p: Record<string, unknown>,
    savedImages: SavedImageWithId[],
  ): Promise<boolean> {
    imageProblem = null
    const loaded: Source[] = []
    for (const im of savedImages) {
      const r = await loadImage(fileFromBytes(im.png, im.name))
      if ('code' in r) {
        imageProblem = r
        return false
      }
      loaded.push(toSource(r))
    }
    sources = [...sources, ...loaded]
    let items: EditorItem[] = []
    let hoopId = typeof p.hoopId === 'string' ? p.hoopId : doc.hoopId
    if (p.version === 2 && Array.isArray(p.items)) {
      const map = new Map(loaded.map((s, i) => [savedImages[i]?.id, s.id]))
      // 舊的存檔沒有翻轉、組、弧形這些欄位：缺的補預設值
      const defaults: Record<string, unknown> = { flipX: false, flipY: false, groupId: null, locked: false }
      items = (p.items as EditorItem[]).map((it) => {
        const filled = { ...defaults, ...(it.kind === 'text' ? { curve: 0 } : {}), ...it } as EditorItem
        return filled.kind === 'image' ? { ...filled, imageId: map.get(filled.imageId) ?? filled.imageId } : filled
      })
    } else {
      const one = fromLegacy(p, loaded[0]?.id)
      items = one ? [one] : []
    }
    if (!hoops.some((h) => h.id === hoopId)) hoopId = doc.hoopId
    commit({ items, hoopId, selectedId: items.length === 1 ? items[0].id : null })
    return true
  }

  async function restore(d: SavedDesign) {
    resumed = false
    await applySaved(d.params as Record<string, unknown>, d.images ?? (d.image ? [{ id: 'legacy', ...d.image }] : []))
  }

  // ── 自動儲存：做到一半的設計，下次打開接著做 ──
  let resumed = $state(false)
  let autosaveReady = false

  function currentDraft() {
    const used = new Set(doc.items.flatMap((i) => (i.kind === 'image' ? [i.imageId] : [])))
    return {
      params: { version: 2, items: doc.items, hoopId: doc.hoopId },
      images: sources
        .filter((s) => used.has(s.id))
        .map((s) => ({ id: s.id, name: s.name, width: s.width, height: s.height, png: dataUrlToBytes(s.previewUrl) })),
    }
  }

  $effect(() => {
    void loadCurrent().then(async (draft) => {
      if (draft && doc.items.length === 0) {
        const items = (draft.params as { items?: unknown[] }).items
        if (Array.isArray(items) && items.length > 0 && (await applySaved(draft.params, draft.images))) resumed = true
      }
      autosaveReady = true
    })
  })

  $effect(() => {
    const items = doc.items
    void doc.hoopId
    void sources
    if (!autosaveReady) return
    const timer = setTimeout(() => {
      if (items.length === 0) void clearCurrent()
      else void saveCurrent(currentDraft())
    }, 600)
    return () => clearTimeout(timer)
  })

  function startOver() {
    resumed = false
    commit({ items: [], hoopId: doc.hoopId, selectedId: null })
  }

  async function forget(d: SavedDesign) {
    await removeRecent(d.id)
    await refreshRecent()
  }

  // ── 存檔 ──
  let saving = $state(false)
  let failure = $state<UserMessage | null>(null)
  let exported = $state<{ name: string; files: EncodedFile[]; firstTime: boolean } | null>(null)

  async function save(mode: 'default' | 'all') {
    if (!plan || !canSave || saving) return
    saving = true
    failure = null
    try {
      const firstTime = prefs.isFirstTime || mode === 'all'
      const name = designName()
      const out = await client.encodeAll(plan, hoop, formatsToExport(prefs.defaultFormats, mode), name, nextSequence())
      if (out.error || !out.files) {
        failure = out.error
        return
      }
      await downloadFiles(out.files)
      void rememberCurrent()
      exported = { name, files: out.files, firstTime: firstTime && prefs.isFirstTime }
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

  // ── 繡框的新增、修改、刪除 ──
  let editing = $state<{ hoop?: Hoop } | null>(null)

  function refreshHoops(selectId?: string) {
    hoops = store.list()
    const wanted = selectId ?? doc.hoopId
    const id = hoops.some((h) => h.id === wanted) ? wanted : (hoops.find((h) => h.id === 'b70-medium') ?? hoops[0]).id
    if (id !== doc.hoopId) commit({ ...doc, hoopId: id })
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

  // 手機上畫布固定在最上面，捲動到下面的工具時不要被它蓋住：告訴瀏覽器要多留這麼多位置
  let stageHeight = $state(0)
  $effect(() => {
    document.documentElement.style.setProperty('--stage-h', `${stageHeight + 24}px`)
  })

  // ── 畫畫 ──
  let drawing = $state(false)
  let brush = $state<Brush>({ widthMm: 2.5, colorIdx: 0, filled: false })
  const brushCss = $derived(`rgb(${THREADS[brush.colorIdx].r},${THREADS[brush.colorIdx].g},${THREADS[brush.colorIdx].b})`)

  function startDrawing() {
    if (selected?.kind === 'draw') {
      brush = { widthMm: selected.widthMm, colorIdx: selected.colorIdx, filled: selected.filled }
    } else {
      doc = selectItem(doc, null)
    }
    drawing = true
    shapeMenu = false
  }
  function onStroke(points: { x: number; y: number }[]) {
    if (points.length === 0) return
    const target = selected?.kind === 'draw' ? selected : null
    commit(target ? appendStroke(doc, target.id, points, fit) : startDraw(doc, points, brush, fit))
  }
  function setBrush(p: Partial<Brush>, key?: string) {
    brush = { ...brush, ...p }
    if (selected?.kind === 'draw') patch(selected.id, p as Partial<EditorItem>, key)
  }
  function undoStroke() {
    if (selected?.kind === 'draw') commit(removeLastStroke(doc, selected.id))
  }

  // ── 做很多個、對稱 ──
  let manyCount = $state(5)
  let manyGapCm = $state(1)
  let manyRadiusCm = $state(3)
  const itemSize = (id: string) => {
    const i = infos.find((x) => x.id === id)
    return i && i.width > 0 ? { width: i.width, height: i.height } : { width: 300, height: 300 }
  }
  const rowOf = (id: string, axis: 'x' | 'y') =>
    commit(repeatRow(doc, id, { count: manyCount, gap: manyGapCm * 100, axis, size: itemSize(id) }))

  // 鍵盤快捷鍵（電腦）：Ctrl／⌘ + Z 上一步、+ Shift + Z 或 Y 下一步、+ D 複製；Delete 刪掉選到的。
  // 在輸入框裡打字時不攔截（輸入框自己有復原）。
  function onShortcut(e: KeyboardEvent) {
    const el = e.target as HTMLElement | null
    if (e.defaultPrevented || el?.closest('input, textarea, select, [contenteditable]')) return
    const mod = e.ctrlKey || e.metaKey
    const key = e.key.toLowerCase()
    if (mod && key === 'z') {
      e.preventDefault()
      return e.shiftKey ? redo() : undo()
    }
    if (mod && key === 'y') {
      e.preventDefault()
      return redo()
    }
    if (mod && key === 'd' && selected) {
      e.preventDefault()
      return commit(duplicateItem(doc, selected.id))
    }
    if ((e.key === 'Delete' || e.key === 'Backspace') && selected && !drawing) {
      e.preventDefault()
      remove(selected.id)
    }
  }

  const shapeKinds: ShapeKind[] = ['circle', 'rect', 'heart', 'star', 'line']
  let shapeMenu = $state(false)
</script>

{#snippet addBar()}
      <div class="card add-bar" class:bar={wide}>
        {#if !wide}<h2>{strings.editor.addTitle}</h2>{/if}
        <div class="adds">
          <button onclick={() => commit(addText(doc))} data-testid="add-text">{strings.editor.addText}</button>
          <button onclick={() => pickImage()} data-testid="add-image">{strings.editor.addImage}</button>
          <button onclick={() => cameraInput?.click()}>{strings.editor.addPhoto}</button>
          <button onclick={() => (shapeMenu = !shapeMenu)} aria-expanded={shapeMenu} data-testid="add-shape">{strings.editor.addShape}</button>
          <button onclick={startDrawing} data-testid="add-draw">{strings.editor.addDraw}</button>
        </div>
        {#if shapeMenu}
          <div class="menu" data-testid="shape-menu">
            <div class="chips shapes" role="group" aria-label={strings.editor.addShape}>
              {#each shapeKinds as k (k)}
                <button
                  onclick={() => {
                    addShapeOf(k)
                    shapeMenu = false
                  }}
                  data-testid="shape-{k}">{strings.editor.shapes[k]}</button>
              {/each}
              <button onclick={() => { commit(addFrame(doc, 'rect', hoop)); shapeMenu = false }} data-testid="frame-rect">{strings.editor.frameRect}</button>
              <button onclick={() => { commit(addFrame(doc, 'circle', hoop)); shapeMenu = false }} data-testid="frame-circle">{strings.editor.frameCircle}</button>
            </div>
          </div>
        {/if}
        {#if loadingImage}<p class="muted" role="status">{strings.image.loading}</p>{/if}
        {#if imageProblem}<Notice message={imageProblem} tone="warn" />{/if}
      </div>

{/snippet}

{#snippet saveBar()}
  <div class="save" class:bar={wide}>
    <button class="primary" onclick={() => save('default')} disabled={!canSave || saving || computing} data-testid="save">
      {saving ? strings.actions.saving : strings.actions.saveToUsb}
    </button>
    <p class="default" data-testid="default-note">{defaultLabel}</p>
  </div>
{/snippet}

<svelte:window bind:innerWidth={winW} onkeydown={onShortcut} />

<main>
  <header>
    <h1>{strings.appName}</h1>
    {#if wide}
      {@render addBar()}
    {/if}
    <div class="history">
      <button onclick={undo} disabled={!canUndo} aria-label={strings.actions.undo}>↶ {strings.actions.undo}</button>
      <button onclick={redo} disabled={!canRedo} aria-label={strings.actions.redo}>{strings.actions.redo} ↷</button>
    </div>
    {#if wide}
      {@render saveBar()}
    {/if}
  </header>

  <input type="file" accept="image/*" hidden bind:this={fileInput} onchange={onFile} data-testid="file-input" />
  <input type="file" accept="image/*" capture="environment" hidden bind:this={cameraInput} onchange={onFile} />

  <div class="work">
    <section class="stage-col">
      <div class="card canvas-card" bind:clientHeight={stageHeight}>
        <EditorCanvas
          {plan}
          {hoop}
          items={doc.items}
          {infos}
          {fit}
          selectedId={doc.selectedId}
          label={strings.text.preview}
          onSelect={select}
          onChange={(id, p, key) => patch(id, p, key.split(':')[0])}
          onRemove={remove}
          fill={wide}
          {drawing}
          brush={{ css: brushCss, units: brush.widthMm * 10 }}
          {onStroke}
        />
        <div class="status">
          {#if doc.items.length === 0}
            <p class="muted">{strings.editor.startHint}</p>
          {:else if computing && !result}
            <p class="muted" role="status">{strings.loading}</p>
          {:else if sizeText}
            <p class="muted" data-testid="size">{sizeText}</p>
          {:else if isEmptyHint}
            <p class="muted">{strings.text.empty}</p>
          {/if}
          {#if computing && result}
            <p class="muted" role="status">{strings.updating}</p>
          {/if}
        </div>
        {#if resumed && doc.items.length > 0}
          <p class="muted" role="status" data-testid="resumed-note">{strings.editor.resumed}</p>
        {/if}
        {#if preview?.blocked && !isEmptyHint}
          <Notice message={preview.blocked} tone="warn" />
        {/if}
      </div>
    </section>

    <section class="panel-col">
      {#each preview?.adjustments ?? [] as m (m.code + m.text)}
        <Notice message={m} />
      {/each}
      {#if failure}
        <Notice message={failure} tone="warn" />
      {/if}

      {#if !wide}
        {@render addBar()}
      {/if}

      {#if drawing}
        <div class="card item-panel" data-testid="draw-panel">
          <h2>{strings.editor.drawTitle}</h2>
          <p class="muted">{strings.editor.drawHint}</p>
          <h3>{strings.text.color}</h3>
          <div class="swatches" role="radiogroup" aria-label={strings.text.color}>
            {#each THREADS as t, i (t.name)}
              <button
                role="radio"
                aria-checked={brush.colorIdx === i}
                aria-label={t.name}
                class:on={brush.colorIdx === i}
                style="--c: rgb({t.r},{t.g},{t.b})"
                onclick={() => setBrush({ colorIdx: i })}
              ><span class="dot"></span><span class="name">{t.name}</span></button>
            {/each}
          </div>
          <h3>{strings.editor.drawWidth}</h3>
          <div class="size">
            <input
              type="range"
              min="1"
              max="8"
              step="0.5"
              value={brush.widthMm}
              aria-label={strings.editor.drawWidth}
              oninput={(e) => setBrush({ widthMm: Number(e.currentTarget.value) }, 'line')}
            />
            <output>{brush.widthMm.toFixed(1)} {strings.editor.lineWidthUnit}</output>
          </div>
          <h3>{strings.editor.drawFill}</h3>
          <div class="chips" role="radiogroup" aria-label={strings.editor.drawFill}>
            <button role="radio" aria-checked={!brush.filled} class:on={!brush.filled} onclick={() => setBrush({ filled: false })}>{strings.editor.hollow}</button>
            <button role="radio" aria-checked={brush.filled} class:on={brush.filled} onclick={() => setBrush({ filled: true })}>{strings.editor.filled}</button>
          </div>
          <div class="tools">
            <button onclick={undoStroke} disabled={selected?.kind !== 'draw'} data-testid="draw-undo">{strings.editor.drawUndo}</button>
            <button class="primary-ish" onclick={() => (drawing = false)} data-testid="draw-done">{strings.editor.drawDone}</button>
          </div>
        </div>
      {:else if selected}
        {@const it = selected}
        <div class="card item-panel" data-testid="item-panel" data-kind={it.kind}>
          <h2>{strings.editor.kinds[it.kind]}</h2>

          {#if selectedInfo?.problem && !(it.kind === 'text' && it.text.trim() === '')}
            <Notice message={selectedInfo.problem} tone="warn" />
          {/if}

          {#if it.kind === 'text'}
            <label for="text"><h3>{strings.text.title}</h3></label>
            <textarea
              id="text"
              rows="2"
              placeholder={strings.text.placeholder}
              value={it.text}
              oninput={(e) => patch(it.id, { text: e.currentTarget.value }, 'text')}
            ></textarea>

            <h3>{strings.text.font}</h3>
            <div class="chips" role="radiogroup" aria-label={strings.text.font}>
              {#each FONT_FILES as f (f.id)}
                <button
                  role="radio"
                  aria-checked={it.fontId === f.id}
                  class:on={it.fontId === f.id}
                  onclick={() => patch(it.id, { fontId: f.id })}
                >{strings.fonts[f.id]}</button>
              {/each}
            </div>

            <h3>{strings.editor.curve}</h3>
            <div class="size">
              <span class="end">{strings.editor.curveDown}</span>
              <input
                type="range"
                min="-180"
                max="180"
                step="10"
                value={it.curve}
                aria-label={strings.editor.curve}
                oninput={(e) => patch(it.id, { curve: Number(e.currentTarget.value) }, 'curve')}
              />
              <span class="end">{strings.editor.curveUp}</span>
            </div>
            <p class="muted center">{it.curve === 0 ? strings.editor.curveFlat : `${Math.abs(it.curve)}°`}</p>
          {/if}

          {#if it.kind === 'image'}
            {@const src = sourceOf(it)}
            {#if src}
              <ImageCrop
                src={src.previewUrl}
                alt={src.name}
                rect={it.crop}
                onchange={(r: CropRect | null) => {
                  const c = cropRgba(src.rgba, src.width, src.height, r)
                  patch(it.id, { crop: r, sizeCm: fitImageSizeCm(hoop, c.width, c.height) })
                }}
              />
            {/if}
            <div class="pick">
              <button class="secondary" onclick={() => pickImage(it.id)} data-testid="choose-image">{strings.image.change}</button>
            </div>

            <h3>{strings.image.colors}</h3>
            <div class="size">
              <span class="end">{strings.image.colorsFew}</span>
              <input
                type="range"
                min="2"
                max="12"
                step="1"
                value={it.colors}
                aria-label={strings.image.colors}
                oninput={(e) => patch(it.id, { colors: Number(e.currentTarget.value) }, 'colors')}
              />
              <span class="end">{strings.image.colorsMany}</span>
            </div>
            <p class="muted center">{strings.image.colorsCount(it.colors)}</p>

            <h3>{strings.image.simplify}</h3>
            <div class="size">
              <span class="end">{strings.image.simplifyLow}</span>
              <input
                type="range"
                min="0"
                max="100"
                step="5"
                value={it.simplify}
                aria-label={strings.image.simplify}
                oninput={(e) => patch(it.id, { simplify: Number(e.currentTarget.value) }, 'simplify')}
              />
              <span class="end">{strings.image.simplifyHigh}</span>
            </div>
          {/if}

          {#if it.kind === 'shape'}
            <h3>{strings.editor.fillStyle}</h3>
            {#if it.shape !== 'line'}
              <div class="chips" role="radiogroup" aria-label={strings.editor.fillStyle}>
                <button role="radio" aria-checked={it.filled} class:on={it.filled} onclick={() => patch(it.id, { filled: true })}>{strings.editor.filled}</button>
                <button role="radio" aria-checked={!it.filled} class:on={!it.filled} onclick={() => patch(it.id, { filled: false })}>{strings.editor.hollow}</button>
              </div>
            {/if}
            {#if it.shape === 'line' || !it.filled}
              <h3>{strings.editor.lineWidth}</h3>
              <div class="size">
                <input
                  type="range"
                  min="1"
                  max="8"
                  step="0.5"
                  value={it.lineWidthMm}
                  aria-label={strings.editor.lineWidth}
                  oninput={(e) => patch(it.id, { lineWidthMm: Number(e.currentTarget.value) }, 'line')}
                />
                <output>{it.lineWidthMm.toFixed(1)} {strings.editor.lineWidthUnit}</output>
              </div>
            {/if}
          {/if}

          {#if it.kind === 'draw'}
            <div class="pick">
              <button class="secondary" onclick={startDrawing} data-testid="draw-more">{strings.editor.drawMore}</button>
            </div>
            <h3>{strings.editor.drawWidth}</h3>
            <div class="size">
              <input
                type="range"
                min="1"
                max="8"
                step="0.5"
                value={it.widthMm}
                aria-label={strings.editor.drawWidth}
                oninput={(e) => patch(it.id, { widthMm: Number(e.currentTarget.value) }, 'line')}
              />
              <output>{it.widthMm.toFixed(1)} {strings.editor.lineWidthUnit}</output>
            </div>
            <h3>{strings.editor.drawFill}</h3>
            <div class="chips" role="radiogroup" aria-label={strings.editor.drawFill}>
              <button role="radio" aria-checked={!it.filled} class:on={!it.filled} onclick={() => patch(it.id, { filled: false })}>{strings.editor.hollow}</button>
              <button role="radio" aria-checked={it.filled} class:on={it.filled} onclick={() => patch(it.id, { filled: true })}>{strings.editor.filled}</button>
            </div>
          {/if}

          {#if it.kind !== 'image'}
            <h3>{strings.text.color}</h3>
            <div class="swatches" role="radiogroup" aria-label={strings.text.color}>
              {#each THREADS as t, i (t.name)}
                <button
                  role="radio"
                  aria-checked={it.colorIdx === i}
                  aria-label={t.name}
                  class:on={it.colorIdx === i}
                  style="--c: rgb({t.r},{t.g},{t.b})"
                  onclick={() => patch(it.id, { colorIdx: i })}
                ><span class="dot"></span><span class="name">{t.name}</span></button>
              {/each}
            </div>
          {/if}

          {#if it.kind !== 'draw'}
            {@const range = sizeRange(it)}
            {@const base = baseCm(it)}
            <h3>{it.kind === 'text' ? strings.text.size : it.kind === 'image' ? strings.image.size : strings.editor.size}</h3>
            <div class="size">
              <input
                type="range"
                min={range.min}
                max={range.max}
                step={range.step}
                value={Math.min(range.max, Math.max(range.min, base * it.scale))}
                aria-label={it.kind === 'text' ? strings.text.size : it.kind === 'image' ? strings.image.size : strings.editor.size}
                oninput={(e) => patch(it.id, { scale: Number(e.currentTarget.value) / base }, 'size')}
              />
              <output>{(base * it.scale).toFixed(1)} {strings.editor.sizeUnit}</output>
            </div>
          {/if}

          {#if groupSize(doc, it.id) > 1}
            <div class="group" data-testid="group-note">
              <p>{strings.editor.groupNote(groupSize(doc, it.id))}</p>
              <div class="tools">
                <button onclick={() => (solo = !solo)} aria-pressed={solo} class:on={solo} data-testid="group-solo">
                  {solo ? strings.editor.groupTogether : strings.editor.groupSolo}
                </button>
                <button onclick={() => commit(ungroup(doc, it.id))} data-testid="group-break">{strings.editor.groupBreak}</button>
              </div>
            </div>
          {/if}

          <h3>{strings.editor.manyTitle}</h3>
          <div class="size">
            <span class="end">{strings.editor.manyCount}</span>
            <input type="range" min="2" max="24" step="1" bind:value={manyCount} aria-label={strings.editor.manyCount} />
            <output>{manyCount} {strings.editor.manyUnit}</output>
          </div>
          <div class="size">
            <span class="end">{strings.editor.manyGap}</span>
            <input type="range" min="0" max="5" step="0.5" bind:value={manyGapCm} aria-label={strings.editor.manyGap} />
            <output>{manyGapCm.toFixed(1)} {strings.editor.sizeUnit}</output>
          </div>
          <div class="size">
            <span class="end">{strings.editor.manyRadius}</span>
            <input type="range" min="1" max="12" step="0.5" bind:value={manyRadiusCm} aria-label={strings.editor.manyRadius} />
            <output>{manyRadiusCm.toFixed(1)} {strings.editor.sizeUnit}</output>
          </div>
          <div class="tools">
            <button onclick={() => rowOf(it.id, 'x')} data-testid="many-row">{strings.editor.manyRow}</button>
            <button onclick={() => rowOf(it.id, 'y')} data-testid="many-column">{strings.editor.manyColumn}</button>
            <button onclick={() => commit(repeatCircle(doc, it.id, { count: manyCount, radius: manyRadiusCm * 100 }))} data-testid="many-circle">{strings.editor.manyCircle}</button>
          </div>
          <p class="muted">{strings.editor.manyHint}</p>

          <h3>{strings.editor.mirrorTitle}</h3>
          <div class="tools">
            <button onclick={() => commit(mirrorCopy(doc, it.id, 'x'))} data-testid="mirror-x">{strings.editor.mirrorX}</button>
            <button onclick={() => commit(mirrorCopy(doc, it.id, 'y'))} data-testid="mirror-y">{strings.editor.mirrorY}</button>
            <button onclick={() => commit(flipItem(doc, it.id, 'x'))} data-testid="flip-x">{strings.editor.flipX}</button>
            <button onclick={() => commit(flipItem(doc, it.id, 'y'))} data-testid="flip-y">{strings.editor.flipY}</button>
          </div>

          <h3>{strings.editor.layerTitle}</h3>
          {#if it.locked}<p class="muted" data-testid="lock-note">{strings.editor.lockNote}</p>{/if}
          <div class="tools">
            <button onclick={() => commit(toggleLock(doc, it.id))} aria-pressed={it.locked} class:on={it.locked} data-testid="lock">
              {it.locked ? strings.editor.unlock : strings.editor.lock}
            </button>
            <button onclick={() => patch(it.id, { rotation: it.rotation - 15 })}>↺ {strings.editor.turnLeft}</button>
            <button onclick={() => patch(it.id, { rotation: it.rotation + 15 })}>↻ {strings.editor.turnRight}</button>
            <button onclick={() => patch(it.id, { rotation: 0 })} disabled={it.rotation === 0}>{strings.editor.straighten}</button>
            <button onclick={() => commit(centerItem(doc, it.id, 'x'))}>{strings.editor.centerX}</button>
            <button onclick={() => commit(centerItem(doc, it.id, 'y'))}>{strings.editor.centerY}</button>
            <button onclick={() => commit(moveLayer(doc, it.id, 'forward'))}>{strings.editor.forward}</button>
            <button onclick={() => commit(moveLayer(doc, it.id, 'backward'))}>{strings.editor.backward}</button>
            <button onclick={() => commit(duplicateItem(doc, it.id))} data-testid="duplicate">{strings.editor.duplicate}</button>
            <button class="danger" onclick={() => remove(it.id)} data-testid="remove">{strings.editor.remove}</button>
          </div>
        </div>
      {:else if doc.items.length > 0}
        <p class="muted hint">{strings.editor.selectHint}</p>
      {/if}

      <div class="card">
        <h2>{strings.text.hoop}</h2>
        <HoopPicker
          {hoops}
          selectedId={doc.hoopId}
          onSelect={(id) => commit({ ...doc, hoopId: id })}
          onAdd={() => (editing = {})}
          onEdit={(h) => (editing = { hoop: h })}
          onRemove={removeHoop}
        />
      </div>

      {#if recent.length > 0}
        <section class="card" data-testid="recent">
          <h2>{strings.recent.title}</h2>
          <ul class="recent">
            {#each recent as d (d.id)}
              <li>
                <span class="rname">{d.name}</span>
                <button onclick={() => restore(d)} data-testid="restore">{strings.recent.restore}</button>
                <button class="danger" onclick={() => forget(d)} aria-label="{strings.recent.remove} {d.name}">{strings.recent.remove}</button>
              </li>
            {/each}
          </ul>
        </section>
      {/if}

      <section class="card extra">
        <button class="secondary" onclick={() => save('all')} disabled={!canSave || saving || computing} data-testid="save-all">
          {strings.actions.saveAll}
        </button>
        <button class="secondary" onclick={startOver} disabled={doc.items.length === 0} data-testid="start-over">{strings.editor.startOver}</button>
        <button class="secondary" onclick={onTry} data-testid="try-entry">{strings.actions.tryEntry}</button>
      </section>
    </section>
  </div>

    {#if removedNote}
      <div class="toast" role="status" data-testid="removed-note">
        <span>{strings.editor.removed(removedNote.name)}</span>
        <button onclick={takeBack} data-testid="take-back">{strings.editor.undoRemove}</button>
      </div>
    {/if}

  {#if !wide}
    {@render saveBar()}
  {/if}
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
  :global(html) { scroll-padding-top: var(--stage-h, 0px); scroll-padding-bottom: 130px; }
  @media (min-width: 900px) { :global(html) { scroll-padding-top: 0; scroll-padding-bottom: 0; } }
  main { max-width: 1180px; margin: 0 auto; padding: 12px 16px 150px; }
  header { display: flex; align-items: center; justify-content: space-between; gap: 8px; flex-wrap: wrap; }
  h1 { font-size: 1.4em; margin: 6px 0; }
  h2 { font-size: 1.05em; margin: 14px 0 8px; }
  h3 { font-size: 0.98em; margin: 14px 0 6px; }
  .history { display: flex; gap: 8px; }
  .history button { min-height: 48px; padding: 0 14px; border: 2px solid var(--line); border-radius: 12px; background: var(--card); }
  .history button:disabled { opacity: 0.4; }

  /* 手機與直放的平板：一欄，畫布固定在上面，工具在下面捲動 */
  .work { display: block; }
  .stage-col { position: sticky; top: 0; z-index: 5; background: var(--bg); padding-top: 1px; }
  .stage-col .canvas-card { margin-top: 8px; }
  /* 平板橫放與電腦：整個畫面不捲動。上面一條工具列（加東西、上一步、存檔），
     下面左邊畫布、右邊工具區，兩邊各自捲動，畫布永遠看得到。 */
  @media (min-width: 900px) {
    main { max-width: none; height: 100vh; padding: 0; display: flex; flex-direction: column; overflow: hidden; }
    header { flex: none; flex-wrap: nowrap; gap: 14px; padding: 8px 16px; border-bottom: 1px solid var(--line); background: var(--card); }
    h1 { font-size: 1.15em; margin: 0; white-space: nowrap; }
    .work { flex: 1; min-height: 0; display: grid; grid-template-columns: minmax(0, 1fr) 420px; gap: 0; align-items: stretch; }
    .stage-col { position: static; overflow-y: auto; padding: 10px 16px; background: transparent; display: flex; flex-direction: column; justify-content: center; }
    .stage-col .canvas-card { margin: 0; }
    .panel-col { overflow-y: auto; padding: 4px 16px 24px; border-left: 1px solid var(--line); background: #fdfaf4; }
    .panel-col .card:first-child { margin-top: 12px; }
    :global(html), :global(body) { overflow: hidden; }
  }

  .add-bar.bar { display: block; position: relative; flex: 1; margin: 0; padding: 0; border: 0; background: transparent; }
  .add-bar.bar :first-child { margin-top: 0; }
  .add-bar.bar .adds { display: flex; flex-wrap: nowrap; gap: 8px; }
  .add-bar.bar .adds button { min-height: 48px; padding: 0 16px; white-space: nowrap; }
  .add-bar.bar .adds button:last-child:nth-child(odd) { grid-column: auto; }
  /* 圖案選單：從工具列往下展開 */
  .add-bar.bar > .menu { position: absolute; z-index: 20; top: 54px; left: 0; padding: 10px; border-radius: 12px; background: var(--card); border: 1px solid var(--line); box-shadow: 0 6px 18px rgba(0, 0, 0, 0.16); }
  .add-bar.bar > .muted, .add-bar.bar > :global([role=alert]) { position: absolute; top: 54px; left: 0; z-index: 20; }
  .save.bar { position: static; padding: 0; background: none; z-index: auto; flex: none; }
  .save.bar .primary { width: auto; min-width: 150px; min-height: 48px; padding: 0 22px; font-size: 1.05em; }
  .save.bar .default { display: none; }
  .adds { display: grid; grid-template-columns: repeat(2, 1fr); gap: 10px; }
  .adds button, .tools button, .shapes button { min-height: 52px; padding: 6px 12px; border: 2px solid var(--line); border-radius: 12px; background: #fff; font-weight: 700; }
  .adds button:first-child { border-color: var(--accent); }
  .adds button:last-child:nth-child(odd) { grid-column: 1 / -1; }
  .shapes { margin-top: 10px; }
  .tools { display: grid; grid-template-columns: repeat(2, 1fr); gap: 8px; }
  .tools button:disabled { opacity: 0.4; }
  .tools .danger { border-color: #d89a9a; color: #9c1c1c; }
  .toast { position: fixed; left: 16px; right: 16px; bottom: calc(110px + env(safe-area-inset-bottom)); z-index: 7; max-width: 560px; margin: 0 auto; box-shadow: 0 4px 14px rgba(0,0,0,0.18); display: flex; align-items: center; justify-content: space-between; gap: 10px; background: #fff7e0; border: 2px solid #e8c25a; border-radius: 12px; padding: 8px 12px; margin-top: 10px; }
  .tools .primary-ish { border-color: var(--accent); background: #fdecec; }
  .group { background: #eef5ff; border: 2px solid #9db8e8; border-radius: 12px; padding: 4px 12px 12px; margin: 12px 0 0; }
  .group p { margin: 8px 0; }
  .tools button.on { border-color: var(--accent); background: #fdecec; }
  .toast button { min-height: 48px; padding: 0 16px; border: 2px solid var(--accent); border-radius: 10px; background: #fff; font-weight: 700; }
  .status { min-height: 1.5em; }
  .hint { text-align: center; }
  .pick { display: flex; gap: 10px; margin-top: 8px; }
  .pick .secondary { flex: 1; }
  .end { font-size: 0.8em; color: var(--muted); min-width: 4.2em; text-align: center; }
  .center { text-align: center; }
  .recent { list-style: none; margin: 0; padding: 0; }
  .recent li { display: flex; align-items: center; gap: 8px; padding: 8px 0; border-bottom: 1px solid var(--line); }
  .recent .rname { flex: 1; font-weight: 700; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .recent button { min-height: 48px; padding: 0 12px; border: 2px solid var(--line); border-radius: 10px; background: #fff; }
  .recent .danger { border-color: #d89a9a; color: #9c1c1c; }
  .card { background: var(--card); border: 1px solid var(--line); border-radius: var(--radius); padding: 4px 16px 16px; margin: 12px 0; }
  .card :first-child { margin-top: 12px; }
  textarea { width: 100%; font-size: 1.3em; padding: 12px; border: 2px solid var(--line); border-radius: 12px; resize: vertical; background: #fff; }
  .muted { color: var(--muted); margin: 8px 0 0; font-size: 0.92em; }
  .chips { display: flex; gap: 10px; flex-wrap: wrap; }
  .chips button { min-height: 56px; min-width: 80px; padding: 6px 16px; border: 2px solid var(--line); border-radius: 12px; background: #fff; display: flex; flex-direction: column; align-items: center; justify-content: center; }
  .chips button.on { border-color: var(--accent); background: #fdecec; font-weight: 700; }
  .size { display: flex; align-items: center; gap: 12px; }
  .size input { flex: 1; height: 48px; accent-color: var(--accent); }
  .size output { min-width: 5.5em; text-align: right; font-weight: 700; }
  .swatches { display: grid; grid-template-columns: repeat(4, 1fr); gap: 8px; }
  .swatches button { min-height: 64px; border: 2px solid var(--line); border-radius: 12px; background: #fff; display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 2px; padding: 4px; }
  @media (min-width: 900px) {
    .swatches { grid-template-columns: repeat(5, 1fr); gap: 6px; }
    .swatches button { min-height: 56px; }
    .swatches .dot { width: 24px; height: 24px; }
  }
  .swatches .dot { width: 28px; height: 28px; border-radius: 50%; background: var(--c); border: 1px solid rgba(0, 0, 0, 0.25); }
  .swatches .name { font-size: 0.72em; }
  .swatches button.on { border-color: var(--accent); background: #fdecec; font-weight: 700; }
  .save { position: fixed; left: 0; right: 0; bottom: 0; padding: 12px 16px calc(8px + env(safe-area-inset-bottom)); background: linear-gradient(transparent, var(--bg) 30%); z-index: 6; }
  .extra { display: flex; flex-direction: column; gap: 10px; padding-top: 16px; }
  .secondary { min-height: 56px; border: 2px solid var(--line); border-radius: var(--radius); background: #fff; font-weight: 700; }
  .secondary:disabled { opacity: 0.5; }
  .default { text-align: center; margin: 6px 0 0; font-size: 0.85em; color: var(--muted); }
  .primary { display: block; width: 100%; max-width: 560px; margin: 0 auto; min-height: 60px; border: 0; border-radius: var(--radius); background: var(--accent); color: var(--accent-ink); font-size: 1.2em; font-weight: 700; }
  .primary:disabled { background: #c9b8b0; }
</style>
