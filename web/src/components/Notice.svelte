<script lang="ts">
  import type { UserMessage } from '../lib/types'

  let { message, tone = 'info' }: { message: UserMessage; tone?: 'info' | 'warn' } = $props()
</script>

<div class="notice {tone}" role={tone === 'warn' ? 'alert' : 'status'} data-code={message.code}>
  <p class="text">{message.text}</p>
  {#if message.hint}<p class="hint">{message.hint}</p>{/if}
</div>

<style>
  .notice { border: 2px solid; border-radius: 12px; padding: 12px 14px; margin: 10px 0; }
  .info { background: var(--info-bg); border-color: var(--info-line); }
  .warn { background: var(--warn-bg); border-color: var(--warn-line); }
  p { margin: 0; }
  .hint { margin-top: 4px; color: var(--muted); font-size: 0.92em; }
</style>
