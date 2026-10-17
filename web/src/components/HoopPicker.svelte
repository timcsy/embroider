<script lang="ts">
  import { strings } from '../lib/strings.zh-TW'
  import type { Hoop } from '../lib/types'

  let {
    hoops,
    selectedId,
    onSelect,
    onAdd,
    onEdit,
    onRemove,
  }: {
    hoops: Hoop[]
    selectedId: string
    onSelect: (id: string) => void
    onAdd: () => void
    onEdit: (hoop: Hoop) => void
    onRemove: (hoop: Hoop) => void
  } = $props()

  let confirming = $state<string | null>(null)

  const sizeText = (h: Hoop) =>
    strings.hoop.sizeText(
      String(Math.round(h.width / 10) / 10),
      String(Math.round(h.height / 10) / 10),
    )

  const selected = $derived(hoops.find((h) => h.id === selectedId))
</script>

<div class="chips" role="radiogroup" aria-label={strings.text.hoop}>
  {#each hoops as h (h.id)}
    <button
      role="radio"
      aria-checked={selectedId === h.id}
      class:on={selectedId === h.id}
      onclick={() => {
        confirming = null
        onSelect(h.id)
      }}
    >
      <span>{h.name}</span>
      <small>{sizeText(h)}</small>
    </button>
  {/each}
  <button class="add" onclick={onAdd} data-testid="hoop-add">＋ {strings.hoop.add}</button>
</div>

{#if selected && !selected.builtin}
  <div class="manage">
    <button onclick={() => onEdit(selected)}>{strings.hoop.edit}</button>
    {#if confirming === selected.id}
      <button class="danger" onclick={() => onRemove(selected)} data-testid="hoop-remove-confirm">
        {strings.hoop.confirmRemove}
      </button>
      <button onclick={() => (confirming = null)}>{strings.hoop.cancel}</button>
    {:else}
      <button class="danger" onclick={() => (confirming = selected.id)} data-testid="hoop-remove">
        {strings.hoop.remove}
      </button>
    {/if}
  </div>
{/if}

<style>
  .chips { display: flex; gap: 10px; flex-wrap: wrap; }
  .chips button { min-height: 56px; min-width: 80px; padding: 6px 16px; border: 2px solid var(--line); border-radius: 12px; background: #fff; display: flex; flex-direction: column; align-items: center; justify-content: center; }
  .chips small { color: var(--muted); font-size: 0.75em; }
  .chips button.on { border-color: var(--accent); background: #fdecec; font-weight: 700; }
  .chips .add { border-style: dashed; color: var(--accent); font-weight: 700; }
  .manage { display: flex; gap: 10px; flex-wrap: wrap; margin-top: 10px; }
  .manage button { min-height: 48px; padding: 0 16px; border: 2px solid var(--line); border-radius: 12px; background: #fff; }
  .manage .danger { border-color: #d89a9a; color: #9c1c1c; }
</style>
