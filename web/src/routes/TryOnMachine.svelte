<script lang="ts">
  import type { Prefs } from '../lib/prefs'
  import { strings } from '../lib/strings.zh-TW'
  import type { OutputFormatInfo } from '../lib/types'

  let { formats, prefs, onBack }: { formats: OutputFormatInfo[]; prefs: Prefs; onBack: () => void } = $props()

  // prefs 不是響應式的，訂閱它的變更通知來更新畫面
  let version = $state(0)
  $effect(() => prefs.onChange(() => version++))
  const defaults = $derived.by(() => {
    void version
    return [...prefs.defaultFormats]
  })

  const choose = (id: string) => prefs.setDefaultFormats([id])
  const reset = () => prefs.clearDefaultFormats()
</script>

<main>
  <header>
    <button class="back" onclick={onBack}>← {strings.try.back}</button>
  </header>
  <h1>{strings.try.title}</h1>
  <p>{strings.try.intro}</p>

  <ol class="steps">
    {#each strings.try.steps as step (step)}
      <li>{step}</li>
    {/each}
  </ol>

  <p class="safety">{strings.try.safety}</p>

  <h2>{strings.try.question}</h2>
  {#each formats as f (f.id)}
    <section class="card" data-testid="format-{f.id}">
      <p class="line">{strings.try.formatLine(f.label, f.description)}</p>
      {#if defaults.includes(f.id)}
        <button class="chosen" disabled>✓ {strings.try.chosen}</button>
      {:else}
        <button class="primary" onclick={() => choose(f.id)} data-testid="choose-{f.id}">{strings.try.choose}</button>
      {/if}
    </section>
  {/each}

  <p class="current" data-testid="current-default">
    {#if defaults.length === 0}
      {strings.export.defaultAll}
    {:else}
      {strings.export.currentDefault(formats.filter((f) => defaults.includes(f.id)).map((f) => f.label).join('、'))}
    {/if}
  </p>
  <button class="secondary" onclick={reset} data-testid="reset-default">{strings.try.reset}</button>
</main>

<style>
  main { max-width: 560px; margin: 0 auto; padding: 12px 16px 48px; }
  h1 { font-size: 1.4em; margin: 10px 0; }
  h2 { font-size: 1.1em; margin: 20px 0 8px; }
  .back { min-height: 48px; padding: 0 14px; border: 2px solid var(--line); border-radius: 12px; background: var(--card); }
  .steps { padding-left: 1.4em; }
  .steps li { margin: 8px 0; }
  .safety { background: var(--warn-bg); border: 2px solid var(--warn-line); border-radius: 12px; padding: 10px 12px; }
  .card { background: var(--card); border: 1px solid var(--line); border-radius: var(--radius); padding: 12px 16px 16px; margin: 12px 0; }
  .line { margin: 0 0 10px; }
  button { display: block; width: 100%; min-height: 56px; border-radius: var(--radius); font-size: 1.05em; font-weight: 700; }
  .primary { border: 0; background: var(--accent); color: var(--accent-ink); }
  .chosen { border: 2px solid var(--info-line); background: var(--info-bg); }
  .secondary { border: 2px solid var(--line); background: #fff; margin-top: 8px; }
  .current { margin: 16px 0 4px; font-weight: 700; }
</style>
