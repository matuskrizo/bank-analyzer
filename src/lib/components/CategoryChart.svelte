<script lang="ts">
  // Výdavky podľa kategórie – vodorovné stĺpce v čistom HTML/CSS.
  // Jedna séria = jedna farba; názov kategórie je priamo pri stĺpci.
  // Klik na stĺpec nastaví filter kategórie.
  import { app, UNCATEGORIZED } from "../state.svelte";
  import { money } from "../format";

  const MAX_ROWS = 8;

  // Viac ako 8 kategórií zlúčime do "Ostatné", aby graf ostal čitateľný.
  let rows = $derived.by(() => {
    const all = app.byCategory;
    if (all.length <= MAX_ROWS) return all;
    const rest = all.slice(MAX_ROWS - 1);
    return [
      ...all.slice(0, MAX_ROWS - 1),
      {
        name: `Ostatné (${rest.length})`,
        categoryId: null,
        cents: rest.reduce((s, r) => s + r.cents, 0),
        count: rest.reduce((s, r) => s + r.count, 0),
        isOther: true,
      },
    ];
  });

  let total = $derived(app.byCategory.reduce((sum, r) => sum + r.cents, 0));
  let max = $derived(Math.max(1, ...rows.map((r) => r.cents)));

  let hovered = $state<string | null>(null);

  function select(row: (typeof rows)[number]) {
    if ("isOther" in row) return;
    app.filters.category = row.name === UNCATEGORIZED ? "none" : (row.categoryId ?? "all");
  }

  const pct = (cents: number) => `${Math.round((cents / Math.max(total, 1)) * 100)} %`;
</script>

<section class="card">
  <h2>Výdavky podľa kategórie</h2>
  {#if rows.length === 0}
    <p class="muted empty">Žiadne výdavky vo vybranom období.</p>
  {:else}
    <ul class="bars">
      {#each rows as row (row.name)}
        <li>
          <button
            class="row"
            class:dim={hovered !== null && hovered !== row.name}
            onpointerenter={() => (hovered = row.name)}
            onpointerleave={() => (hovered = null)}
            onfocus={() => (hovered = row.name)}
            onblur={() => (hovered = null)}
            onclick={() => select(row)}
            aria-label="{row.name}: {money(row.cents)}, {pct(row.cents)} výdavkov, {row.count} transakcií"
          >
            <span class="name">{row.name}</span>
            <span class="track">
              <span class="bar" style:width="{(row.cents / max) * 100}%"></span>
            </span>
            <span class="num value">{money(row.cents)}</span>
          </button>
          {#if hovered === row.name}
            <div class="tip" role="tooltip">
              <strong>{pct(row.cents)}</strong> výdavkov · {row.count}
              {row.count === 1 ? "transakcia" : row.count < 5 ? "transakcie" : "transakcií"}
            </div>
          {/if}
        </li>
      {/each}
    </ul>
  {/if}
</section>

<style>
  h2 {
    font-size: 15px;
    margin-bottom: 12px;
  }

  .empty {
    margin: 24px 0;
  }

  .bars {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  li {
    position: relative;
  }

  .row {
    display: grid;
    grid-template-columns: 150px 1fr 96px;
    align-items: center;
    gap: 10px;
    width: 100%;
    border: none;
    background: none;
    padding: 5px 4px;
    border-radius: 6px;
    text-align: left;
    transition: opacity 0.12s;
  }

  .row:hover,
  .row:focus-visible {
    background: var(--hover);
  }

  .row.dim {
    opacity: 0.55;
  }

  .name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--text-secondary);
  }

  .track {
    height: 14px;
    border-left: 1px solid var(--axis);
  }

  .bar {
    display: block;
    height: 100%;
    min-width: 2px;
    background: var(--series-1);
    border-radius: 0 4px 4px 0; /* zaoblený len koniec s dátami */
  }

  .value {
    color: var(--text-primary);
  }

  .tip {
    position: absolute;
    right: 4px;
    top: -28px;
    z-index: 2;
    padding: 4px 9px;
    border-radius: 6px;
    background: var(--surface-raised);
    border: 1px solid var(--border);
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.12);
    font-size: 13px;
    white-space: nowrap;
    pointer-events: none;
  }
</style>
