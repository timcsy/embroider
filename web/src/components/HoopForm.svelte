<script lang="ts">
  import { untrack } from 'svelte'
  import { parseHoopForm } from '../lib/hoopForm'
  import type { HoopInput } from '../lib/hoops'
  import { strings } from '../lib/strings.zh-TW'
  import type { Hoop, HoopShape, UserMessage } from '../lib/types'
  import Notice from './Notice.svelte'

  let {
    initial,
    onSubmit,
    onCancel,
  }: {
    initial?: Hoop
    /** 回傳訊息代表失敗（留在表單上顯示），回傳 null 代表成功 */
    onSubmit: (input: HoopInput) => UserMessage | null
    onCancel: () => void
  } = $props()

  const fromUnits = (v: number) => String(v / 10)

  // 表單只在開啟時讀一次初始值，之後以使用者填的為準
  const start = untrack(() => initial)
  let name = $state(start?.name ?? '')
  let widthMm = $state(start ? fromUnits(start.width) : '')
  let heightMm = $state(start ? fromUnits(start.height) : '')
  let shape = $state<HoopShape>(start?.shape ?? 'rect')
  let problem = $state<UserMessage | null>(null)

  function submit(e: Event) {
    e.preventDefault()
    const parsed = parseHoopForm({ name, widthMm, heightMm, shape })
    problem = parsed.ok ? onSubmit(parsed.input) : parsed.message
  }

  const base = import.meta.env.BASE_URL
</script>

<div class="backdrop" role="presentation">
  <form class="sheet" onsubmit={submit} aria-labelledby="hoop-form-title" data-testid="hoop-form">
    <h2 id="hoop-form-title">{initial ? strings.hoop.formTitleEdit : strings.hoop.formTitleNew}</h2>

    <section class="measure" aria-label={strings.hoop.measureTitle}>
      <img src="{base}hoop-measure.svg" alt="" />
      <p><strong>{strings.hoop.measureHint}</strong></p>
      <p class="small">{strings.hoop.measureMore}</p>
    </section>

    <label>
      {strings.hoop.name}
      <input type="text" bind:value={name} placeholder={strings.hoop.namePlaceholder} maxlength="20" />
    </label>

    <fieldset>
      <legend>{strings.hoop.shape}</legend>
      <div class="chips" role="radiogroup" aria-label={strings.hoop.shape}>
        {#each ['rect', 'circle', 'ellipse'] as const as s (s)}
          <button
            type="button"
            role="radio"
            aria-checked={shape === s}
            class:on={shape === s}
            onclick={() => (shape = s)}
          >{strings.hoop.shapes[s]}</button>
        {/each}
      </div>
    </fieldset>

    <label>
      {strings.hoop.width}
      <input type="text" inputmode="decimal" bind:value={widthMm} placeholder="100" />
    </label>
    {#if shape !== 'circle'}
      <label>
        {strings.hoop.height}
        <input type="text" inputmode="decimal" bind:value={heightMm} placeholder="100" />
      </label>
    {/if}

    {#if problem}<Notice message={problem} tone="warn" />{/if}

    <div class="buttons">
      <button type="button" class="secondary" onclick={onCancel}>{strings.hoop.cancel}</button>
      <button type="submit" class="primary" data-testid="hoop-save">{strings.hoop.save}</button>
    </div>
  </form>
</div>

<style>
  .backdrop { position: fixed; inset: 0; background: rgba(40, 30, 20, 0.45); display: flex; align-items: flex-end; z-index: 10; }
  .sheet { background: var(--card); width: 100%; max-width: 560px; margin: 0 auto; max-height: 94vh; overflow: auto; border-radius: 20px 20px 0 0; padding: 18px 18px 24px; }
  h2 { margin: 0 0 8px; font-size: 1.3em; }
  .measure { background: #faf6ee; border-radius: 12px; padding: 10px 12px; margin-bottom: 12px; }
  .measure img { display: block; width: 100%; max-width: 320px; margin: 0 auto 6px; }
  .measure p { margin: 4px 0; }
  .small { color: var(--muted); font-size: 0.88em; }
  label { display: block; margin: 12px 0; font-weight: 700; }
  input[type='text'] { display: block; width: 100%; margin-top: 6px; padding: 12px; font-size: 1.15em; border: 2px solid var(--line); border-radius: 12px; font-weight: 400; }
  fieldset { border: 0; padding: 0; margin: 12px 0; }
  legend { font-weight: 700; margin-bottom: 6px; }
  .chips { display: flex; gap: 10px; }
  .chips button { flex: 1; min-height: 52px; border: 2px solid var(--line); border-radius: 12px; background: #fff; }
  .chips button.on { border-color: var(--accent); background: #fdecec; font-weight: 700; }
  .buttons { display: flex; gap: 12px; margin-top: 16px; }
  .buttons button { flex: 1; min-height: 56px; border-radius: var(--radius); font-size: 1.1em; font-weight: 700; }
  .primary { border: 0; background: var(--accent); color: var(--accent-ink); }
  .secondary { border: 2px solid var(--line); background: #fff; }
</style>
