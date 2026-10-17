<script lang="ts">
  import { strings } from '../lib/strings.zh-TW'

  let {
    designName,
    fileNames,
    firstTime,
    canShare,
    onShare,
    onTry,
    onClose,
  }: {
    designName: string
    fileNames: string[]
    firstTime: boolean
    canShare: boolean
    onShare: () => void
    onTry: () => void
    onClose: () => void
  } = $props()
</script>

<div class="backdrop" role="presentation">
  <div class="sheet" role="dialog" aria-modal="true" aria-labelledby="export-title" data-testid="export-sheet">
    <h2 id="export-title">{strings.export.title}</h2>
    <p>{strings.export.intro}</p>
    {#if firstTime}
      <p class="first" data-testid="first-time">{strings.export.firstTime}</p>
    {/if}
    <p class="small">{strings.export.downloadHint}</p>
    <h3>{strings.export.nameMap}</h3>
    <ul>
      {#each fileNames as name (name)}
        <li><span class="design">{designName}</span> <span aria-hidden="true">→</span> <code>{name}</code></li>
      {/each}
    </ul>
    {#if canShare}
      <p class="small">{strings.export.shareNote}</p>
      <button class="secondary" onclick={onShare} data-testid="share">{strings.export.share}</button>
    {/if}
    {#if firstTime}
      <button class="secondary" onclick={onTry} data-testid="try-next">{strings.export.tryNext}</button>
    {/if}
    <button class="primary" onclick={onClose}>{strings.export.done}</button>
  </div>
</div>

<style>
  .backdrop { position: fixed; inset: 0; background: rgba(40, 30, 20, 0.45); display: flex; align-items: flex-end; z-index: 10; }
  .sheet { background: var(--card); width: 100%; max-width: 560px; margin: 0 auto; max-height: 92vh; overflow: auto; border-radius: 20px 20px 0 0; padding: 20px 18px 24px; }
  h2 { margin: 0 0 8px; font-size: 1.3em; }
  h3 { margin: 14px 0 6px; font-size: 1em; }
  p { margin: 6px 0; }
  .first { background: var(--info-bg); border-radius: 10px; padding: 10px 12px; }
  .small { color: var(--muted); font-size: 0.9em; }
  ul { list-style: none; padding: 0; margin: 0 0 16px; }
  li { padding: 6px 0; border-bottom: 1px solid var(--line); display: flex; gap: 8px; align-items: baseline; flex-wrap: wrap; }
  code { font-weight: 700; letter-spacing: 0.04em; }
  button { display: block; width: 100%; min-height: 56px; border-radius: var(--radius); font-size: 1.1em; font-weight: 700; margin-top: 10px; }
  .primary { border: 0; background: var(--accent); color: var(--accent-ink); }
  .secondary { border: 2px solid var(--line); background: #fff; }
</style>
